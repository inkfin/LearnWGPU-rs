use std::collections::VecDeque;
// use std::time::{Duration, Instant}; // this will panic on WASM
use instant::{Duration, Instant};

#[derive(Debug)]
pub struct Timer {
    pub elapse_timer: Instant,
    pub render_timer: Instant,
    pub state_timer: Instant,
    pub all_events_timer: Instant,

    pub time_vec_render: VecDeque<f32>,
    pub time_vec_state: VecDeque<f32>,
    pub time_vec_all_events: VecDeque<f32>,
    pub time_vec_max_size: i32,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            elapse_timer: Instant::now(),
            render_timer: Instant::now(),
            state_timer: Instant::now(),
            all_events_timer: Instant::now(),
            time_vec_render: VecDeque::new(),
            time_vec_state: VecDeque::new(),
            time_vec_all_events: VecDeque::new(),
            time_vec_max_size: 128,
        }
    }

    pub fn get_all_events_time(&self) -> Duration {
        self.all_events_timer.elapsed()
    }

    pub fn get_and_update_render_time(&mut self) -> Duration {
        let elapse = self.render_timer.elapsed();
        // info!("Render delta time: {:?}", elapse);

        // update time vec
        if self.time_vec_render.len() as i32 >= self.time_vec_max_size {
            self.time_vec_render.pop_front();
        }
        self.time_vec_render.push_back(elapse.as_secs_f32());

        elapse
    }

    pub fn get_and_update_state_time(&mut self) -> Duration {
        let elapse = self.state_timer.elapsed();
        // info!("State delta time: {:?}", elapse);

        // update time vec
        if self.time_vec_state.len() as i32 >= self.time_vec_max_size {
            self.time_vec_state.pop_front();
        }
        self.time_vec_state.push_back(elapse.as_secs_f32());

        elapse
    }

    pub fn get_and_update_all_events_time(&mut self) -> Duration {
        let all_events_time = self.get_all_events_time();
        // info!("All events delta time: {:?}", all_events_time);
        self.all_events_timer = Instant::now();

        // update time vec
        if self.time_vec_all_events.len() as i32 >= self.time_vec_max_size {
            self.time_vec_all_events.pop_front();
        }
        self.time_vec_all_events
            .push_back(all_events_time.as_secs_f32());

        all_events_time
    }
}
