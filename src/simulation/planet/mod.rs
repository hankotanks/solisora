mod orbit;

use cgmath::Point2;

use super::Simulation;

#[derive(Clone)]
pub(crate) struct Planet {
    index: usize,
    pos: Point2<f32>,
    radius: f32,
    orbit: Option<orbit::Orbit>,
    moon_indices: Vec<usize>,
    feature: Option<PlanetaryFeature>
}

impl Planet {
    pub(crate) fn index(&self) -> usize {
        self.index
    }
    pub(crate) fn pos(&self) -> Point2<f32> {
        self.pos
    }

    pub(crate) fn radius(&self) -> f32 {
        self.radius
    }

    pub(crate) fn moon_indices(&self) -> impl Iterator<Item = &usize> {
        self.moon_indices.iter()
    }

    pub(crate) fn feature(&self) -> Option<PlanetaryFeature> {
        self.feature.as_ref().cloned()
    }

    pub(crate) fn set_feature(&mut self, f: PlanetaryFeature) {
        self.feature = Some(f);
    }

    pub(crate) fn parent(&self) -> Option<usize> {
        if self.orbit.is_some() {
            return Some(self.orbit.unwrap().parent());
        }

        None
    }
}

impl Default for Planet {
    fn default() -> Self {
        Self { 
            index: 0usize,
            pos: Point2::new(0f32, 0f32),
            radius: Self::SUN_RADIUS, 
            orbit: None, 
            moon_indices: Vec::new(),
            feature: None
        }
    }
}

impl std::hash::Hash for Planet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.radius.to_string().hash(state);
        self.orbit.hash(state);
        self.moon_indices().copied().collect::<Vec<usize>>().hash(state);
    }
}

impl Planet {
    pub(crate) const SUN_RADIUS: f32 = 0.04;

    pub(crate) fn new(index: usize, radius: f32) -> Self {
        Self {
            index,
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

    pub(crate) fn add_feature(&mut self, feature: PlanetaryFeature) {
        self.feature = Some(feature);
    }

    pub(crate) fn get_orbital_radius(&self, simulation: &Simulation) -> f32 {
        let mut r = self.radius * 2f32;

        for &moon_index in self.moon_indices() {
            let moon = &simulation.planets[moon_index];

            let o = moon.orbit.unwrap();
            let o = o.distance() + moon.get_orbital_radius(simulation);

            r = r.max(o);
        }

        r
    }

    pub(crate) fn update_pos(&mut self, parent_pos: Point2<f32>, parent_radius: f32) {
        if let Some(mut orbit) = self.orbit {
            // Larger planets move slower
            let multiplier = Self::SUN_RADIUS / parent_radius;

            self.pos = parent_pos;
            self.pos = orbit.update(self.pos, multiplier);
            
            self.orbit = Some(orbit);
        } 
    }
}

#[derive(Clone)]
pub(crate) enum PlanetaryFeature {
    Station(usize),
    Resources
}