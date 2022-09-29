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
    ship_acceleration: f32,
    ship_resource_cost: usize,
    ship_scan_range: f32,
    ship_theft_range: f32

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
            ship_acceleration: 1.05,
            ship_resource_cost: 4,
            ship_scan_range: 0.3,
            ship_theft_range: 0.02
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
        // SUN -- 2 w/ STATIONS -- 1 w/ RESOURCES
        if system.len() < 4 {
            panic!()
        }

        {
            fn new_station() -> PlanetFeature { 
                PlanetFeature::Station { num_resources: 0 } 
            }

            fn new_resources() -> PlanetFeature { 
                PlanetFeature::Resources 
            }

            // Ensure that planets with essential features are present
            let last_pl_index = system.len() - 1;
            let rand_pl_index = prng.gen_range(2..system.len());
            system[1].feat = Some(new_station());
            system[last_pl_index].feat = Some(new_station());
            system[rand_pl_index].feat = Some(new_resources());

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
        for _ in 0..config.ship_count {
            let mut ship = Ship::new(ShipJob::Miner, config.ship_speed);
            // Use polar coordinates to ensure even distribution
            ship.pos = rand_pos(&mut prng, system_rad);

            // Add ship after updating position
            ships.push(ship);
        }

        {
            // Ships start at random points, with random destinations
            // Initial goals are specific to each ship's job
            let resources = resource_indices(&system);
            for ship in ships.iter_mut() {
                ship.goal = ShipGoal::Visit { 
                    target: *resources.iter().choose(&mut prng).unwrap()
                };
            }
        }

        // TODO
        for _ in 0..5 {
            ships.push( {
                let mut pirate = Ship::new(ShipJob::Pirate, config.ship_speed);
                pirate.pos = rand_pos(&mut prng, system_rad);
                pirate.goal = ShipGoal::Search;
    
                pirate
            } );
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
            if let Some(
                PlanetFeature::Station { ref mut num_resources } 
            ) = self.system[pl_index].feat {
                if *num_resources > self.config.ship_resource_cost {
                    *num_resources -= self.config.ship_resource_cost;
                    let mut ship = Ship::new(
                        ShipJob::Trader { has_resource: false }, 
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
    }

    /// Updates the planet at given index, then recursively updates its moons
    /// If called on the sun (Self::system[0]), updates the whole system
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

    /// Updates ship position and checks the status of its goal
    /// If the ship has achieved its goal, Self::update_ship_goal is called
    pub fn update_ship(&mut self, ship_index: usize) {     
        fn arrived(ship_pos: Point2<f32>, pl_pos: Point2<f32>, pl_rad: f32) -> bool {
            ship_pos.distance2(pl_pos) <= pl_rad.powf(2f32)
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
        let mut ship_goal = self.ships[ship_index].goal;
        match ship_goal {
            ShipGoal::Visit { target: pl_index } => {
                // Update ship objective IFF it has reached its destination
                let pl_pos = self.system[pl_index].pos;
                let pl_rad = self.system[pl_index].rad;

                let mut ship = &mut self.ships[ship_index];
                if arrived(ship.pos, pl_pos, pl_rad) {
                    ship.speed = ship.initial_speed; // reset speed
                    ship_objective_complete = true;
                }

                update_ship_pos(&mut ship, pl_pos);
                ship.speed *= self.config.ship_acceleration;
            },

            ShipGoal::Wait { target: pl_index, progress } => {
                // Ships dock on planets while waiting
                self.ships[ship_index].pos = self.system[pl_index].pos;

                // Update ship objective if the ship is done mining
                if progress < self.config.ship_mine_progress {
                    ship_goal /* TODO */ = ShipGoal::Wait { 
                        target: pl_index, 
                        progress: progress + 1 
                    };
                } else {
                    ship_objective_complete = true;
                }
            },

            ShipGoal::Search => {
                // Move towards a random point
                let mut ship = &mut self.ships[ship_index];
                ship.angle += 0.0174f32 * if self.prng.gen_bool(0.5) { 4f32 } else { -4f32 };
                ship.pos.x += (ship.angle + 90f32 * 0.0174f32).cos() * ship.speed;
                ship.pos.y -= (ship.angle + 90f32 * 0.0174f32).sin() * ship.speed;

                if ship.pos.distance((0f32, 0f32).into()) > self.system_rad {
                    ship.angle += 0.0174f32 * 180f32;
                }
                
                ship_objective_complete = true;
            },
            
            ShipGoal::Scan { ref mut prey } => {
                let mut prey_indices = Vec::new();

                let ship_count = self.ships.len();
                for target_ship_index in 0..ship_count {
                    let target_ship_job = self.ships[target_ship_index].job;
                    if let ShipJob::Trader { has_resource: true } = target_ship_job {
                        let ship_pos = self.ships[ship_index].pos;
                        let target_ship_pos = self.ships[target_ship_index].pos;
                        let dist = ship_pos.distance(target_ship_pos);
                        if dist < self.config.ship_scan_range {
                            prey_indices.push(target_ship_index);
                        }
                    }
                }

                if let Some(prey_index) = prey_indices.iter().choose(&mut self.prng) {
                    *prey = Some(*prey_index);
                } 

                // Update no matter what after the scan cycle
                ship_objective_complete = true;
            },

            ShipGoal::Hunt { prey } => {
                self.ships[prey].speed = self.ships[prey].initial_speed;

                let prey_pos = self.ships[prey].pos;
                let mut ship = &mut self.ships[ship_index];
                update_ship_pos(ship, prey_pos);
                ship.speed *= self.config.ship_acceleration;

                let dest_rad = self.config.ship_theft_range;
                if arrived(ship.pos, prey_pos, dest_rad) {
                    ship_objective_complete = true;
                    ship.speed = ship.initial_speed;
                }
            }
        }

        self.ships[ship_index].goal = ship_goal; // TODO
        if ship_objective_complete {
            self.update_ship_goal(ship_index)
        }
    }

    /// Assumes that the ship has achieved its previous goal
    fn update_ship_goal(&mut self, ship_index: usize) {
        // Returns a mutable reference to the num_resources field of a station
        // Panics if given planet doesn't have a station
        fn num_resources(pl: &mut Planet) -> &mut usize {
            if let Some(PlanetFeature::Station { ref mut num_resources } ) = pl.feat {
                return num_resources;
            }
        
            panic!()
        }

        fn clamp_to_radius(pos: &mut Point2<f32>, rad: f32) {
            pos.x = pos.x.clamp(-1f32 * rad, rad);
            pos.y = pos.y.clamp(-1f32 * rad, rad);
        }
        
        // All ship logic occurs in this match expression
        let job = self.ships[ship_index].job;
        let goal = self.ships[ship_index].goal;
        self.ships[ship_index].goal = match (job, goal) {
            (
                ShipJob::Trader { has_resource }, 
                ShipGoal::Visit { target } 
            ) => {
                // Deliver resources if the Trader was carrying them
                if has_resource {
                    *num_resources(&mut self.system[target]) += 1;
                    self.ships[ship_index].job = ShipJob::Trader { 
                        has_resource: false 
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
                if { // Determine if the ship should carry resources
                    let target_res = *num_resources(&mut self.system[target]);
                    let dest_res = *num_resources(&mut self.system[dest]);

                    // Should carry resources if destination has less
                    // AND if it didn't carry any to this station
                    target_res > dest_res && !has_resource 
                } {
                    // Take resource from station and give to ship
                    *num_resources(&mut self.system[target]) -= 1;
                    self.ships[ship_index].job = ShipJob::Trader { 
                        has_resource: true 
                    };
                }                     
                
                ShipGoal::Visit { target: dest }
            },

            ( // After arriving at station or resources
                ShipJob::Miner, 
                ShipGoal::Visit { target } 
            ) => {
                // Behavior depends on the type of planet is just visited
                match self.system[target].feat.as_ref().unwrap() {
                    PlanetFeature::Station { .. } => {
                        // Deposit mined resources at the station
                        *num_resources(&mut self.system[target]) += 1;

                        // Visit a new planet with resources
                        let resources = nearest_with_feature(
                            &self.system, 
                            Some(PlanetFeature::Resources), 
                            self.ships[ship_index].pos);
                        ShipGoal::Visit { target: resources[0] }
                    },
                    PlanetFeature::Resources => {
                        // Pause to mine resources
                        ShipGoal::Wait { target, progress: 0 }
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
                    Some(PlanetFeature::Station { num_resources: 0 } ), 
                    self.ships[ship_index].pos);
                ShipGoal::Visit { target: stations[0] }
            },
            
            (
                ShipJob::Pirate,
                ShipGoal::Search { .. }
            ) => {
                ShipGoal::Scan { prey: None }
            },

            (
                ShipJob::Pirate,
                ShipGoal::Scan { prey }
            ) => {
                match prey {
                    Some(prey_index) => ShipGoal::Hunt { prey: prey_index },
                    None => ShipGoal::Search
                }
            },

            (
                ShipJob::Pirate,
                ShipGoal::Hunt { prey }
            ) => {
                if let ShipJob::Trader { ref mut has_resource } = self.ships[prey].job {
                    *has_resource = false;
                }

                ShipGoal::Search
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
    filter_system(system, Some(PlanetFeature::Station { num_resources: 0 }))
}

fn resource_indices(system: &[Planet]) -> Vec<usize> {
    filter_system(system, Some(PlanetFeature::Resources))
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