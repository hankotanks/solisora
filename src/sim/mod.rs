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
            system_rad: 2.0,
            system_seed: None,
            sun_rad: 0.1,
            pl_size_multiplier: 0.1..0.3,
            pl_moon_prob: 0.5,
            pl_feat_prob: 0.5
        }
    }
}

pub struct Sim {
    pub system: Vec<Planet>,
    pub system_rad: f32,
    pub ships: Vec<Ship>
}

impl Default for Sim {
    fn default() -> Self {
        let config = SimConfig::default();
        Self::new(config)
    }
}

impl Sim {
    pub fn new(config: SimConfig) -> Self {
        // Calculates combined radius of a subsystem, centered at pl_index
        fn total_rad(system: &Vec<Planet>, pl_index: usize) -> f32 {
            let mut pl_rad = system[pl_index].rad;
            for &moon_index in system[pl_index].moon_indices.iter() {
                let dist = system[moon_index].orbit.as_ref().unwrap().dist;
                let dist = dist + total_rad(system, moon_index);
                pl_rad = pl_rad.max(dist);
            }
            
            pl_rad
        }

        fn orbit_distance(system: &Vec<Planet>, pl_index: usize, rad: f32) -> f32 {
            total_rad(&system, pl_index) + system[pl_index].rad + rad * 3f32
        }

        // Create an StdRng object from a seed, if it is provided
        let mut prng = match config.system_seed {
            Some(s) => SeedableRng::seed_from_u64(s),
            None => StdRng::from_entropy()
        };

        // Initialize system with sun
        let mut system = vec![Planet::new(config.sun_rad)];
        
        loop { // Populate system with planet subsystems
            let pl_index = system.len();
            let pl_rad = config.sun_rad * prng.gen_range(config.pl_size_multiplier.clone());
            system.push(Planet::new(pl_rad));

            while { 
                total_rad(&system, pl_index) < system[pl_index].rad * 5f32 &&
                prng.gen_bool(config.pl_moon_prob) 
            } {
                let moon_rad = prng.gen_range(config.pl_size_multiplier.clone());
                let moon_rad = moon_rad * system[pl_index].rad;
                let moon_index = system.len();

                let dist = orbit_distance(&system, pl_index, moon_rad);

                system[pl_index].moon_indices.push(moon_index);
                system.push(Planet::new(moon_rad));
                system[moon_index].orbit = Some(Orbit::new(pl_index, dist));
            }

            // Randomly add PlanetFeatures throughout the system
            for pl in system.iter_mut() {
                if prng.gen_bool(config.pl_feat_prob) {
                    pl.feat = Some(PlanetFeature::iter().choose(&mut prng).unwrap());
                }
            }

            let pl_system_rad = orbit_distance(&system, pl_index, system[pl_index].rad);

            // If the new system exceeds the SimConfig field 'system_rad'
            // Remove it and break
            let system_rad = total_rad(&system, 0);
            if system_rad + pl_system_rad > config.system_rad {
                system.truncate(pl_index);
                break;
            }

            // Update the sun's moon_indices field
            // Lastly, attach an orbit to the origin of the new subsytem
            system[0].moon_indices.push(pl_index);
            system[pl_index].orbit = Some(
                Orbit::new(0, system_rad + pl_system_rad)
            );
        };

        let system_rad = total_rad(&system, 0);
        
        Self {
            system,
            system_rad,
            ships: {
                Vec::new()
            }
        }
    }

    pub fn update(&mut self) {
        self.update_planet(0);
    }

    pub fn update_planet(&mut self, pl_index: usize) {
        fn dist_to_sun(pos: Point2<f32>, orbit: Orbit) -> f32 {
            Point2::new(
                pos.x + orbit.dist * orbit.angle.cos(),
                pos.y + orbit.dist * orbit.angle.sin()
            ).distance(
                (0f32, 0f32).into()
            )
        }

        if let Some(mut pl_orbit) = self.system[pl_index].orbit {
            let parent_pos = self.system[pl_orbit.parent_index].pos;

            // Calculate the angle offset
            // BEFORE taking the distance from the sun into account
            let mut offset = 0.0174f32; // equivalent to 1 degree
            offset *= pl_orbit.speed;
            offset *= self.system[0].rad / self.system[pl_orbit.parent_index].rad; // smaller bodies move faster
            offset *= if pl_orbit.ccw { -1f32 } else { 1f32 };

            let dist = {
                let mut temp_orbit = pl_orbit;
                temp_orbit.angle += offset;
                temp_orbit.angle %= TAU;
                dist_to_sun(parent_pos, temp_orbit) };
            offset *= (self.system_rad - dist) / self.system_rad; // the nearer a planet is, the FASTER it goes

            // update the current angle of the orbit
            pl_orbit.angle += offset;
            pl_orbit.angle %= TAU;

            // Update orbit and pos
            self.system[pl_index].pos = Point2::new(
                parent_pos.x + pl_orbit.dist * pl_orbit.angle.cos(),
                parent_pos.y + pl_orbit.dist * pl_orbit.angle.sin());
            self.system[pl_index].orbit = Some(pl_orbit);
        }

        // Update all of the current planet's moons
        for moon_index in self.system[pl_index].moon_indices.clone().drain(0..) {
            self.update_planet(moon_index);
        }
    }
}