use std::{collections::HashMap, net::TcpStream, time::Duration};

use diff::Diff;
use game::{entities::{physics_square, Entity}, game::{Drawable, HasOwner, HasRigidBody, Texture, TickContext, Tickable}, game_state::{GameState, GameStateDiff}, networking::{self, receive_headered}, proxies::macroquad::math::vec2::Vec2, time::Time, uuid};
use macroquad::{input::is_key_down, texture::Texture2D};
use serde_json::to_string_pretty;

pub struct Client {
    pub game_state: GameState,
    pub is_host: bool,
    pub last_tick_game_state: GameState,
    pub textures: HashMap<String, Texture2D>,
    pub sounds: HashMap<String, macroquad::audio::Sound>,
    pub last_tick: Time,
    pub uuid: String,
    pub server: TcpStream,
    pub camera_offset: Vec2
}

impl Client {

    pub async fn run(&mut self) {
        loop {
        
            //macroquad::window::clear_background(macroquad::color::BLACK);
    
            self.tick();
            
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
    
            //println!("{}",  macroquad::time::get_fps());
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

        match networking::send_headered(diff_string_bytes, &mut self.server) {
            Ok(_) => {}//println!("sent updates to the server!"),
            Err(error) => {
                match error.kind() {
                    std::io::ErrorKind::WouldBlock => {
                        todo!("tried to send update to server but it blocked: {}", error);
                    }

                    _ => {
                        println!("failed to send update to server: {}", error)
                    }
                }
            },
        }

    }

    pub fn receive_updates(&mut self) {

        let game_state_diff_string_bytes = match receive_headered(&mut self.server) {
            Ok(game_state_diff_string_bytes) => {
                println!("Received an update from the server!");
                game_state_diff_string_bytes
            },
            Err(error) => {
                match error.kind() {
                    std::io::ErrorKind::WouldBlock => {
                        //println!("tried to receive update from server but it would have blocked");
                        return;
                        
                    },
                    _ => {
                        println!("something went wrong trying to receive update from server: {}", error);
                        return;
                    }

                }
            },
        };
        
        let game_state_diff_string = match String::from_utf8(game_state_diff_string_bytes.clone()) {
            Ok(game_state_diff_string) => game_state_diff_string,
            Err(error) => {
                println!("failed to decode game state diff as string {}", error);
                return;
            },
        };

        

        let game_state_diff: GameStateDiff = match serde_json::from_str(&game_state_diff_string) {
            Ok(game_state_diff) => game_state_diff,
            Err(error) => {
                println!("failed to deserialize game state diff: {}", error);
                return;
            },
        };

        println!("new update: {}", serde_json::to_string_pretty(&game_state_diff).expect("sex"));

        self.game_state.apply(&game_state_diff);
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

    pub fn connect(address: &str) -> Self {

        let uuid = uuid();

        println!("{}", uuid);

        let mut server = match TcpStream::connect(address) {
            Ok(stream) => stream,
            Err(error) => {
                panic!("failed to connect: {}", error)
            },
        };

        let game_state_string_bytes = match receive_headered(&mut server) {
            Ok(game_state_diff_string_bytes) => game_state_diff_string_bytes,
            Err(error) => {
                panic!("failed to receive game state from server: {}", error);
            },
        };
        
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

        println!("Initial state: {}", serde_json::to_string_pretty(&game_state).expect("sex"));
        println!("end initial state");

        server.set_nonblocking(true).expect("failed to set socket as non blocking");
        
        Self {
            game_state: game_state.clone(),
            is_host: true,
            last_tick_game_state: game_state.clone(),
            textures: HashMap::new(),
            sounds: HashMap::new(),
            last_tick: Time::now(),
            uuid: uuid,
            server: server,
            camera_offset: Vec2::new(0., 0.)
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