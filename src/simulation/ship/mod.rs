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

    pub(crate) fn set_pos(&mut self, p: cgmath::Point2<f32>) {
        self.pos = p;
    }

    pub(crate) fn angle(&self) -> f32 {
        self.angle
    }

    pub(crate) fn behavior(&self) -> ShipBehavior {
        self.behavior
    }
}

impl Ship {
    pub(crate) fn new(simulation: &mut Simulation) -> Self {
        Self::with_behavior(
            simulation, 
            ShipBehavior::iter().choose(&mut rand::thread_rng()).unwrap()
        )
    }

    pub(crate) fn with_behavior(simulation: &mut Simulation, behavior: ShipBehavior) -> Self {
        let speed = rand::thread_rng().gen::<f32>() * 0.01f32;
        let mut ship = Self {
            goal: None,
            behavior,
            pos: {
                match simulation.planets_with_feature(
                    Some(std::mem::discriminant(&planet::PlanetaryFeature::Station(0usize)))
                ).choose(&mut rand::thread_rng()) {
                    Some(planet) => {
                        planet.pos()
                    },
                    None => {
                        cgmath::Point2::new(
                            rand::thread_rng().gen::<f32>() * 2f32 - 1f32,
                            rand::thread_rng().gen::<f32>() * 2f32 - 1f32
                        )
                    }
                }
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
    fn set_objective(&mut self, simulation: &mut Simulation) {
        self.goal = match self.behavior {
            ShipBehavior::Miner => {
                match self.goal {
                    Some(ShipGoal::VisitPlanet(index)) => {
                        match simulation.planets[index].feature() {
                            Some(PlanetaryFeature::Station(resources)) => {
                                simulation.planets[index].set_feature(PlanetaryFeature::Station(resources + 1));
                                // find nearest resources
                                match simulation.closest_planet_with_feature(self.pos, Some(std::mem::discriminant(&PlanetaryFeature::Resources))) {
                                    Some(index) => Some(ShipGoal::VisitPlanet(index)),
                                    None => None
                                }
                            },
                            Some(PlanetaryFeature::Resources) => {
                                Some(ShipGoal::Wait(index, 100))
                            },
                            _ => panic!()
                        }
                    },
                    Some(ShipGoal::Wait(..)) => {
                        match simulation.closest_planet_with_feature(self.pos, Some(std::mem::discriminant(&PlanetaryFeature::Station(0usize)))) {
                            Some(index) => Some(ShipGoal::VisitPlanet(index)),
                            None => None
                        }
                    },
                    None => {
                        match simulation.closest_planet_with_feature(self.pos, Some(std::mem::discriminant(&PlanetaryFeature::Resources))) {
                            Some(index) => Some(ShipGoal::VisitPlanet(index)),
                            None => None
                        }
                    },
                    _ => panic!()
                }                
            },
            ShipBehavior::Trader => {
                let mut current_resources = 0usize;
                if let Some(ShipGoal::VisitPlanet(current_planet)) | Some(ShipGoal::VisitPlanetWithResources(current_planet)) = self.goal {
                    if let Some(PlanetaryFeature::Station(resources)) = simulation.planets[current_planet].feature() {
                        current_resources = resources;
                    }
                }

                 match self.goal {
                    Some(ShipGoal::VisitPlanet(current_planet)) => {
                        match simulation.planets_with_feature(
                            Some(std::mem::discriminant(&planet::PlanetaryFeature::Station(0usize)))
                        ).filter(|planet| {
                            if let Some(PlanetaryFeature::Station(r)) = planet.feature() {
                                return r < current_resources;
                            }
                            
                            panic!()
                        } ).choose(&mut rand::thread_rng()) {
                            Some(target_planet) => {
                                let target_index = target_planet.index();

                                simulation.planets[current_planet].set_feature(
                                    PlanetaryFeature::Station(current_resources - 1usize)
                                );
                                
                                Some(ShipGoal::VisitPlanetWithResources(target_index))
                            },
                            None => {
                                None
                            }
                        }
                    },
                    Some(ShipGoal::VisitPlanetWithResources(current_planet)) => {
                        simulation.planets[current_planet].set_feature(
                            PlanetaryFeature::Station(current_resources + 1usize)
                        );

                        match simulation.planets_with_feature(
                            Some(std::mem::discriminant(&planet::PlanetaryFeature::Station(0usize)))
                        ).filter(|planet| {
                            if let Some(PlanetaryFeature::Station(r)) = planet.feature() {
                                return r > current_resources;
                            }
                            
                            panic!()
                        } ).choose(&mut rand::thread_rng()) {
                            Some(target_planet) => {
                                Some(ShipGoal::VisitPlanet(target_planet.index()))
                            },
                            None => {
                                None
                            }
                        }
                    },
                    None => {
                        match simulation.planets_with_feature(
                            Some(std::mem::discriminant(&planet::PlanetaryFeature::Station(0usize)))
                        ).choose(&mut rand::thread_rng()) {
                            Some(target) => {
                                Some(ShipGoal::VisitPlanet(target.index()))
                            },
                            None => None
                        }
                    },
                    _ => panic!()
                 }
            },
            ShipBehavior::Pirate => {
                None
            }
        };
    }

    pub(crate) fn update(&mut self, simulation: &mut Simulation) {
        if let Some(goal) = &self.goal {
            match goal {
                ShipGoal::VisitPlanet(index) | ShipGoal::VisitPlanetWithResources(index) => {
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
        } else {
            self.set_objective(simulation);
        }
    }
}

#[derive(Copy, Clone)]
enum ShipGoal {
    VisitPlanet(usize),
    VisitPlanetWithResources(usize),
    Wait(usize, usize)
}

#[derive(Copy, Clone, strum_macros::EnumIter)]
pub(crate) enum ShipBehavior {
    Miner,
    Trader,
    Pirate
}