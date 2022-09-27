pub mod ship;
pub mod planet;

use std::{
    f32::consts::{PI, TAU},
    ops::Range,
    mem::discriminant,
    cmp::Ordering::Equal 
};

use rand::{
    Rng, 
    SeedableRng, 
    seq::IteratorRandom, 
    rngs::StdRng 
};

use cgmath::{
    Point2, 
    MetricSpace, 
    Rad, 
    Angle 
};

use strum::IntoEnumIterator;

use ship::{
    Ship,
    ShipJob,
    ShipGoal 
};

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
    pl_moon_prob: f64,
    pl_feat_prob: f64,
    pl_size_multiplier: Range<f32>,
    ship_count: usize,
    ship_mine_progress: usize,
    ship_speed: f32,
    ship_resource_cost: usize
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            system_rad: 2.0,
            system_seed: None,
            sun_rad: 0.1,
            pl_moon_prob: 0.5,
            pl_feat_prob: 0.5,
            pl_size_multiplier: 0.1..0.5,
            ship_count: 4,
            ship_mine_progress: 100,
            ship_speed: 0.01,
            ship_resource_cost: 4
        }
    }
}

pub struct Sim {
    pub prng: StdRng,
    pub system: Vec<Planet>,
    pub system_rad: f32,
    pub ships: Vec<Ship>,
    pub config: SimConfig
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
                // Recursively find the total orbital radius of the moon
                let dist = system[moon_index].orbit.as_ref().unwrap().dist;
                let dist = dist + total_rad(system, moon_index);

