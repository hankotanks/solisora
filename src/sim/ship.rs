use rand::Rng;

pub struct Ship {
    pub pos: cgmath::Point2<f32>,
    pub speed: f32,
    pub initial_speed: f32,
    pub angle: f32,
    pub goal: ShipGoal,
    pub ship_type: ShipType,
}

impl Ship {
    fn new(ship_type: ShipType) -> Self {
        let mut prng = rand::thread_rng();
        let speed = prng.gen::<f32>() * 0.01f32;
        Self {
            pos: (0f32, 0f32).into(),
            speed,
            initial_speed: speed,
            angle: 0f32,
            goal: ShipGoal::Visit(0usize),
            ship_type
        }
    }
}

pub enum ShipType {
    Trader,
    Miner
}

pub enum ShipGoal {
    Visit(usize),
    Wait(usize, usize)
}