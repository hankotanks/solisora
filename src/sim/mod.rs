pub mod ship;
pub mod planet;

use std::f32::consts::TAU;
use std::ops::Range;
use strum::IntoEnumIterator;

use rand::{
    Rng, 
    SeedableRng, 
    seq::IteratorRandom, 
    rngs::StdRng
};

use cgmath::{
    Point2, 
    MetricSpace
};

use ship::Ship;
use planet::{
    Planet,
    Orbit,
    PlanetFeature
};

pub struct Sim {
    pub system: Vec<Planet>,
    pub ships: Vec<Ship>
}

#[derive(Clone)]
pub struct SimConfig {
    system_rad: f32,
    system_seed: Option<u64>,
    sun_rad: f32,
    pl_size_multiplier: Range<f32>,
    pl_moon_prob: f64,
    pl_feat_prob: f64
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            system_rad: 1.0,
            system_seed: None,
            sun_rad: 0.1,
            pl_size_multiplier: 0.1..0.3,
            pl_moon_prob: 0.5,
            pl_feat_prob: 0.5
        }
    }
}

impl Default for Sim {
    fn default() -> Self {
        let config = SimConfig::default();
        Self::new(config)
    }
}

impl Sim {
    pub fn new(config: SimConfig) -> Self {
        fn total_rad(system: &Vec<Planet>, pl_index: usize) -> f32 {
            let mut pl_rad = system[pl_index].rad;
            for &moon_index in system[pl_index].moon_indices.iter() {
                let dist = system[moon_index].orbit.as_ref().unwrap().dist;
                let dist = dist + total_rad(system, moon_index);
                pl_rad = pl_rad.max(dist);
            }
            
            pl_rad
        }

        let mut prng = match config.system_seed {
            Some(s) => SeedableRng::seed_from_u64(s),
            None => StdRng::from_entropy()
        };
        
        Self {
            system: {
                let mut system = vec![
                    Planet::new(config.sun_rad)
                ];

                loop {
                    let pl_index = system.len();
                    let pl_rad = config.sun_rad * prng.gen_range(config.pl_size_multiplier.clone());
                    system.push(Planet::new(pl_rad));

                    while { 
                        let rad = total_rad(&system, pl_index);
                        let padding = system[pl_index].rad * 5f32;
                        rad < padding && prng.gen_bool(config.pl_moon_prob) 
                    } {
                        let moon_rad = prng.gen_range(config.pl_size_multiplier.clone());
                        let moon_rad = moon_rad * system[pl_index].rad;
                        let moon_index = system.len();

                        let dist = total_rad(&system, pl_index);
                        let dist = dist + system[pl_index].rad + moon_rad * 3f32;

                        system[pl_index].moon_indices.push(moon_index);
                        system.push(Planet::new(moon_rad));
                        system[moon_index].orbit = Some(Orbit::new(pl_index, dist));
                    }

                    for pl in system.iter_mut() {
                        if prng.gen_bool(config.pl_feat_prob) {
                            pl.feat = Some(PlanetFeature::iter().choose(&mut prng).unwrap());
                        }
                    }

                    let pl_system_rad = total_rad(&system, pl_index);
                    let pl_system_rad = pl_system_rad + system[pl_index].rad * 3f32;

                    let system_rad = total_rad(&system, 0);
                    if system_rad + pl_system_rad > config.system_rad {
                        system.truncate(pl_index);
                        break;
                    }

                    system[0].moon_indices.push(pl_index);
                    system[pl_index].orbit = Some(
                        Orbit::new(0, system_rad + pl_system_rad)
                    );
                }

                system
            },
            ships: {
                Vec::new()
            }
        }
    }

    pub fn update(&mut self) {
        self.update_planet(0);
    }

    pub fn update_planet(&mut self, pl_index: usize) {
        if let Some(mut pl_orbit) = self.system[pl_index].orbit {
            let parent_pos = self.system[pl_orbit.parent_index].pos;
            let parent_rad = self.system[pl_orbit.parent_index].rad;

            let mut offset = 0.0174f32;
            offset *= pl_orbit.speed;
            offset *= self.system[0].rad / parent_rad;
            offset *= if pl_orbit.ccw { -1f32 } else { 1f32 };

            let d = {
                let angle = (pl_orbit.angle + offset) % TAU;
                Point2::new(
                    parent_pos.x + pl_orbit.dist * angle.cos(),
                    parent_pos.y + pl_orbit.dist * angle.sin()
                ).distance((0f32, 0f32).into())
            };
            
            offset *= 1f32 - (d / 2f32.sqrt());

            pl_orbit.angle += offset;
            pl_orbit.angle %= TAU;

            self.system[pl_index].pos = Point2::new(
                parent_pos.x + pl_orbit.dist * pl_orbit.angle.cos(),
                parent_pos.y + pl_orbit.dist * pl_orbit.angle.sin()
            );
            self.system[pl_index].orbit = Some(pl_orbit);
        }

        for m in self.system[pl_index].moon_indices.clone().drain(0..) {
            self.update_planet(m);
        }
    }
}