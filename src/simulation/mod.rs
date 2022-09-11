pub(crate) mod planet;
pub(crate) mod ship;

use rand::Rng;

pub(crate) struct Simulation {
    planets: Vec<planet::Planet>,
    ships: Vec<ship::Ship>
}

impl Default for Simulation {
    fn default() -> Self {
        let mut simulation = Self { 
            planets: vec![planet::Planet::default()],
            ships: Vec::new()
        };
        
        loop {
            let planet_index = simulation.add();
            let planet_radius = simulation.planets[planet_index].get_orbital_radius(&simulation);

            let system_radius = simulation.planets[0].get_orbital_radius(&simulation);
            if system_radius + planet_radius > 1f32 {
                simulation.planets.truncate(planet_index);
                break;
            }

            simulation.planets[0].add_moon(planet_index);
            simulation.planets[planet_index].add_orbit(0, system_radius + planet_radius);
        }

        // Update once to put planets in position
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
        let planet_radius = self.planets[0].radius();
        let planet_radius = rand::thread_rng().gen_range(
            (planet_radius * 0.1f32)..(planet_radius * 0.5f32)
        );

        let planet_index = self.planets.len();

        self.planets.push(planet::Planet::new(planet_radius));

        while rand::thread_rng().gen_bool(0.5f64) {
            let moon_radius = (planet_radius * 0.1f32)..(planet_radius * 0.5f32);
            let moon_radius = rand::thread_rng().gen_range(moon_radius).max(planet::Planet::SUN_RADIUS * 0.05f32);

            let distance = self.planets[planet_index].get_orbital_radius(self);
            let distance = distance + moon_radius * 3f32;

            let moon_index = self.planets.len();
            self.planets[planet_index].add_moon(moon_index);
            self.planets.push(planet::Planet::new(moon_radius));
            self.planets[moon_index].add_orbit(planet_index, distance);
        }

        let planet_with_station = planet_index..self.planets.len();
        let planet_with_station = rand::thread_rng().gen_range(planet_with_station);
        self.planets[planet_with_station].add_feature(
            planet::PlanetaryFeature::Station
        );

        planet_index
    }

    pub(crate) fn update(&mut self) {
        self.update_planet_position(0);

        for entity_index in (0..self.ships.len()).rev() {
            let mut entity = self.ships.remove(entity_index);
            entity.update(&self);

            self.ships.push(entity);
        }
    }

    fn update_planet_position(&mut self, index: usize) {
        if let Some(parent_index) = self.planets[index].parent() {
            let parent_pos = self.planets[parent_index].pos();
            let parent_radius = self.planets[parent_index].radius();

            self.planets[index].update_pos(parent_pos, parent_radius);
        }

        let moon_indices = self.planets[index].moon_indices().copied().collect::<Vec<usize>>();
        for moon_index in moon_indices {
            self.update_planet_position(moon_index);
        }
    }
    
}

impl Simulation {
    pub(crate) fn planets(&self) -> impl Iterator<Item = &planet::Planet> {
        self.planets.iter()
    }

    pub(crate) fn ships(&self) -> impl Iterator<Item = &ship::Ship> {
        self.ships.iter()
    }

    pub(crate) fn planets_with_stations(&self) -> Vec<usize> {
        let mut indices = Vec::new();
        self.planets.iter().enumerate().for_each(|(index, b)| { 
            if matches!(b.feature(), Some(planet::PlanetaryFeature::Station)) {
                indices.push(index);
            }
        } );

        indices
    }
}