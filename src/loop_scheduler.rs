use executor::Executor;
use utils::Closure;
use std::collections::VecDeque;

pub struct LoopScheduler {
    pending_jobs: VecDeque<Closure>,
    // useful for nested operations (e.g. callback being added while another
    // callback is executed)
    //running_callback: bool,
}

impl LoopScheduler {
    pub fn new() -> LoopScheduler {
        LoopScheduler {
            pending_jobs: VecDeque::new(),
            //running_callback: false,
        }
    }

    pub fn run(&mut self) {
        while let Some(j) = self.pending_jobs.pop_front() {
            //self.running_callback = true;
            j.invoke();
            //self.running_callback = false;
        }
    }
}

impl Executor for LoopScheduler {
    fn post<F : FnOnce() + 'static>(&mut self, f: F) {
        self.pending_jobs.push_back(Closure::new(f));
    }
}
