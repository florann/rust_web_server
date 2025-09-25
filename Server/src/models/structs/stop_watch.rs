use std::time::{Duration, Instant};

pub struct StopWatch {
    start_time: Option<Instant>,
    time_elapsed: Duration,
} 

impl StopWatch {
    fn new() -> Self {
        StopWatch {
            start_time: None,
            time_elapsed: Duration::new(0, 0)
        }
    }

    fn start(&mut self) {
        self.start_time = Some(Instant::now())
    }

    fn get_current_instant(self) -> Result<Duration, String> {
        if let Some(start_time) = self.start_time {
            Ok(start_time.elapsed())
        }
        else {
            Err("start_time not initialized".to_string())
        }
    }

    fn stop(&mut self) {
        if let Some(start_time) = self.start_time {
            self.time_elapsed = start_time.elapsed();
        }
    }
}