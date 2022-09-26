use strum::EnumIter;

pub struct Ship {
    pub pos: cgmath::Point2<f32>,
    pub speed: f32,
    pub initial_speed: f32,
    pub angle: f32,
    pub goal: ShipGoal,
    pub ship_type: ShipType,
}

impl Ship {
    pub fn new(ship_type: ShipType) -> Self {
        let speed = rand::Rng::gen::<f32>(&mut rand::thread_rng()) * 0.01f32;
        Self {
            pos: (0f32, 0f32).into(),
            speed,
            initial_speed: speed,
            angle: 0f32,
            goal: ShipGoal::Visit { target: 0 },
            ship_type
        }
    }
}

#[derive(Copy, Clone, EnumIter)]
pub enum ShipType {
    Trader { has_resource: bool },
    Miner
}

#[derive(Copy, Clone)]
pub enum ShipGoal {
    Visit { target: usize },
    Wait { target: usize, counter: usize }
}