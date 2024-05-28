use std::{collections::HashMap, net::TcpStream, thread::sleep, time::Duration};

use diff::Diff;
use game::{entities::{physics_square::PhysicsSquare, Entity}, game::{Drawable, HasOwner, HasRigidBody, Texture, TickContext, Tickable}, game_state::{GameState, GameStateDiff}, networking::{self, receive_headered}, proxies::macroquad::math::vec2::Vec2, time::Time, uuid};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use macroquad::{color::WHITE, input::{is_key_down, is_key_released, is_mouse_button_down, is_mouse_button_pressed, is_mouse_button_released}, shapes::DrawRectangleParams, texture::Texture2D};


pub struct Client {
    pub game_state: GameState,
    pub is_host: bool,
    pub last_tick_game_state: GameState,
    pub textures: HashMap<String, Texture2D>,
    pub sounds: HashMap<String, macroquad::audio::Sound>,
    pub last_tick: Time,
    pub uuid: String,
    pub server_receive: ewebsock::WsReceiver,
    pub server_send: ewebsock::WsSender,
    pub camera_offset: Vec2,
    pub update_count: i32,
    pub start_time: Time
}

impl Client {

    pub async fn run(&mut self) {

        loop {
        
            //macroquad::window::clear_background(macroquad::color::BLACK);
    
            self.tick();
            
            self.game_state.space.step(&self.uuid);
            
            self.draw().await;

            self.send_updates();
            
            self.receive_updates();

            // we dont want to track the changes that happen to the game state when we receive updates
            // so we set the checkpoint right after we receive the updates
            // this way it will only track what happened when we ticked the game state
            self.last_tick_game_state = self.game_state.clone();
    
            macroquad::window::next_frame().await;
    
            if macroquad::input::is_key_down(macroquad::input::KeyCode::J) {
                let state_string = serde_json::to_string_pretty(&self.game_state).unwrap();
    
                std::fs::write("state.json", state_string).expect("failed to write current state to state.json")
            }
    
            // cap framerate at 200fps (or 5 ms per frame)
            // TODO: this needs to take into account the time it took to draw the last frame
            std::thread::sleep(
                Duration::from_millis(5)
            );
    
            println!("{}",  macroquad::time::get_fps());
        }
    }

    // generate and send diff of game state from the last time we called this function (previous tick)
    pub fn send_updates(&mut self) {

        if self.last_tick_game_state == self.game_state {
            //println!("no changes in game state, not sending update");
            return;
        }
        let diff = self.last_tick_game_state.diff(&self.game_state);

        let diff_string = match serde_json::to_string(&diff) {
            Ok(diff_string) => diff_string,
            Err(error) => panic!("failed to serialize game state diff: {}", error),
        };

        let diff_string_bytes = diff_string.as_bytes();

        let compressed_diff_string_bytes = compress_prepend_size(&diff_string_bytes);

        self.server_send.send(ewebsock::WsMessage::Binary(compressed_diff_string_bytes.to_vec()));

    }

    pub fn receive_updates(&mut self) {
        let mut update_count = 0;
        
        // we loop until there are no new updates
        loop {

            let compressed_game_state_diff_string_bytes = match self.server_receive.try_recv() {
                Some(event) => {
                    match event {
                        ewebsock::WsEvent::Opened => todo!("unhandled 'Opened' event"),
                        ewebsock::WsEvent::Message(message) => {
                            match message {
                                ewebsock::WsMessage::Binary(bytes) => bytes,
                                _ => todo!("unhandled message type when trying to receive updates from server")
                            }
                        },
                        ewebsock::WsEvent::Error(error) => todo!("unhandled 'Error' event when trying to receive update from server: {}", error),
                        ewebsock::WsEvent::Closed => todo!("server closed"),
                    }
                },
                None => break, // this means there are no more updates
            };
            
            let game_state_diff_string_bytes = decompress_size_prepended(&compressed_game_state_diff_string_bytes).expect("Failed to decompress incoming update");

            let game_state_diff_string = match String::from_utf8(game_state_diff_string_bytes.clone()) {
                Ok(game_state_diff_string) => game_state_diff_string,
                Err(error) => {
                    panic!("failed to decode game state diff as string {}", error);
                },
            };

            let game_state_diff: GameStateDiff = match serde_json::from_str(&game_state_diff_string) {
                Ok(game_state_diff) => game_state_diff,
                Err(error) => {
                    panic!("failed to deserialize game state diff: {}", error);
                },
            };

            self.game_state.apply(&game_state_diff);

            update_count += 1;

            println!("update count: {}", update_count);

        }

    } 
    pub async fn draw(&mut self) {
        for entity in self.game_state.entities.iter_mut() {

            match entity {
                Entity::Player(player) => {player.draw(&mut self.textures, &self.camera_offset).await}
                Entity::Zombie(zombie) => {zombie.draw(&mut self.textures, &self.camera_offset).await}
                Entity::Bullet(bullet) => {bullet.draw(&self.camera_offset)},
                Entity::Coin(coin) => {coin.draw(&self.camera_offset)},
                Entity::Tree(tree) => {tree.draw(&mut self.textures, &self.camera_offset).await},
                Entity::Wood(wood) => {wood.draw(&mut self.textures, &self.camera_offset).await},
                Entity::Raft(raft) => {raft.draw(&self.camera_offset)},
                Entity::RaftComponent(raft_component) => {raft_component.draw(&self.camera_offset)},
                Entity::PhysicsSquare(physics_square) => {physics_square.draw(&self.camera_offset, &self.game_state.space).await}
            };
        }
    }

