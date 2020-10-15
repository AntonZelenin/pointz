use std::time::Duration;

const FRAMES_CAPACITY: usize = 60;

pub struct Meter {
    last_fps: Vec<i32>,
}

impl Meter {
    pub fn new() -> Self {
        Meter {
            last_fps: Vec::with_capacity(FRAMES_CAPACITY),
        }
    }

    pub fn push(&mut self, duration: Duration) {
        if self.last_fps.len() == FRAMES_CAPACITY {
            self.last_fps.pop();
        }
        self.last_fps.insert(0, self.get_fps(duration));
    }

    fn get_fps(&self, duration: Duration) -> i32 {
        (1.0 / duration.as_secs_f32()) as i32
    }

    pub fn get_average(&self) -> i32 {
        let len = self.last_fps.len() as i32;
        if len == 0 {
            return 0;
        }
        let sum: i32 = self.last_fps.iter().sum();
        sum / len
    }
}
