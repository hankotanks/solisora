mod orbit;

use crate::prelude::Point;

use super::Simulation;

#[derive(Clone)]
pub(crate) struct Body {
    pos: Point,
    radius: f32,
    orbit: Option<orbit::Orbit>,
    moon_indices: Vec<usize>
}

impl Body {
    pub(crate) fn pos(&self) -> Point {
        self.pos
    }

    pub(crate) fn radius(&self) -> f32 {
        self.radius
    }

    pub(crate) fn moon_indices(&self) -> impl Iterator<Item = &usize> {
        self.moon_indices.iter()
    }

    pub(crate) fn parent(&self) -> Option<usize> {
        if self.orbit.is_some() {
            return Some(self.orbit.unwrap().parent());
        }

        None
    }
}

impl Default for Body {
    fn default() -> Self {
        Self { 
            pos: Point::default(),
            radius: Self::SUN_RADIUS, 
            orbit: None, 
            moon_indices: Vec::new() 
        }
    }
}

impl Body {
    const SUN_RADIUS: f32 = 0.075;

    pub(crate) fn new(radius: f32) -> Self {
        Self {
            pos: Point::default(),
            radius,
            orbit: None,
            moon_indices: Vec::new()
        }
    }

    pub(crate) fn add_moon(&mut self, index: usize) {
        self.moon_indices.push(index);
    }

    pub(crate) fn add_orbit(&mut self, parent: usize, distance: f32) {
        self.orbit = Some(orbit::Orbit::new(parent, distance));
    }

    pub(crate) fn get_orbital_radius(&self, simulation: &Simulation) -> f32 {
        let mut r = self.radius * 2f32;

        for &moon_index in self.moon_indices() {
            let moon = &simulation.bodies[moon_index];

            let o = moon.orbit.unwrap();
            let o = o.distance() + moon.get_orbital_radius(&simulation);

            r = r.max(o);
        }

        r
    }

    pub(crate) fn update_pos(&mut self, parent_pos: Point, parent_radius: f32) {
        if let Some(mut orbit) = self.orbit {
            let multiplier = Self::SUN_RADIUS / parent_radius;

            self.pos = parent_pos;
            self.pos = orbit.update(self.pos, multiplier);
            
            self.orbit = Some(orbit);
        } 
    }
}