use std::time::Instant;

use macroquad::{color::RED, input::{self, MouseButton}, math::Vec2};

use crate::game::{Collidable, Controllable, Damagable, Draggable, Friction, Game, Moveable, Rect, Scale, Sound, Texture, Tickable, Velocity};

use super::Entity;

pub struct Player {
    pub rect: macroquad::math::Rect,
    pub velocity: macroquad::math::Vec2,
    pub friction_coefficient: f32,
    pub health: i32,
    pub acceleration: f32,
    pub texture_path: String,
    pub attacking: bool,
    pub attack_start: Instant,
    pub scale: Vec2,
    pub up_bind: macroquad::input::KeyCode,
    pub down_bind: macroquad::input::KeyCode,
    pub left_bind: macroquad::input::KeyCode,
    pub right_bind: macroquad::input::KeyCode,
    pub sound_path: String,
    pub dragging: bool
}

impl Player {
    pub fn new() -> Self {
        Self {
            rect: macroquad::math::Rect {x: 30.0, y: 30.0, w: 85.0, h: 100.0},
            scale: Vec2::new(5.0, 5.0),
            acceleration: 30.0,
            texture_path: "assets/player.png".to_string(),
            velocity: Vec2{x: 0.0, y: 0.0},
            health: 100,
            friction_coefficient: 20.,
            up_bind: macroquad::input::KeyCode::W,
            down_bind: macroquad::input::KeyCode::S,
            left_bind: macroquad::input::KeyCode::A,
            right_bind: macroquad::input::KeyCode::D,
            attacking: false,
            attack_start: Instant::now(),
            sound_path: "assets/sounds/pickup.wav".to_string(),
            dragging: false
        }
    }
    fn attack(&mut self, game: &mut Game) {
        
        let attack_hitbox = macroquad::math::Rect::new(self.get_rect().right(), self.get_rect().y, 30.0, 50.0);

        macroquad::shapes::draw_rectangle(attack_hitbox.x, attack_hitbox.y, attack_hitbox.w, attack_hitbox.h, RED);

        self.attacking = true;
        self.attack_start = Instant::now();

        self.set_texture_path("assets/player_attack.png".to_string());

        for entity in game.entities.iter_mut() {

            // only attack if entity is a player
            if let Entity::Player(player) = entity {
                if attack_hitbox.overlaps(&player.rect) {
                    player.damage(2);

                    println!("{}", player.get_health());
                }
            }
        }
    }
}

impl Tickable for Player {
    fn tick(&mut self, game: &mut crate::game::Game) {

        {
            self.control(game.last_tick.elapsed());
        }

        {
            for entity in game.entities.iter_mut() {

                if let Entity::Player(player) = entity {
                    self.collide(player, game.last_tick.elapsed());
                }
            }
        }
        
        {
            if input::is_mouse_button_down(MouseButton::Left) {
                self.attack(game);

                println!("attacking!");
            }
            else {
                println!("Resetting texture!");
                self.set_texture_path("assets/player.png".to_string());
            }
        }

        {
            self.move_by_velocity(game.last_tick.elapsed());
        }

        {
            self.drag(); 
        }
        
        {
            self.apply_friction(game.last_tick.elapsed());
        }

    }
}


impl Draggable for Player {
    fn get_dragging(&self) -> bool {
        self.dragging
    }

    fn set_dragging(&mut self, dragging: bool) {
        self.dragging = dragging
    }
}

impl Sound for Player {
    fn get_sound_path(&self) -> String {
        self.sound_path.clone()
    }

    fn set_sound_path(&mut self, sound_path: String) {
       self.sound_path = sound_path; 
    }  
}

impl Damagable for Player {
    fn get_health(&self) -> i32 {
        self.health
    }

    fn set_health(&mut self, health: i32) {
        self.health = health
    }
}

impl Rect for Player {
    fn get_rect(&self) -> macroquad::math::Rect {
        self.rect
    }
    fn set_rect(&mut self, rect: macroquad::math::Rect) {
        self.rect = rect;
    }
}

impl Velocity for Player {
    fn get_velocity(&self) -> macroquad::math::Vec2 {
        self.velocity
    }

    fn set_velocity(&mut self, velocity: macroquad::math::Vec2) {
        self.velocity = velocity
    }
}

impl Texture for Player {
    fn get_texture_path(&self) -> String {
        self.texture_path.clone()
    }

    fn set_texture_path(&mut self, texture_path: String) {
        self.texture_path = texture_path;
    }
}

impl Scale for Player {
    fn get_scale(&self) -> macroquad::math::Vec2 {
        self.scale
    }
}

impl Moveable for Player {}

impl Collidable for Player {}

impl Controllable for Player {
    fn get_acceleration(&self) -> f32 {
        self.acceleration
    }
    fn up_bind(&mut self) -> macroquad::input::KeyCode {
        self.up_bind
    }
    fn down_bind(&mut self) -> macroquad::input::KeyCode {
        self.down_bind
    }
    fn left_bind(&mut self) -> macroquad::input::KeyCode {
        self.left_bind
    }
    fn right_bind(&mut self) -> macroquad::input::KeyCode {
        self.right_bind
    }
}

impl Friction for Player {
    fn friction_coefficient(&self) -> f32 {
        self.friction_coefficient   
    }
}


