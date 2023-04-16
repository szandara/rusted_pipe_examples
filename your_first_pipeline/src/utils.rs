use std::{
    thread,
    time::{Duration, Instant},
};

pub struct FpsLimiter {
    fps_control: Instant,
    fps_wait: Duration,
}

impl FpsLimiter {
    pub fn new(fps: usize) -> Self {
        Self {
            fps_control: Instant::now(),
            fps_wait: Duration::from_millis(1000 / fps as u64),
        }
    }

    pub fn wait(&self) {
        let elapsed = self.fps_control.elapsed();

        if self.fps_wait > elapsed {
            thread::sleep(self.fps_wait - elapsed);
        }
    }
}
