use std::f32::consts::TAU;
use rand::Rng;

use crate::simulation::point;

#[derive(Copy, Clone)]
pub(super) struct Orbit {
    parent: usize,
    distance: f32,
    angle: f32,
    counterclockwise: bool,
    speed: f32
}

impl Orbit {
    pub(super) fn parent(&self) -> usize {
        self.parent
    }

    pub(super) fn distance(&self) -> f32 {
        self.distance
    }
}

impl Orbit {
    pub(super) fn new(parent: usize, distance: f32) -> Self {
        Self {
            parent,
            distance, 
            angle: rand::thread_rng().gen_range(0f32..TAU),
            counterclockwise: rand::thread_rng().gen_bool(0.5f64),
            speed: rand::thread_rng().gen_range(1..4) as f32 * 0.5f32
        }
    }

    pub(super) fn update(&mut self, center: point::Point, multiplier: f32) -> point::Point {
        let mut offset = 0.0174f32;

        offset *= self.speed;
        offset *= multiplier;
        
        if self.counterclockwise {
            offset *= -1f32;
        }

        self.angle += offset;
        self.angle %= TAU;

        point::Point::new(
            center.x() + self.distance * self.angle.cos(),
            center.y() + self.distance * self.angle.sin()
        )
    }
}