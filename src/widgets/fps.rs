use std::time::Duration;

const FRAMES_CAPACITY: usize = 60;

pub struct Meter {
    last_fps: Vec<i32>,
}

impl Meter {
    pub fn new() -> Self {
        Meter {
            last_fps: vec![0; FRAMES_CAPACITY],
        }
    }

    pub fn push(&mut self, duration: Duration) {
        self.last_fps.pop();
        self.last_fps.insert(0, self.get_fps(duration));
    }

    fn get_fps(&self, duration: Duration) -> i32 {
        (1.0 / duration.as_secs_f32()) as i32
    }

    pub fn get_average(&self) -> i32 {
        let sum: i32 = self.last_fps.iter().sum();
        sum / self.last_fps.len() as i32
    }
}
