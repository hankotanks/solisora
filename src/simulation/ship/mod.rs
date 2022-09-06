use cgmath::{MetricSpace, Rad, Angle};
use rand::{seq::SliceRandom, Rng};

use crate::simulation::Simulation;

#[derive(Clone)]
pub(crate) struct Ship {
    goal: Option<ShipGoal>,
    pos: cgmath::Point2<f32>,
    speed: f32,
    initial_speed: f32,
    angle: f32
}

impl std::hash::Hash for Ship {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.goal.hash(state);
        self.initial_speed.to_string().hash(state);
    }
}

impl Ship {
    pub(crate) fn pos(&self) -> cgmath::Point2<f32> {
        self.pos
    }

    pub(crate) fn angle(&self) -> f32 {
        self.angle
    }
}

impl Ship {
    pub(crate) fn new(simulation: &Simulation) -> Self {
        let speed = rand::thread_rng().gen::<f32>() * 0.01f32;
        let mut ship = Self {
            goal: None,
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
        self.goal = Some(
            ShipGoal::ArriveAt( {
                *simulation.bodies_with_stations().choose(&mut rand::thread_rng()).unwrap()
            } )
        );
    }

    fn clear_objective(&mut self) {
        self.goal = None;
    }

    pub(crate) fn update(&mut self, simulation: &Simulation) {
        if let Some(goal) = &self.goal {
            match goal {
                ShipGoal::ArriveAt(index) => {
                    let goal_pos = simulation.bodies[*index].pos();
    
                    let distance = self.pos.distance2(goal_pos);

                    if distance <= simulation.bodies[*index].radius().powf(2f32) {
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

#[derive(Copy, Clone, std::hash::Hash)]
enum ShipGoal {
    ArriveAt(usize)
}