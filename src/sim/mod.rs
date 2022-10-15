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
    ship_speed: f32,
    ship_acceleration: f32,
    ship_cost: usize,
    miner_count: usize,
    harvest_duration: usize,
    harvest_variance: Range<isize>,
    pirate_count: usize,
    pirate_territory: f32,
    raid_range: f32,
    raid_duration: usize,
    raid_variance: Range<isize>,
    death_prob: f64
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            system_rad: 2.0,
            system_seed: None,
            sun_rad: 0.1,
            pl_moon_prob: 0.5,
            pl_feat_prob: 0.8,
            pl_size_multiplier: 0.1..0.3,
            ship_speed: 0.005,
            ship_acceleration: 1.05,
            ship_cost: 4,
            miner_count: 16,
            harvest_duration: 100,
            harvest_variance: -20..20,
            pirate_count: 8,
            pirate_territory: 0.4,
            raid_range: 0.2,
            raid_duration: 40,
            raid_variance: -20..20,
            death_prob: 0.4
        }
    }
}

pub struct Sim {
    pub prng: StdRng,
    pub system: Vec<Planet>,
    pub system_rad: f32,
    pub ships: Vec<Ship>,
    pub killed: Vec<usize>,
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
        fn padded_total_rad(system: &Vec<Planet>, pl_index: usize, rad: f32) -> f32 {
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
            let mut pl_rad = config.sun_rad;
            pl_rad *= prng.gen_range(config.pl_size_multiplier.clone());
            system.push(Planet::new(pl_rad));

            while { 
                total_rad(&system, pl_index) < system[pl_index].rad * 5f32 &&
                prng.gen_bool(config.pl_moon_prob) 
            } {
                let mult = prng.gen_range(config.pl_size_multiplier.clone());
                let moon_rad = pl_rad * mult;
                let moon_index = system.len();

                let dist = padded_total_rad(&system, pl_index, moon_rad);
                let moon_orbit = Orbit::new(pl_index, dist);

                system[pl_index].moon_indices.push(moon_index);
                system.push(Planet::new(moon_rad));
                system[moon_index].orbit = Some(moon_orbit);
            }

            // Total radius of the planet subsystem
            let pl_system_rad = padded_total_rad(&system, pl_index, pl_rad);

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

        fn rand_feature(prng: &mut StdRng) -> PlanetFeature {
            PlanetFeature::iter().choose(prng).unwrap()
        }

        // Must be at least 4 planets for the ships to have proper behavior
        // The sun, 2 planets with stations, 1 with ore
        if system.len() < 4 {
            panic!()
        }

        {
            fn new_station() -> PlanetFeature { 
                PlanetFeature::Station { stock: 0 } 
            }

            fn new_ore_feature() -> PlanetFeature { 
                PlanetFeature::Ore 
            }

            // Ensure that planets with essential features are present
            let last_pl_index = system.len() - 1;
            let rand_pl_index = prng.gen_range(2..system.len());
            system[1].feat = Some(new_station());
            system[last_pl_index].feat = Some(new_station());
            system[rand_pl_index].feat = Some(new_ore_feature());

            // Randomly add PlanetFeatures throughout the system
            for pl in system.iter_mut().skip(1) {
                if prng.gen_bool(config.pl_feat_prob) && pl.feat.is_none() {
                    pl.feat = Some(rand_feature(&mut prng));
                }
            }
        }

        // The ACTUAL radius of the system, in contrast to config.system_rad
        let system_rad = total_rad(&system, 0);

        let mut ships = Vec::new();
        for _ in 0..config.miner_count {
            let mut ship = Ship::new(ShipJob::Miner, config.ship_speed);
            // Use polar coordinates to ensure even distribution
            ship.pos = rand_pos(&mut prng, system_rad);

            // Add ship after updating position
            ships.push(ship);
        }

        {
            // Ships start at random points, with random destinations
            // Initial goals are specific to each ship's job
            let ores = ore_indices(&system);
            for ship in ships.iter_mut() {
                ship.goal = ShipGoal::Visit { 
                    target: *ores.iter().choose(&mut prng).unwrap()
                };
            }
        }

        // Generate a few pirate ships to steal from traders
        for _ in 0..config.pirate_count {
            let pirate_pos = rand_pos(&mut prng, system_rad * 0.5);
            let mut pirate = Ship::new(
                ShipJob::Pirate { origin: (pirate_pos.x, pirate_pos.y) }, 
                config.ship_speed);
            pirate.pos = pirate_pos;
            pirate.goal = ShipGoal::Wander; // pirates start by wandering

            // Move the pirate to a random spot within its territory
            let offset = rand_pos(&mut prng, config.pirate_territory);
            pirate.pos.x += offset.x;
            pirate.pos.y += offset.y;

            // Add pirate after giving it a random pos
            ships.push(pirate);
        }

        Self {
            prng,
            system,
            system_rad,
            ships,
            killed: Vec::new(),
            config
        }        
    }

