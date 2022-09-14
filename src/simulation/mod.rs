pub(crate) mod planet;
pub(crate) mod ship;

use std::mem::Discriminant;

use cgmath::MetricSpace;
use rand::Rng;

use self::planet::PlanetaryFeature;

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

        // TODO: Not all ships are rendered...
        for _ in 0..100 {
            let ship = ship::Ship::new(&mut simulation);
            simulation.ships.push(ship);
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

        self.planets.push(planet::Planet::new(planet_index, planet_radius));

        while rand::thread_rng().gen_bool(0.5f64) {
            let moon_radius = (planet_radius * 0.1f32)..(planet_radius * 0.5f32);
            let moon_radius = rand::thread_rng().gen_range(moon_radius).max(planet::Planet::SUN_RADIUS * 0.05f32);

            let distance = self.planets[planet_index].get_orbital_radius(self);
            let distance = distance + moon_radius * 3f32;

            let moon_index = self.planets.len();
            self.planets[planet_index].add_moon(moon_index);
            self.planets.push(planet::Planet::new(moon_index, moon_radius));
            self.planets[moon_index].add_orbit(planet_index, distance);
        }

        let planet_with_station = planet_index..self.planets.len();
        let planet_with_station = rand::thread_rng().gen_range(planet_with_station);
        self.planets[planet_with_station].add_feature(
            planet::PlanetaryFeature::Station(0usize)
        );

        let planet_with_resources = planet_index..self.planets.len();
        let planet_with_resources = rand::thread_rng().gen_range(planet_with_resources);
        if self.planets[planet_with_resources].feature().is_none() {
            self.planets[planet_with_resources].add_feature(
                planet::PlanetaryFeature::Resources
            );
        }

        planet_index
    }

    pub(crate) fn update(&mut self) {
        self.update_planet_position(0);

        // update stations
        for planet in 0..self.planets.len() {
            if let Some(PlanetaryFeature::Station(resources)) = self.planets[planet].feature() {
                if resources > 10 {
                    self.planets[planet].add_feature(PlanetaryFeature::Station(0usize));
                    let mut ship = ship::Ship::with_behavior(self, ship::ShipBehavior::Trader);
                    ship.set_pos(self.planets[planet].pos());
                    self.ships.push(
                        ship
                    );
                }
            }
        }

        // update ships
        for ship in (0..self.ships.len()).rev() {
            let mut ship  = self.ships.remove(ship);
            ship.update(self);

            self.ships.push(ship);
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

    pub(crate) fn planets_with_feature(&self, filter: Option<Discriminant<PlanetaryFeature>>) -> impl Iterator<Item = &planet::Planet> {
        self.planets.iter().filter(move |planet| {
            match planet.feature() {
                Some(feature) => {
                    match filter {
                        Some(d) => std::mem::discriminant(&feature) == d,
                        None => false
                    }
                },
                None => filter.is_none()
            }
        } )
    }

    pub(crate) fn closest_planet_with_feature(&self, pos: cgmath::Point2<f32>, filter: Option<Discriminant<PlanetaryFeature>>) -> usize {
        let mut closest = 0;
        let mut closest_distance = std::f32::MAX;
        for station in self.planets_with_feature(filter) {
            let distance = pos.distance2(station.pos());
            if distance < closest_distance {
                closest = station.index();
                closest_distance = distance;
            }
        }

        closest
    }
}