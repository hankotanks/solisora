pub(crate) mod body;
pub(crate) mod ship;

use rand::Rng;

pub(crate) struct Simulation {
    bodies: Vec<body::Body>,
    ships: Vec<ship::Ship>
}

impl Default for Simulation {
    fn default() -> Self {
        let mut simulation = Self { 
            bodies: vec![body::Body::default()],
            ships: Vec::new()
        };
        
        loop {
            let planet_index = simulation.add();
            let planet_radius = simulation.bodies[planet_index].get_orbital_radius(&simulation);

            let system_radius = simulation.bodies[0].get_orbital_radius(&simulation);
            if system_radius + planet_radius > 1f32 {
                simulation.bodies.truncate(planet_index);
                break;
            }

            simulation.bodies[0].add_moon(planet_index);
            simulation.bodies[planet_index].add_orbit(0, system_radius + planet_radius);
        }

        // Update once to put bodies in position
        simulation.update();

        for _ in 0..50 {
            simulation.ships.push(
                ship::Ship::new(&simulation)
            );
        }

        simulation
    }
}

impl Simulation {
    // Adds a new planet system (without an orbit) and returns the index of the planet
    fn add(&mut self) -> usize {
        let planet_radius = self.bodies[0].radius();
        let planet_radius = rand::thread_rng().gen_range(
            (planet_radius * 0.1f32)..(planet_radius * 0.5f32)
        );

        let planet_index = self.bodies.len();

        self.bodies.push(body::Body::new(planet_radius));

        while rand::thread_rng().gen_bool(0.5f64) {
            let moon_radius = (planet_radius * 0.1f32)..(planet_radius * 0.5f32);
            let moon_radius = rand::thread_rng().gen_range(moon_radius).max(body::Body::SUN_RADIUS * 0.05f32);

            let distance = self.bodies[planet_index].get_orbital_radius(self);
            let distance = distance + moon_radius * 3f32;

            let moon_index = self.bodies.len();
            self.bodies[planet_index].add_moon(moon_index);
            self.bodies.push(body::Body::new(moon_radius));
            self.bodies[moon_index].add_orbit(planet_index, distance);
        }

        let body_with_station = planet_index..self.bodies.len();
        let body_with_station = rand::thread_rng().gen_range(body_with_station);
        self.bodies[body_with_station].add_feature(
            body::BodyFeature::Station
        );

        planet_index
    }

    pub(crate) fn update(&mut self) {
        self.update_body(0);

        for entity_index in (0..self.ships.len()).rev() {
            let mut entity = self.ships.remove(entity_index);
            entity.update(&self);

            self.ships.push(entity);
        }
    }

    fn update_body(&mut self, index: usize) {
        if let Some(parent_index) = self.bodies[index].parent() {
            let parent_pos = self.bodies[parent_index].pos();
            let parent_radius = self.bodies[parent_index].radius();

            self.bodies[index].update_pos(parent_pos, parent_radius);
        }

        let moon_indices = self.bodies[index].moon_indices().copied().collect::<Vec<usize>>();
        for moon_index in moon_indices {
            self.update_body(moon_index);
        }
    }
    
}

impl Simulation {
    pub(crate) fn bodies(&self) -> impl Iterator<Item = &body::Body> {
        self.bodies.iter()
    }

    pub(crate) fn ships(&self) -> impl Iterator<Item = &ship::Ship> {
        self.ships.iter()
    }

    pub(crate) fn bodies_with_stations(&self) -> Vec<usize> {
        let mut indices = Vec::new();
        self.bodies.iter().enumerate().for_each(|(index, b)| { 
            if matches!(b.feature(), Some(body::BodyFeature::Station)) {
                indices.push(index);
            }
        } );

        indices
    }
}