                // Check if this orbit is maximal
                pl_rad = pl_rad.max(dist);
            }
            
            pl_rad
        }

        // Helper function
        fn dist_to_padded_orbit(system: &Vec<Planet>, pl_index: usize, rad: f32) -> f32 {
            total_rad(system, pl_index) + system[pl_index].rad + rad * 3f32
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

                let dist = dist_to_padded_orbit(&system, pl_index, moon_rad);
                system[pl_index].moon_indices.push(moon_index);
                system.push(Planet::new(moon_rad));
                system[moon_index].orbit = Some(Orbit::new(pl_index, dist));
            }

            // Total radius of the planet subsystem
            let pl_system_rad = dist_to_padded_orbit(&system, pl_index, system[pl_index].rad);

            // If the new system exceeds the SimConfig field 'system_rad'
            // Remove it and break
            let system_rad = total_rad(&system, 0);
            if system_rad + pl_system_rad > config.system_rad {
                system.truncate(pl_index);
                break;
            }

            // Update the sun's moon_indices field
            // Lastly, attach an orbit to the origin of the new subsystem
            system[0].moon_indices.push(pl_index);
            system[pl_index].orbit = Some(
                Orbit::new(0, system_rad + pl_system_rad)
            );
        };

        // There needs to be at least 4 planets for the ships to have proper behavior
        // SUN -- 2 w/ STATIONS -- 1 w/ RESOURCES
        if system.len() < 4 {
            panic!()
        }

        // Ensure that planets with essential features are present
        let rand_pl_index = prng.gen_range(2..system.len());
        system[1].feat = Some(PlanetFeature::Station { num_resources: 0 } );
        system[rand_pl_index].feat = Some(PlanetFeature::Resources);
        system.last_mut().unwrap().feat = Some(PlanetFeature::Station { num_resources: 0 } );

        // Randomly add PlanetFeatures throughout the system
        for pl in system.iter_mut().skip(1) {
            if prng.gen_bool(config.pl_feat_prob) && pl.feat.is_none() {
                pl.feat = Some(PlanetFeature::iter().choose(&mut prng).unwrap());
            }
        }

        // The ACTUAL radius of the system, in contrast to config.system_rad
        let system_rad = total_rad(&system, 0);

        // Used to choose a destination for new ships
        let stations = filter_system(&system, Some(PlanetFeature::Station { num_resources: 0 } ));
        let resources = filter_system(&system, Some(PlanetFeature::Resources));

        // Ships start at random points, with random destinations
        // Initial goals are specific to each ship's job
        let mut ships = Vec::new();
        for _ in 0..config.ship_count {
            let mut ship = Ship::new(ShipJob::Miner, config.ship_speed);

            // Use polar coordinates to ensure an even distribution of values
            let r = system_rad * prng.gen::<f32>().sqrt();
            let theta = prng.gen::<f32>() * TAU;

            // Convert position to cartesian coordinates, then assign goal
            ship.pos = Point2::new(r * theta.cos(), r * theta.sin());
            ship.goal = match ship.job {
                ShipJob::Miner => ShipGoal::Visit { target: *resources.iter().choose(&mut prng).unwrap() },
                ShipJob::Trader { .. } => ShipGoal::Visit { target: *stations.iter().choose(&mut prng).unwrap() } };
            ships.push(ship);
        }

        Self {
            prng,
            system,
            system_rad,
            ships,
            config
        }        
    }

    pub fn update(&mut self) {
        // Update positions of all planets
        self.update_planet_pos(0);

        // Spawn new ships from stations with sufficient resources
        for pl_index in 0..self.system.len() {
            if let Some(PlanetFeature::Station { ref mut num_resources } ) = self.system[pl_index].feat {
                if *num_resources > self.config.ship_resource_cost {
                    *num_resources -= self.config.ship_resource_cost;
                    let mut ship = Ship::new(ShipJob::Trader { has_resource: false }, self.config.ship_speed);
                    ship.pos = self.system[pl_index].pos;
                    ship.goal = ShipGoal::Visit { target: pl_index };
                    self.ships.push(ship);
                }
            }
        }

        // Update every ship
        for ship_index in 0..self.ships.len() {
            self.update_ship(ship_index);
        }
    }

    pub fn update_planet_pos(&mut self, pl_index: usize) {
        fn dist_to_sun(pos: Point2<f32>, orbit: Orbit) -> f32 {
            Point2::new(
                pos.x + orbit.dist * orbit.angle.cos(),
                pos.y + orbit.dist * orbit.angle.sin()
            ).distance(
                (0f32, 0f32).into()
            )
        }

        // Don't update the sun's position... it doesn't move
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
            if pl_orbit.parent_index == 0 {
                offset *= (self.system_rad - dist).sqrt() / self.system_rad; // the nearer a planet is, the FASTER it goes
            }

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
            self.update_planet_pos(moon_index);
        }
    }

    pub fn update_ship(&mut self, ship_index: usize) {     
        match self.ships[ship_index].goal {
            ShipGoal::Visit { target: pl_index } => {
                // Update ship objective IFF it has reached its destination
                let pl_pos = self.system[pl_index].pos;
                if self.ships[ship_index].pos.distance2(pl_pos) <= self.system[pl_index].rad.powf(2f32) {
                    self.update_ship_objective(ship_index);
                    self.ships[ship_index].speed = self.ships[ship_index].initial_speed; // reset speed
                }

                // Position offsets
                let dx = pl_pos.x - self.ships[ship_index].pos.x;
                let dy = pl_pos.y - self.ships[ship_index].pos.y;

                // Update position, angle and increase speed
                self.ships[ship_index].pos.x += dx * self.ships[ship_index].speed;
                self.ships[ship_index].pos.y += dy * self.ships[ship_index].speed;
                self.ships[ship_index].angle = Rad::atan2(dx, dy).0 + PI;
                self.ships[ship_index].speed *= 1.05f32;
            },

            ShipGoal::Wait { target: pl_index, progress } => {
                // Ships dock on planets while waiting
                self.ships[ship_index].pos = self.system[pl_index].pos;

                // Update ship objective if the ship is done mining
                if progress < self.config.ship_mine_progress {
                    self.ships[ship_index].goal = ShipGoal::Wait { target: pl_index, progress: progress + 1 }
                } else {
                    self.update_ship_objective(ship_index);
                }
            }
        }
    }

    fn update_ship_objective(&mut self, ship_index: usize) {
        // Returns a mutable reference to the num_resources field of a station on a given planet
        // Panics if it doesn't have a station
        fn num_resources(pl: &mut Planet) -> &mut usize {
            if let Some(PlanetFeature::Station { ref mut num_resources } ) = pl.feat {
                return num_resources;
            }
        
            panic!()
        }
        
        // All ship logic occurs in this match expression
        self.ships[ship_index].goal = match (self.ships[ship_index].job, self.ships[ship_index].goal) {
            (ShipJob::Trader { has_resource }, ShipGoal::Visit { target: curr_pl_index } ) => {
                // Deliver resources if the Trader was carrying them
                if has_resource {
                    *num_resources(&mut self.system[curr_pl_index]) += 1;
                    self.ships[ship_index].job = ShipJob::Trader { has_resource: false };
                }

                // Find the ship's new destination
                let mut stations = filter_system(&self.system, 
                    Some(PlanetFeature::Station { num_resources: 0 } ));
                stations.retain(|pl| *pl != curr_pl_index);
                let dest_pl_index = *stations.iter().choose(&mut self.prng).unwrap();

                // Determine if the ship should carry resources from one stations to the new destination
                let curr_pl_resources = *num_resources(&mut self.system[curr_pl_index]);
                let dest_pl_resources = *num_resources(&mut self.system[dest_pl_index]);
                if curr_pl_resources > dest_pl_resources {
                    *num_resources(&mut self.system[curr_pl_index]) -= 1; // take resource from station...
                    self.ships[ship_index].job = ShipJob::Trader { has_resource: true }; // give to ship
                }                     
                
                ShipGoal::Visit { target: dest_pl_index }
            },

            (ShipJob::Miner, ShipGoal::Visit { target: curr_pl_index } ) => {
                // The ship's behavior depends on the type of planet is just visited
                match self.system[curr_pl_index].feat {
                    Some(PlanetFeature::Station { .. } ) => {
                        // Deposit mined resources at the station
                        *num_resources(&mut self.system[curr_pl_index]) += 1;

                        // Visit a new planet with resources
                        let resources = nearest_with_feature(
                            &self.system, 
                            Some(PlanetFeature::Resources), 
                            self.ships[ship_index].pos);
                        ShipGoal::Visit { target: resources[0] }
                    },
                    Some(PlanetFeature::Resources) => {
                        ShipGoal::Wait { target: curr_pl_index, progress: 0 } // pause to mine resources
                    },
                    _ => panic!()
                }
            },

            (ShipJob::Miner, ShipGoal::Wait { .. } ) => {
                // After mining, the ship travels to the nearest station to deposit its resources
                let stations = nearest_with_feature(
                    &self.system, 
                    Some(PlanetFeature::Station { num_resources: 0 } ), 
                    self.ships[ship_index].pos);
                ShipGoal::Visit { target: stations[0] }
            },
            
            _ => self.ships[ship_index].goal
        };
    
    }
}

fn filter_system(system: &[Planet], filter: Option<PlanetFeature>) -> Vec<usize> {
    let mut pl_indices = Vec::new();
    for (pl_index, pl) in system.iter().enumerate() {
        if match &filter {
            Some(filter) => {
                if let Some(feat) = &pl.feat {
                    discriminant(filter) == discriminant(feat)
                } else {
                    false
                }
            },
            None => {
                pl.feat.is_none()
            }
        } {
            pl_indices.push(pl_index)
        }
    }

    pl_indices
}

fn nearest_with_feature(system: &[Planet], filter: Option<PlanetFeature>, pos: Point2<f32>) -> Vec<usize> {
    let mut pl_indices = filter_system(system, filter);
    pl_indices.sort_by(|&a, &b| {
        let dist_a = pos.distance2(system[a].pos);
        let dist_b = pos.distance2(system[b].pos);

        dist_a.partial_cmp(&dist_b).unwrap_or(Equal)
    } );

    pl_indices
}