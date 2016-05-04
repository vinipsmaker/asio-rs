use std::io;
use std::collections::BTreeMap;
use std::num::Wrapping;
use mio::{EventLoop, EventSet, Handler, Token};
//use mio::tcp::{TcpListener, TcpStream};
use utils::Closure;
use executor::Executor;

struct IoServiceHandler {
    pub pending_jobs: usize,
    pub pending_msgs: BTreeMap<usize, Closure>,
    pub next_msg: Wrapping<usize>,
}

impl Default for IoServiceHandler {
    fn default() -> Self {
        IoServiceHandler {
            pending_jobs: 0,
            pending_msgs: BTreeMap::new(),
            next_msg: Wrapping(0),
        }
    }
}

impl Handler for IoServiceHandler {
    type Timeout = Closure;
    type Message = usize;

    fn ready(&mut self, _event_loop: &mut EventLoop<Self>, _token: Token,
             _events: EventSet) {
    }

    fn notify(&mut self, event_loop: &mut EventLoop<Self>, msg: Self::Message) {
        self.pending_jobs -= 1;
        if let Some(msg) = self.pending_msgs.remove(&msg) {
            msg.invoke();
        }
        if self.pending_jobs == 0 {
            event_loop.shutdown();
        }
    }

    fn timeout(&mut self, event_loop: &mut EventLoop<Self>,
               timeout: Self::Timeout) {
        self.pending_jobs -= 1;
        timeout.invoke();
        if self.pending_jobs == 0 {
            event_loop.shutdown();
        }
    }

    fn interrupted(&mut self, _event_loop: &mut EventLoop<Self>) { }

    fn tick(&mut self, _event_loop: &mut EventLoop<Self>) { }
}

// TODO: break IoService into the following traits:
//
// - TimerQueue
// - SocketReactor
pub struct IoService {
    event_loop: EventLoop<IoServiceHandler>,
    handler: IoServiceHandler,
}

impl IoService {
    pub fn new() -> io::Result<IoService> {
        Ok(IoService {
            event_loop: try!(EventLoop::new()),
            handler: IoServiceHandler::default(),
        })
    }

    pub fn schedule_timeout<F>(&mut self, timeout_ms: u64, f: F)
        where F : FnOnce() + 'static {
        self.handler.pending_jobs += 1;
        self.event_loop.timeout_ms(Closure::new(f), timeout_ms);
    }

    pub fn run(&mut self) {
        self.event_loop.run(&mut self.handler);
    }
}

impl Executor for IoService {
    fn post<F : FnOnce() + 'static>(&mut self, f: F) {
        let Wrapping(cur) = self.handler.next_msg;
        self.handler.next_msg += Wrapping(1);
        self.handler.pending_msgs.insert(cur, Closure::new(f));
        self.event_loop.channel().send(cur);
    }
}
