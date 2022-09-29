use strum::EnumIter;

pub struct Ship {
    pub pos: cgmath::Point2<f32>,
    pub speed: f32,
    pub initial_speed: f32,
    pub angle: f32,
    pub goal: ShipGoal,
    pub job: ShipJob,
}

impl Ship {
    pub fn new(job: ShipJob, speed: f32) -> Self {
        Self {
            pos: (0f32, 0f32).into(),
            speed,
            initial_speed: speed,
            angle: 0f32,
            goal: ShipGoal::Visit { target: 0 },
            job
        }
    }
}

#[derive(Copy, Clone, EnumIter)]
pub enum ShipJob {
    Trader { has_ore: bool },
    Miner,
    Pirate
}

#[derive(Copy, Clone)]
pub enum ShipGoal {
    Visit { target: usize },
    Wait { target: usize, progress: usize },
    Wander,
    Hunt { prey: usize },
    Scan,
}