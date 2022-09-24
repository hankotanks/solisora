use std::f32::consts::TAU;
use rand::Rng;
use strum::EnumIter;

#[derive(Copy, Clone)]
pub struct Orbit {
    pub parent_index: usize,
    pub dist: f32,
    pub speed: f32,
    pub ccw: bool,
    pub angle: f32    
}

impl Orbit {
    pub fn new(parent_index: usize, dist: f32) -> Self {
        let mut prng = rand::thread_rng();
        Self {
            parent_index,
            dist,
            speed: 0.5f32 * prng.gen_range(1..4) as f32,
            ccw: prng.gen_bool(0.5f64),
            angle: prng.gen_range(0f32..TAU)
        }
    }
}

pub struct Planet {
    pub pos: cgmath::Point2<f32>,
    pub rad: f32,
    pub orbit: Option<Orbit>,
    pub feat: Option<PlanetFeature>,
    pub moon_indices: Vec<usize>
}

impl Planet {
    pub fn new(radius: f32) -> Self {
        Self {
            pos: (0f32, 0f32).into(),
            rad: radius,
            orbit: None,
            feat: None,
            moon_indices: Vec::new()
        }
    }
}

#[derive(EnumIter)]
pub enum PlanetFeature {
    Station,
    Resources
}