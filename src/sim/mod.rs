pub mod ship;
pub mod planet;

use std::{
    f32::consts::TAU,
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
    ShipType,
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
    num_ships: usize
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
            num_ships: 10
        }
    }
}

pub struct Sim {
    pub prng: StdRng,
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

        // Helper function
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

            // Total radius of the planet subsystem
            let pl_system_rad = orbit_distance(&system, pl_index, system[pl_index].rad);

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

        // There needs to be at least 3 planets for the ships to have proper hbehavior
        if system.len() < 3 {
            panic!()
        }

        // Ensure that planets with essential features are present
        system[1].feat = Some(PlanetFeature::Station { num_resources: 0 } );
        system.last_mut().unwrap().feat = Some(PlanetFeature::Resources);

        // Randomly add PlanetFeatures throughout the system
        for pl in system.iter_mut().skip(1) {
            if prng.gen_bool(config.pl_feat_prob) && pl.feat.is_none() {
                pl.feat = Some(PlanetFeature::iter().choose(&mut prng).unwrap());
            }
        }

        let mut sim = Self {
            prng,
            system,
            system_rad: 0f32,
            ships: Vec::new()
        };

        sim.system_rad = total_rad(&sim.system, 0);
        sim.update_planet_pos(0);

        // Ships start near planets with stations
        let resources = filter_system(&sim.system, Some(PlanetFeature::Resources));
        let stations = filter_system(&sim.system, Some(PlanetFeature::Station { num_resources: 0 } ));
        for _ in 0..config.num_ships {
            let mut ship = Ship::new(ShipType::iter().choose(&mut sim.prng).unwrap());
            ship.pos = sim.system[*stations.iter().choose(&mut sim.prng).unwrap()].pos;
            ship.goal = match ship.ship_type {
                ShipType::Miner => ShipGoal::Visit { target: *resources.iter().choose(&mut sim.prng).unwrap() },
                ShipType::Trader { .. } => ShipGoal::Visit { target: *stations.iter().choose(&mut sim.prng).unwrap() }
            };

            sim.ships.push(ship);
        }

        sim
    }

    pub fn update(&mut self) {
        // Update positions of all planets
        self.update_planet_pos(0);

        // Spawn new ships from stations with sufficient resources
        for pl_index in 0..self.system.len() {
            if let Some(PlanetFeature::Station { ref mut num_resources } ) = self.system[pl_index].feat {
                if *num_resources > 10 {
                    *num_resources -= 10;
                    let mut ship = Ship::new(ShipType::Trader { has_resource: false } );
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
        match self.ships[ship_index].goal.clone() {
            ShipGoal::Visit { target: pl_index } => {
                // Update ship objective IFF it has reached its destination
                let pl_pos = self.system[pl_index].pos;
                if self.ships[ship_index].pos.distance2(pl_pos) <= self.system[pl_index].rad.powf(2f32) {
                    self.change_ship_objective(ship_index);
                    self.ships[ship_index].speed = self.ships[ship_index].initial_speed; // reset speed
                }

                // Position offsets
                let dx = pl_pos.x - self.ships[ship_index].pos.x;
                let dy = pl_pos.y - self.ships[ship_index].pos.y;

                // Update position, angle and increase speed
                self.ships[ship_index].pos.x += dx * self.ships[ship_index].speed;
                self.ships[ship_index].pos.y += dy * self.ships[ship_index].speed;
                self.ships[ship_index].angle = Rad::atan2(dx, dy).0 + 3.14;
                self.ships[ship_index].speed *= 1.05f32;
            },
            ShipGoal::Wait { target: pl_index, counter } => {
                // Ships dock on planets while waiting
                self.ships[ship_index].pos = self.system[pl_index].pos;

                if counter > 0usize {
                    self.ships[ship_index].goal = ShipGoal::Wait { target: pl_index, counter: counter - 1 }
                } else {
                    self.change_ship_objective(ship_index);
                }
            }
        }
    }

    fn change_ship_objective(&mut self, ship_index: usize) {
        // Panics if the given planet doesn't have a station
        fn num_resources(pl: &Planet) -> usize {
            if let Some(PlanetFeature::Station { num_resources }) = pl.feat {
                return num_resources
            }
        
            panic!()
        }
        
        // Returns a mutable reference to the num_resources field of a station on a given planet
        // Panics if it doesn't have a station
        fn modify_num_resources(pl: &mut Planet) -> &mut usize {
            if let Some(PlanetFeature::Station { ref mut num_resources } ) = pl.feat {
                return num_resources;
            }
        
            panic!()
        }
        
        // All ship logic occurs in this match expression
        self.ships[ship_index].goal = match (self.ships[ship_index].ship_type, self.ships[ship_index].goal) {
            (ShipType::Trader { has_resource }, ShipGoal::Visit { target: curr_pl_index } ) => {
                // Deliver resources if the Trader was carrying them
                if has_resource {
                    *modify_num_resources(&mut self.system[curr_pl_index]) += 1;
                    self.ships[ship_index].ship_type = ShipType::Trader { has_resource: false };
                }

                // Find the ship's new destination
                let stations = filter_system(&self.system, 
                    Some(PlanetFeature::Station { num_resources: 0 } ));
                let dest_pl_index = *stations.iter().choose(&mut self.prng).unwrap();

                // Determine if the ship should carry resources from one stations to the new destination
                let curr_pl_resources = num_resources(&self.system[curr_pl_index]);
                let dest_pl_resources = num_resources(&self.system[dest_pl_index]);
                if curr_pl_resources > dest_pl_resources {
                    *modify_num_resources(&mut self.system[curr_pl_index]) -= 1; // take resource from station...
                    self.ships[ship_index].ship_type = ShipType::Trader { has_resource: true }; // give to ship
                }                     
                
                ShipGoal::Visit { target: dest_pl_index }
            },

            (ShipType::Miner, ShipGoal::Visit { target: curr_pl_index } ) => {
                // The ship's behavior depends on the type of planet is just visited
                match self.system[curr_pl_index].feat {
                    Some(PlanetFeature::Station { .. } ) => {
                        // Deposit mined resources at the station
                        *modify_num_resources(&mut self.system[curr_pl_index]) += 1;
                        let resources = nearest_with_feature(
                            &self.system, 
                            Some(PlanetFeature::Resources), 
                            self.ships[ship_index].pos);
                        ShipGoal::Visit { target: resources[0] } // visit a new planet with resources
                    },
                    Some(PlanetFeature::Resources) => {
                        ShipGoal::Wait { target: curr_pl_index, counter: 100 } // pause to mine resources
                    },
                    _ => panic!()
                }
            },

            (ShipType::Miner, ShipGoal::Wait { .. } ) => {
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

fn filter_system(system: &Vec<Planet>, filter: Option<PlanetFeature>) -> Vec<usize> {
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

fn nearest_with_feature(system: &Vec<Planet>, filter: Option<PlanetFeature>, pos: Point2<f32>) -> Vec<usize> {
    let mut pl_indices = filter_system(system, filter);
    pl_indices.sort_by(|&a, &b| {
        pos.distance2(system[a].pos).partial_cmp(&pos.distance2(system[b].pos)).unwrap_or(Equal)
    } );

    pl_indices
}