use std::{fmt::Display, time::{Duration, Instant}};

pub struct StopWatch {
    start_time: Option<Instant>,
    time_elapsed: Duration,
    is_finished: bool
} 

impl StopWatch {
    pub fn new() -> Self {
        StopWatch {
            start_time: None,
            time_elapsed: Duration::new(0, 0),
            is_finished: false
        }
    }

    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.is_finished = false;
    }

    pub fn get_current_instant(self) -> Result<Duration, String> {
        if let Some(start_time) = self.start_time {
            Ok(start_time.elapsed())
        }
        else {
            Err("start_time not initialized".to_string())
        }
    }

    pub fn reset(&mut self) {
        self.start_time = None;
        self.time_elapsed = Duration::ZERO;
        self.is_finished = false;
    }

    pub fn stop(&mut self) {
        if let Some(start_time) = self.start_time {
            self.time_elapsed = start_time.elapsed();
            self.is_finished = true;
        }
    }
}


//Not compliant because the trait Debug should have been used
impl Display for StopWatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_finished 
        {
            write!(f, "Started at {:?} - Last for {:?}", self.start_time.unwrap(), self.time_elapsed) 
        }
        else {
            write!(f, "Time[{:?}]", self.start_time.unwrap().elapsed()) 
        }
    }
}