#[derive(Copy, Clone)]
pub(crate) struct Point {
    x: f32,
    y: f32
}

impl Default for Point {
    fn default() -> Self {
        Self { 
            x: 0f32, 
            y: 0f32 
        }
    }
}

impl Point {
    pub(crate) fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub(crate) fn x(&self) -> f32 {
        self.x
    }

    pub(crate) fn y(&self) -> f32 {
        self.y
    }
}