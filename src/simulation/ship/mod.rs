use cgmath::{MetricSpace, Rad, Angle};
use rand::seq::IteratorRandom;
use rand::Rng;
use strum::IntoEnumIterator;

use crate::simulation::Simulation;

use super::planet::{self, PlanetaryFeature};

#[derive(Clone)]
pub(crate) struct Ship {
    goal: Option<ShipGoal>,
    behavior: ShipBehavior,
    pos: cgmath::Point2<f32>,
    speed: f32,
    initial_speed: f32,
    angle: f32
}

impl Ship {
    pub(crate) fn pos(&self) -> cgmath::Point2<f32> {
        self.pos
    }

    pub(crate) fn angle(&self) -> f32 {
        self.angle
    }

    pub(crate) fn behavior(&self) -> ShipBehavior {
        self.behavior
    }
}

impl Ship {
    pub(crate) fn new(simulation: &Simulation) -> Self {
        let speed = rand::thread_rng().gen::<f32>() * 0.01f32;
        let mut ship = Self {
            goal: None,
            behavior: ShipBehavior::iter().choose(&mut rand::thread_rng()).unwrap(),
            pos: {
                simulation.planets_with_feature(
                    Some(std::mem::discriminant(&planet::PlanetaryFeature::Station))
                ).choose(&mut rand::thread_rng()).unwrap().pos()
            },
            speed,
            initial_speed: speed,
            angle: 0f32
        };

        ship.set_objective(simulation);
        ship
    }
}

impl Ship {
    fn set_objective(&mut self, simulation: &Simulation) {
        self.goal = match self.behavior {
            ShipBehavior::Miner => {
                match self.goal {
                    Some(ShipGoal::VisitPlanet(index)) => {
                        match simulation.planets[index].feature() {
                            Some(PlanetaryFeature::Station) => {
                                // find nearest resources
                                Some(ShipGoal::VisitPlanet(
                                    simulation.closest_planet_with_feature(self.pos, Some(std::mem::discriminant(&PlanetaryFeature::Resources)))
                                ))
                            },
                            Some(PlanetaryFeature::Resources) => {
                                // wait at asteroid
                                Some(ShipGoal::Wait(index, 100))
                            },
                            _ => panic!()
                        }
                    },
                    Some(ShipGoal::Wait(..)) => {
                        Some(ShipGoal::VisitPlanet(
                            simulation.closest_planet_with_feature(self.pos, Some(std::mem::discriminant(&PlanetaryFeature::Station)))
                        ))
                    },
                    None => {
                        Some(ShipGoal::VisitPlanet(
                            simulation.closest_planet_with_feature(self.pos, Some(std::mem::discriminant(&PlanetaryFeature::Resources)))
                        ))
                    }
                }                
            },
            ShipBehavior::Trader => {
                Some(ShipGoal::VisitPlanet( {
                    simulation.planets_with_feature(
                        Some(std::mem::discriminant(&planet::PlanetaryFeature::Station))
                    ).choose(&mut rand::thread_rng()).unwrap().index()
                } ))
            },
            ShipBehavior::Pirate => {
                None
            }
        };
    }

    pub(crate) fn update(&mut self, simulation: &Simulation) {
        if let Some(goal) = &self.goal {
            match goal {
                ShipGoal::VisitPlanet(index) => {
                    let goal_pos = simulation.planets[*index].pos();
    
                    let distance = self.pos.distance2(goal_pos);
                    if distance <= simulation.planets[*index].radius().powf(2f32) {
                        self.set_objective(simulation);

                        self.speed = self.initial_speed;
                    }

                    let dx = goal_pos.x - self.pos.x;
                    let dy = goal_pos.y - self.pos.y;

                    self.pos.x += dx * self.speed;
                    self.pos.y += dy * self.speed;

                    self.angle = Rad::atan2(dx, dy).0 + 3.14;

                    self.speed *= 1.05f32;
                },
                ShipGoal::Wait(index, counter) => {
                    self.pos = simulation.planets[*index].pos();

                    if counter > &0usize {
                        self.goal = Some(ShipGoal::Wait(*index, counter - 1usize));
                    } else {
                        self.set_objective(simulation);
                    }
                }
            }
        }
    }
}

#[derive(Copy, Clone)]
enum ShipGoal {
    VisitPlanet(usize),
    Wait(usize, usize)
}

#[derive(Copy, Clone, strum_macros::EnumIter)]
pub(crate) enum ShipBehavior {
    Miner,
    Trader,
    Pirate
}