    pub fn connect(url: &str) -> Self {

        let uuid = uuid();

        println!("{}", uuid);

        let (server_send, server_receive) = match ewebsock::connect(url, ewebsock::Options::default()) {
            Ok(result) => result,
            Err(error) => {
                panic!("failed to connect to server: {}", error)
            },
        }; 

        // wait for Opened event from server
        loop {
            match server_receive.try_recv() {
                Some(event) => {
                    match event {
                        ewebsock::WsEvent::Opened => {
                            println!("we got the opened message!");
                            break;
                        },
                        ewebsock::WsEvent::Message(message) => {
                            match message {
                                _ => panic!("received a message from the server")
                            }
                        },
                        ewebsock::WsEvent::Error(error) => panic!("received error when trying to connect to server: {}", error),
                        ewebsock::WsEvent::Closed => panic!("server closed when trying to connect"),
                        
                    }
                },
                None => continue,
            }
        }

        let compressed_game_state_string_bytes = loop {

            match server_receive.try_recv() {
                Some(event) => {
                    match event {
                        ewebsock::WsEvent::Opened => todo!("unhandled opened event on connect"),
                        ewebsock::WsEvent::Message(message) => {
                            match message {
                                ewebsock::WsMessage::Binary(bytes) => break bytes,
                                _ => todo!("unhandled message type when receiving initial state")
                            }
                        },
                        ewebsock::WsEvent::Error(error) => todo!("unhandled error when receiving initial state: {}", error),
                        ewebsock::WsEvent::Closed => todo!("unhandled closed event when receiving initial state"),
                    }
                },
                None => continue, // this means that the server would have blocked, so we try again
            };
        };
        
        let game_state_string_bytes = decompress_size_prepended(&compressed_game_state_string_bytes).expect("Failed to decompress initial state");
        
        let game_state_string = match String::from_utf8(game_state_string_bytes.clone()) {
            Ok(game_state_diff_string) => game_state_diff_string,
            Err(error) => {
                panic!("failed to decode game state diff as string {}", error);
            },
        };

        

        let game_state: GameState = match serde_json::from_str(&game_state_string) {
            Ok(game_state_diff) => game_state_diff,
            Err(error) => {
                panic!("failed to deserialize game state diff: {}", error);
            },
        };
        
        Self {
            game_state: game_state.clone(),
            is_host: true,
            last_tick_game_state: game_state.clone(),
            textures: HashMap::new(),
            sounds: HashMap::new(),
            last_tick: Time::now(),
            uuid: uuid,
            server_receive: server_receive,
            server_send: server_send,
            camera_offset: Vec2::new(0., 0.),
            update_count: 0,
            start_time: Time::now()
        }
    }


    pub fn connect_as_master() {

    }

    pub fn control_camera(&mut self) {
        if is_key_down(macroquad::input::KeyCode::Right) {
            self.camera_offset.x += 1.0 * self.last_tick.elapsed().num_milliseconds() as f32;
        }

        if is_key_down(macroquad::input::KeyCode::Left) {
            self.camera_offset.x -= 1.0 * self.last_tick.elapsed().num_milliseconds() as f32;
        }

        if is_key_down(macroquad::input::KeyCode::Down) {
            self.camera_offset.y -= 1.0 * self.last_tick.elapsed().num_milliseconds() as f32;
        }

        if is_key_down(macroquad::input::KeyCode::Up) {
            self.camera_offset.y += 1.0 * self.last_tick.elapsed().num_milliseconds() as f32;
        }
    }

    pub fn tick(&mut self) {

        self.control_camera();

        if is_key_released(macroquad::input::KeyCode::F5) {
            let game_state_json = serde_json::to_string_pretty(&self.game_state).unwrap();

            std::fs::write("state.json", game_state_json).unwrap();
        }

        if is_mouse_button_released(macroquad::input::MouseButton::Left) {

            let mouse_pos = macroquad::input::mouse_position();

            self.game_state.entities.push( 
                PhysicsSquare::new(
                    &mut self.game_state.space,
                    Vec2::new(mouse_pos.0 + 20., mouse_pos.1 + 20.),
                    game::rigid_body::RigidBodyType::Dynamic,
                    20., 
                    20., 
                    &self.uuid,
                    false
                ).into()
            );
        }

        // we create a tick context because we cannot pass Client directly
        // we want others to be able to create their own client structs so TickContext is the middle man
        let mut tick_context = TickContext {
            game_state: &mut self.game_state,
            is_host: &mut self.is_host,
            textures: &mut self.textures,
            sounds: &mut self.sounds,
            last_tick: &mut self.last_tick,
            uuid: &mut self.uuid,
        };

        for index in 0..tick_context.game_state.entities.len() {

            // take the player out, tick it, then put it back in
            let mut entity = tick_context.game_state.entities.remove(index);

            // we only tick the entity if we own it
            if entity.get_owner() == *tick_context.uuid {
                entity.tick(&mut tick_context);
            }
            
            // put the entity back in the same index so it doesnt FUCK things up
            tick_context.game_state.entities.insert(index, entity)

        }

        self.last_tick = Time::now(); 

    }
}