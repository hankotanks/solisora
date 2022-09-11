use cgmath::{MetricSpace, Rad, Angle};
use rand::{seq::{SliceRandom, IteratorRandom}, Rng};
use strum::IntoEnumIterator;

use crate::simulation::Simulation;

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
            pos: (0f32, 0f32).into(),
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
                None
            },
            ShipBehavior::Trader => {
                Some(ShipGoal::ArriveAt( {
                    *simulation.planets_with_stations().choose(&mut rand::thread_rng()).unwrap()
                } ))
            },
            ShipBehavior::Pirate => {
                None
            }
        };
    }

    fn clear_objective(&mut self) {
        self.goal = None;
    }

    pub(crate) fn update(&mut self, simulation: &Simulation) {
        if let Some(goal) = &self.goal {
            match goal {
                ShipGoal::ArriveAt(index) => {
                    let goal_pos = simulation.planets[*index].pos();
    
                    let distance = self.pos.distance2(goal_pos);

                    if distance <= simulation.planets[*index].radius().powf(2f32) {
                        self.clear_objective();
                        self.set_objective(simulation);

                        self.speed = self.initial_speed;
                    }

                    let dx = goal_pos.x - self.pos.x;
                    let dy = goal_pos.y - self.pos.y;

                    self.pos.x += dx * self.speed;
                    self.pos.y += dy * self.speed;

                    self.angle = Rad::atan2(dx, dy).0 + 3.14;

                    self.speed *= 1.05f32;
                }
            }
        }
    }
}

#[derive(Copy, Clone)]
enum ShipGoal {
    ArriveAt(usize)
}

#[derive(Copy, Clone, strum_macros::EnumIter)]
pub(crate) enum ShipBehavior {
    Miner,
    Trader,
    Pirate
}