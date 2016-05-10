use std::cell::RefCell;
use std::rc::Rc;
use std::collections::VecDeque;
use executor::Executor;
use utils::Closure;

pub struct LoopScheduler {
    pending_jobs: Rc<RefCell<VecDeque<Closure>>>,
    // useful for nested operations (e.g. callback being added while another
    // callback is executed)
    //running_callback: bool,
}

impl LoopScheduler {
    pub fn new() -> LoopScheduler {
        LoopScheduler {
            pending_jobs: Rc::new(RefCell::new(VecDeque::new())),
            //running_callback: false,
        }
    }

    pub fn run(&mut self) {
        loop {
            let mut j = {
                match self.pending_jobs.borrow_mut().pop_front() {
                    Some(j) => j,
                    None => break,
                }
            };
            //self.running_callback = true;
            j.invoke();
            //self.running_callback = false;
        }
    }
}

impl Executor for LoopScheduler {
    fn post<F : FnOnce() + 'static>(&self, f: F) {
        self.pending_jobs.borrow_mut().push_back(Closure::new(f));
    }
}