    pub fn update(&mut self) {
        // Update positions of all planets
        self.update_planet_pos(0);

        // Spawn new ships from stations with sufficient stock
        for pl_index in 0..self.system.len() {
            if let Some(
                PlanetFeature::Station { ref mut stock } 
            ) = self.system[pl_index].feat {
                if *stock > self.config.ship_cost {
                    *stock -= self.config.ship_cost;
                    let mut ship = Ship::new(
                        ShipJob::Trader { cargo: false }, 
                        self.config.ship_speed);
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

        // Kill all trading ships that were destroyed in raids this update cycle
        for index in self.killed.drain(0..) {
            for ship in self.ships.iter_mut() {
                if let ShipGoal::Hunt { ref mut prey, .. } = ship.goal {
                    if *prey == index {
                        ship.goal = ShipGoal::Wander;
                    } else if *prey > index {
                        *prey -= 1;
                    }
                }
            }

            self.ships.remove(index);
        }
    }

    /// Updates the planet at given index, then recursively updates its moons
    /// If called on the sun (Self::system[0]), updates the whole system
    fn update_planet_pos(&mut self, pl_index: usize) {
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

            // Each orbit has its own speed multiplier
            offset *= pl_orbit.speed;

            { // Relative size affects angle offset
                let sun_rad = self.system[0].rad;
                let pl_rad = self.system[pl_orbit.parent_index].rad;
                offset *= sun_rad / pl_rad;
            }

            // Reverse if the orbit is counterclockwise
            offset *= if pl_orbit.ccw { -1f32 } else { 1f32 };

            let dist = {
                let mut temp_orbit = pl_orbit;
                temp_orbit.angle += offset;
                temp_orbit.angle %= TAU;
                dist_to_sun(parent_pos, temp_orbit) };
            if pl_orbit.parent_index == 0 {
                // The nearer a planet is, the FASTER it goes
                // Doesn't apply to moons
                offset *= (self.system_rad - dist).sqrt() / self.system_rad; 
            }

            // Update the current angle of the orbit
            pl_orbit.angle += offset;
            pl_orbit.angle %= TAU;

            // Update orbit and pos
            self.system[pl_index].pos = Point2::new(
                parent_pos.x + pl_orbit.dist * pl_orbit.angle.cos(),
                parent_pos.y + pl_orbit.dist * pl_orbit.angle.sin());
            self.system[pl_index].orbit = Some(pl_orbit);
        }

        // Update all of the current planet's moons
        let mut moon_indices = self.system[pl_index].moon_indices.clone();
        for moon_index in moon_indices.drain(0..) {
            self.update_planet_pos(moon_index);
        }
    }

    pub fn pirate_in_range(&self, pirate_index: usize) -> bool {
        if let ShipGoal::Hunt { prey, .. } = self.ships[pirate_index].goal {
            let pirate_pos = self.ships[pirate_index].pos;
            let prey_pos = self.ships[prey].pos;
            let dist = pirate_pos.distance(prey_pos);
            return dist < self.config.raid_range;
        }

        panic!()
        
    }

    /// Updates ship position and checks the status of its goal
    /// If the ship has achieved its goal, Self::update_ship_goal is called
    fn update_ship(&mut self, ship_index: usize) {     
        fn arrived(ship_pos: Point2<f32>, old_ship_pos: Point2<f32>, pl_pos: Point2<f32>, pl_rad: f32) -> bool {
            /* ship_pos.distance(pl_pos) <= pl_rad * 2f32 */
            let old_x = old_ship_pos.x - pl_pos.x;
            let old_y = old_ship_pos.y - pl_pos.y;
            let new_x = ship_pos.x - pl_pos.x;
            let new_y = ship_pos.y - pl_pos.y;

            let a = (new_x - old_x).powf(2f32) + (new_y - old_y).powf(2f32);
            let b = 2f32 * (old_x * (new_x - old_x) + old_y * (new_y - old_y));
            let c = old_x.powf(2f32) + old_y.powf(2f32) - pl_rad.powf(2f32);
            let disc = b.powf(2f32) - 4f32 * a * c;
            if disc <= 0f32 {
                return false;
            }

            let disc = disc.sqrt();
            let t1 = (b * -1f32 + disc) / (2f32 * a);
            let t2 = (b * -1f32 - disc) / (2f32 * a);
            if (0f32 < t1 && t1 < 1f32) || (0f32 < t2 && t2 < 1f32) {
                return true;
            }

            false
        }

        fn update_ship_pos(ship: &mut Ship, dest_pos: Point2<f32>) {
            // Position offsets
            let dx = dest_pos.x - ship.pos.x;
            let dy = dest_pos.y - ship.pos.y;

            // Update position, angle and increase speed
            ship.pos.x += dx * ship.speed;
            ship.pos.y += dy * ship.speed;
            ship.angle = Rad::atan2(dx, dy).0 + PI;
        }

        let mut ship_objective_complete = false;
        match self.ships[ship_index].goal {
            ShipGoal::Visit { target: pl_index } => {
                // Update ship objective IFF it has reached its destination
                let pl_pos = self.system[pl_index].pos;
                let pl_rad = self.system[pl_index].rad;

                let old_ship_pos = self.ships[ship_index].pos;
                // Update ship position and increase speed
                let mut ship = &mut self.ships[ship_index];
                update_ship_pos(ship, pl_pos);
                ship.speed *= self.config.ship_acceleration;

                if arrived(ship.pos, old_ship_pos, pl_pos, pl_rad) {
                    ship.speed = ship.initial_speed; // reset speed
                    ship_objective_complete = true;
                }
            },

            ShipGoal::Wait { target: pl_index, progress } => {
                // Ships dock on planets while waiting
                self.ships[ship_index].pos = self.system[pl_index].pos;
                self.ships[ship_index].goal = ShipGoal::Wait { 
                    target: pl_index, 
                    progress: progress + 1 
                };

                // Update ship objective if the ship is done mining
                if progress == self.config.harvest_duration as isize {
                    ship_objective_complete = true;
                }
            },

            ShipGoal::Wander => {
                let mut ship = &mut self.ships[ship_index];

                // Reverse direction upon reaching edge of territory
                if let ShipJob::Pirate { origin } = ship.job {
                    let dist = ship.pos.distance(origin.into());
                    if dist > self.config.pirate_territory {
                        update_ship_pos(ship, origin.into());
                    } else {
                        // Change heading slightly
                        let mut angle_offset = 0.0348f32;
                        if self.prng.gen_bool(0.5) { angle_offset *= -1.0; }

                        // Keep moving forward
                        ship.angle += angle_offset;
                        ship.pos.x += (ship.angle + 1.566).cos() * ship.speed;
                        ship.pos.y -= (ship.angle + 1.566).sin() * ship.speed;

                        // Always update, scan every other tick
                        ship_objective_complete = true;
                    }
                }
            },
            
            ShipGoal::Scan => {
                // Update no matter what after the scan cycle
                ship_objective_complete = true;
            },

            ShipGoal::Hunt { prey, progress } => {
                // Move towards the prey ship
                let prey_pos = self.ships[prey].pos;
                update_ship_pos(&mut self.ships[ship_index], prey_pos);

                // Check if the target is still a valid target for a raid
                let prey_dist = self.ships[ship_index].pos.distance(prey_pos);
                if let ShipJob::Trader { cargo } = self.ships[prey].job {
                    if !cargo { 
                        ship_objective_complete = true; 
                    } else if prey_dist < self.config.raid_range {
                        // Prevent target ship from accelerating
                        let initial_speed = self.ships[prey].initial_speed;
                        self.ships[prey].speed = initial_speed;
                        self.ships[ship_index].goal = ShipGoal::Hunt {
                            prey,
                            progress: progress + 1
                        };
    
                        // Raid is complete
                        if progress > self.config.raid_duration as isize {
                            if self.prng.gen_bool(self.config.death_prob) && !self.killed.contains(&prey) {
                                self.killed.push(prey);
                            }

                            ship_objective_complete = true;
                        }
                    } else {
                        // Reset goal if the ship escaped
                        self.ships[ship_index].goal = ShipGoal::Wander;
                    }
                } 
            }
        }

        if ship_objective_complete {
            self.update_ship_goal(ship_index)
        }
    }

    /// Assumes that the ship has achieved its previous goal
    fn update_ship_goal(&mut self, ship_index: usize) {
        // Returns a mutable reference to the `stock` field of a station
        // Panics if given planet doesn't have a station
        fn stock(pl: &mut Planet) -> &mut usize {
            if let Some(PlanetFeature::Station { ref mut stock } ) = pl.feat {
                return stock;
            }
        
            panic!()
        }
        
        // All ship logic occurs in this match expression
        let job = self.ships[ship_index].job;
        let goal = self.ships[ship_index].goal;
        self.ships[ship_index].goal = match (job, goal) {
            (
                ShipJob::Trader { cargo }, 
                ShipGoal::Visit { target } 
            ) => {
                // Deliver ore if the Trader was carrying them
                if cargo {
                    *stock(&mut self.system[target]) += 1;
                    self.ships[ship_index].job = ShipJob::Trader { 
                        cargo: false 
                    };
                }

                // Find the ship's new destination
                let dest;

                { // Randomly select it from all planets with stations
                    let mut stations = station_indices(&self.system);
                    stations.retain(|pl| *pl != target);
                    dest = *stations.iter().choose(&mut self.prng).unwrap();
                }
                
                #[allow(clippy::blocks_in_if_conditions)]
                if { // Determine if the ship should carry ore
                    let target_res = *stock(&mut self.system[target]);
                    let dest_res = *stock(&mut self.system[dest]);

                    // Should carry ore if destination has less
                    // AND if it didn't carry any to this station
                    target_res > dest_res && !cargo
                } {
                    // Take ore from station and give to ship
                    *stock(&mut self.system[target]) -= 1;
                    self.ships[ship_index].job = ShipJob::Trader { 
                        cargo: true 
                    };
                }                     
                
                ShipGoal::Visit { target: dest }
            },

            ( // After arriving at station or mining site
                ShipJob::Miner, 
                ShipGoal::Visit { target } 
            ) => {
                // Behavior depends on the type of planet is just visited
                match self.system[target].feat.as_ref().unwrap() {
                    PlanetFeature::Station { .. } => {
                        // Deposit ore at the station
                        *stock(&mut self.system[target]) += 1;

                        // Visit another planet with ore
                        let ores = nearest_with_feature(
                            &self.system, 
                            Some(PlanetFeature::Ore), 
                            self.ships[ship_index].pos);
                        ShipGoal::Visit { target: ores[0] }
                    },
                    PlanetFeature::Ore => {
                        // Pause to mine
                        let progress = self.config.harvest_variance.clone();
                        let progress = progress.choose(&mut self.prng);
                        let progress = progress.unwrap();
                        ShipGoal::Wait { target, progress }
                    }
                }
            },

            (
                ShipJob::Miner, 
                ShipGoal::Wait { .. } 
            ) => {
                // After mining, the ship needs to deposit
                let stations = nearest_with_feature(
                    &self.system, 
                    Some(PlanetFeature::Station { stock: 0 } ), 
                    self.ships[ship_index].pos);
                ShipGoal::Visit { target: stations[0] }
            },
            
            (
                ShipJob::Pirate { .. },
                ShipGoal::Wander { .. }
            ) => ShipGoal::Scan,

            (
                ShipJob::Pirate { .. },
                ShipGoal::Scan
            ) => {
                let mut prey_indices = Vec::new();

                let ship_count = self.ships.len();
                for target_index in 0..ship_count {
                    let target_job = self.ships[target_index].job;
                    if let ShipJob::Trader { cargo: true } = target_job {
                        let ship_pos = self.ships[ship_index].pos;
                        let target_ship_pos = self.ships[target_index].pos;
                        let dist = ship_pos.distance(target_ship_pos);
                        if dist < self.config.pirate_territory * 0.5 {
                            prey_indices.push(target_index);
                        }
                    }
                }

                let prey = prey_indices.iter().choose(&mut self.prng);
                match prey {
                    Some(prey_index) => { 
                        let progress = self.config.raid_variance.clone();
                        let progress = progress.choose(&mut self.prng);
                        let progress = progress.unwrap();
                        ShipGoal::Hunt { prey: *prey_index, progress } 
                    },
                    None => ShipGoal::Wander
                }
            },

            (
                ShipJob::Pirate { .. },
                ShipGoal::Hunt { prey, .. }
            ) => {
                let prey_job = &mut self.ships[prey].job;
                if let ShipJob::Trader { ref mut cargo } = prey_job {
                    *cargo = false;
                }

                ShipGoal::Wander
            },

            _ => self.ships[ship_index].goal
        };
    }
}

fn rand_pos(prng: &mut StdRng, rad: f32) -> Point2<f32> {
    let r = rad * prng.gen::<f32>().sqrt();
    let theta = prng.gen::<f32>() * TAU;
    
    Point2::new(r * theta.cos(), r * theta.sin())
}

fn station_indices(system: &[Planet]) -> Vec<usize> {
    filter_system(system, Some(PlanetFeature::Station { stock: 0 }))
}

fn ore_indices(system: &[Planet]) -> Vec<usize> {
    filter_system(system, Some(PlanetFeature::Ore))
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