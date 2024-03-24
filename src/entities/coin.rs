use crate::{entities::{player::Player, Entity}, game::{Color, Drawable, Friction, Moveable, HasRect, Tickable, Velocity}};
use crate::proxies::macroquad::math::{vec2::Vec2, rect::Rect};

pub struct Coin {
    color: macroquad::color::Color,
    rect: Rect,
    velocity: Vec2,
    friction_coefficient: f32
}

impl Coin {
    pub fn new(x: f32, y: f32) -> Self {
        Coin {
            color: macroquad::color::YELLOW,
            rect: Rect { x: x, y: y, w: 10.0, h: 10.0 },
            velocity: Vec2{x: 0.0, y: 0.0},
            friction_coefficient: 1.
        }
    }

    pub fn gravitate(&mut self, player: &mut Player) {
        let distance = Vec2 {
            x: self.get_rect().center().x - player.get_rect().center().x,
            y: self.get_rect().center().y - player.get_rect().center().y
        };

        if distance.length() <= 300.0 {

            let exponential_factor = -0.05 * distance.length();

            self.velocity += distance.normalize() * exponential_factor;

        }
    }
}

impl Moveable for Coin {}

impl Friction for Coin {
    fn friction_coefficient(&self) -> f32 {
        self.friction_coefficient
    }
}

impl Velocity for Coin {
    fn get_velocity(&self) -> Vec2 {
        self.velocity
    }

    fn set_velocity(&mut self, velocity: Vec2) {
        self.velocity = velocity
    }
}

impl Tickable for Coin {
    fn tick(&mut self, game: &mut crate::game::Game) {
        for entity in game.game_state.entities.iter_mut() {
            
            if let Entity::Player(player) = entity {
                self.gravitate(player);
            }
        }

        self.move_by_velocity(game.last_tick.elapsed());

        self.apply_friction(game.last_tick.elapsed())
    }
}

impl Color for Coin {
    fn color(&self) -> macroquad::color::Color {  
        self.color
    }
}

impl HasRect for Coin {
    fn get_rect(&self) -> Rect {
        self.rect
    }
    fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }
}

impl Drawable for Coin {}

