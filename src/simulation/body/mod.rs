mod orbit;

use cgmath::Point2;

use super::Simulation;

#[derive(Clone)]
pub(crate) struct Body {
    pos: Point2<f32>,
    radius: f32,
    orbit: Option<orbit::Orbit>,
    moon_indices: Vec<usize>,
    feature: Option<BodyFeature>
}

impl Body {
    pub(crate) fn pos(&self) -> Point2<f32> {
        self.pos
    }

    pub(crate) fn radius(&self) -> f32 {
        self.radius
    }

    pub(crate) fn moon_indices(&self) -> impl Iterator<Item = &usize> {
        self.moon_indices.iter()
    }

    pub(crate) fn feature(&self) -> Option<BodyFeature> {
        self.feature.as_ref().cloned()
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
            pos: Point2::new(0f32, 0f32),
            radius: Self::SUN_RADIUS, 
            orbit: None, 
            moon_indices: Vec::new(),
            feature: None
        }
    }
}

impl std::hash::Hash for Body {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.radius.to_string().hash(state);
        self.orbit.hash(state);
        self.moon_indices().copied().collect::<Vec<usize>>().hash(state);
    }
}

impl Body {
    pub(crate) const SUN_RADIUS: f32 = 0.04;

    pub(crate) fn new(radius: f32) -> Self {
        Self {
            pos: Point2::new(0f32, 0f32),
            radius,
            orbit: None,
            moon_indices: Vec::new(),
            feature: None
        }
    }

    pub(crate) fn add_moon(&mut self, index: usize) {
        self.moon_indices.push(index);
    }

    pub(crate) fn add_orbit(&mut self, parent: usize, distance: f32) {
        self.orbit = Some(orbit::Orbit::new(parent, distance));
    }

    pub(crate) fn add_feature(&mut self, feature: BodyFeature) {
        self.feature = Some(feature);
    }

    pub(crate) fn get_orbital_radius(&self, simulation: &Simulation) -> f32 {
        let mut r = self.radius * 2f32;

        for &moon_index in self.moon_indices() {
            let moon = &simulation.bodies[moon_index];

            let o = moon.orbit.unwrap();
            let o = o.distance() + moon.get_orbital_radius(simulation);

            r = r.max(o);
        }

        r
    }

    pub(crate) fn update_pos(&mut self, parent_pos: Point2<f32>, parent_radius: f32) {
        if let Some(mut orbit) = self.orbit {
            // Larger bodies move slower
            let multiplier = Self::SUN_RADIUS / parent_radius;

            self.pos = parent_pos;
            self.pos = orbit.update(self.pos, multiplier);
            
            self.orbit = Some(orbit);
        } 
    }
}

#[derive(Clone)]
pub(crate) enum BodyFeature {
    Station
}