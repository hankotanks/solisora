use std::f32::consts::TAU;

use rand::Rng;
use cgmath::{
    Point2, 
    MetricSpace
};

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

impl std::hash::Hash for Orbit {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.parent.hash(state);
        self.distance.to_string().hash(state);
        // Orbit::angle isn't hashed because it is constantly variable
        self.counterclockwise.hash(state);
        self.speed.to_string().hash(state);
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

    pub(super) fn update(&mut self, center: Point2<f32>, multiplier: f32) -> Point2<f32> {
        let mut offset = 0.0174f32;

        offset *= self.speed;
        offset *= multiplier;
        
        if self.counterclockwise {
            offset *= -1f32;
        }

        let distance = {
            let angle = (self.angle + offset) % TAU;
            Point2::new(
                center.x + self.distance * angle.cos(),
                center.y + self.distance * angle.sin()
            ).distance((0f32, 0f32).into())
        };

        // Bodies further from the Sun move slower
        let distance_multiplier = 1f32 - (distance / 2f32.sqrt());
        offset *= distance_multiplier;

        self.angle += offset;
        self.angle %= TAU;

        Point2::new(
            center.x + self.distance * self.angle.cos(),
            center.y + self.distance * self.angle.sin()
        )
    }
}