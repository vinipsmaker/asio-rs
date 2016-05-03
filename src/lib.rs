mod utils;

use std::collections::VecDeque;
use utils::Closure;

pub struct Executor {
    pending_jobs: VecDeque<Closure>,
    // useful for nested operations (e.g. callback being added while another
    // callback is executed)
    //running_callback: bool,
}

// TODO:
//
// - mio operations
//   - tcp and udp socket oeprations
//   - acceptor operations
//   - timer oeprations
// - move away from callbacks
//   - use coroutines
impl Executor {
    pub fn new() -> Executor {
        Executor {
            pending_jobs: VecDeque::new(),
            //running_callback: false,
        }
    }

    pub fn post<F : FnOnce() + Send + 'static>(&mut self, f: F) {
        self.pending_jobs.push_back(Closure::new(f));
    }

    pub fn run(&mut self) {
        while let Some(j) = self.pending_jobs.pop_front() {
            //self.running_callback = true;
            j.invoke();
            //self.running_callback = false;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
