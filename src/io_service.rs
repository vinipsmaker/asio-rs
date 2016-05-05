use std::io;
use std::collections::BTreeMap;
use std::cell::RefCell;
use std::rc::Rc;
use mio::{EventLoop, EventSet, Handler, Sender, Token};
//use mio::tcp::{TcpListener, TcpStream};
use utils::Closure;
use executor::Executor;
use timer_queue::TimerQueue;

enum Message {
    UserClosure(usize),
    RegisterTimeout(u64, usize),
}

struct IoServiceHandler {
    pending_jobs: Rc<RefCell<usize>>,
    pending_msgs: Rc<RefCell<BTreeMap<usize, Closure>>>,
}

impl IoServiceHandler {
    fn new(pending_jobs: Rc<RefCell<usize>>,
           pending_msgs: Rc<RefCell<BTreeMap<usize, Closure>>>) -> Self {
        IoServiceHandler {
            pending_jobs: pending_jobs,
            pending_msgs: pending_msgs,
        }
    }
}

impl Handler for IoServiceHandler {
    type Timeout = Closure;
    type Message = Message;

    fn ready(&mut self, _event_loop: &mut EventLoop<Self>, _token: Token,
             _events: EventSet) {
    }

    fn notify(&mut self, event_loop: &mut EventLoop<Self>, msg: Self::Message) {
        *self.pending_jobs.borrow_mut() -= 1;
        match msg {
            Message::UserClosure(msg) => {
                if let Some(msg) = self.pending_msgs.borrow_mut().remove(&msg) {
                    msg.invoke();
                }
            }
            Message::RegisterTimeout(timeout_ms, msg) => {
                if let Some(msg) = self.pending_msgs.borrow_mut().remove(&msg) {
                    event_loop.timeout_ms(msg, timeout_ms).unwrap();
                }
            }
        }
        let shutdown = *self.pending_jobs.borrow() == 0;
        if shutdown {
            event_loop.shutdown();
        }
    }

    fn timeout(&mut self, event_loop: &mut EventLoop<Self>,
               timeout: Self::Timeout) {
        *self.pending_jobs.borrow_mut() -= 1;
        timeout.invoke();
        let shutdown = *self.pending_jobs.borrow() == 0;
        if shutdown {
            event_loop.shutdown();
        }
    }

    fn interrupted(&mut self, _event_loop: &mut EventLoop<Self>) { }

    fn tick(&mut self, _event_loop: &mut EventLoop<Self>) { }
}

// TODO: break IoService into the following traits:
//
// - SocketReactor
pub struct IoService {
    pending_jobs: Rc<RefCell<usize>>,
    pending_msgs: Rc<RefCell<BTreeMap<usize, Closure>>>,
    event_loop: Rc<RefCell<EventLoop<IoServiceHandler>>>,
    handler: Rc<RefCell<IoServiceHandler>>,
    msg_sender: Sender<Message>,
}

impl IoService {
    pub fn new() -> io::Result<IoService> {
        let pending_jobs = Rc::new(RefCell::new(0));
        let pending_msgs = Rc::new(RefCell::new(BTreeMap::new()));
        let handler = IoServiceHandler::new(pending_jobs.clone(),
                                            pending_msgs.clone());
        let event_loop = try!(EventLoop::new());
        let msg_sender = event_loop.channel();
        Ok(IoService {
            pending_jobs: pending_jobs,
            pending_msgs: pending_msgs,
            event_loop: Rc::new(RefCell::new(event_loop)),
            handler: Rc::new(RefCell::new(handler)),
            msg_sender: msg_sender,
        })
    }

    pub fn run(&self) {
        let mut handler = self.handler.borrow_mut();
        self.event_loop.borrow_mut().run(&mut handler).unwrap();
    }

    fn next_msg(&self) -> usize {
        let mut cur = 0;
        let pending_msgs = self.pending_msgs.borrow();
        while pending_msgs.contains_key(&cur) {
            cur += 1;
        }
        cur
    }
}

impl Executor for IoService {
    fn post<F : FnOnce() + 'static>(&self, f: F) {
        let cur = self.next_msg();
        *self.pending_jobs.borrow_mut() += 1;
        self.pending_msgs.borrow_mut().insert(cur, Closure::new(f));
        self.msg_sender.send(Message::UserClosure(cur)).unwrap();
    }
}

impl TimerQueue for IoService {
    fn schedule_timeout<F>(&self, timeout_ms: u64, f: F)
        where F : FnOnce() + 'static {
        *self.pending_jobs.borrow_mut() += 2;
        let cur = self.next_msg();
        self.pending_msgs.borrow_mut().insert(cur, Closure::new(f));
        self.msg_sender.send(Message::RegisterTimeout(timeout_ms, cur))
            .unwrap();
    }
}
