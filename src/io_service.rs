use std::io;
use std::collections::BTreeMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::net;
use mio::{Evented, EventLoop, EventSet, Handler, PollOpt, Token, tcp};
use utils::{Closure, get_unused_idx};
use executor::Executor;
use timer_queue::TimerQueue;
use socket_reactor::SocketReactor;

struct IoServiceHandler {
    pending_jobs: Rc<RefCell<usize>>,
    pending_io_msgs: BTreeMap<usize, Closure>,
}

impl IoServiceHandler {
    fn new(pending_jobs: Rc<RefCell<usize>>,) -> Self {
        IoServiceHandler {
            pending_jobs: pending_jobs,
            pending_io_msgs: BTreeMap::new(),
        }
    }
}

impl Handler for IoServiceHandler {
    type Timeout = Closure;
    type Message = ();

    fn ready(&mut self, _event_loop: &mut EventLoop<Self>, token: Token,
             _events: EventSet) {
        let Token(idx) = token;
        if let Some(mut msg) = self.pending_io_msgs.remove(&idx) {
            msg.invoke();
        }
    }

    fn notify(&mut self, _event_loop: &mut EventLoop<Self>,
              _msg: Self::Message) {
    }

    fn timeout(&mut self, _event_loop: &mut EventLoop<Self>,
               mut timeout: Self::Timeout) {
        *self.pending_jobs.borrow_mut() -= 1;
        timeout.invoke();
    }

    fn interrupted(&mut self, _event_loop: &mut EventLoop<Self>) { }

    fn tick(&mut self, _event_loop: &mut EventLoop<Self>) { }
}

// NOTE: Only required because Rc doesn't allow upcast.
enum EventedVariant {
    Tcp(Rc<RefCell<tcp::TcpStream>>),
}

// TODO:
//
// - Replace some `RefCell`s for `std::cell::Cell`.
pub struct IoService {
    pending_jobs: Rc<RefCell<usize>>,
    pending_msgs: Rc<RefCell<Vec<Closure>>>,
    pending_timeout_msgs: Rc<RefCell<Vec<(u64, Closure)>>>,
    pending_io_msgs: Rc<RefCell<Vec<(EventedVariant, EventSet, Closure)>>>,
    event_loop: Rc<RefCell<EventLoop<IoServiceHandler>>>,
    handler: Rc<RefCell<IoServiceHandler>>,
}

impl IoService {
    pub fn new() -> io::Result<IoService> {
        let pending_jobs = Rc::new(RefCell::new(0));
        let handler = IoServiceHandler::new(pending_jobs.clone());
        let event_loop = try!(EventLoop::new());
        Ok(IoService {
            pending_jobs: pending_jobs,
            pending_msgs: Rc::new(RefCell::new(Vec::new())),
            pending_timeout_msgs: Rc::new(RefCell::new(Vec::new())),
            pending_io_msgs: Rc::new(RefCell::new(Vec::new())),
            event_loop: Rc::new(RefCell::new(event_loop)),
            handler: Rc::new(RefCell::new(handler)),
        })
    }

    pub fn run(&self) {
        let mut handler = self.handler.borrow_mut();
        while *self.pending_jobs.borrow() != 0 {
            let mut event_loop = self.event_loop.borrow_mut();
            event_loop.run_once(&mut handler, Some(10)).unwrap();

            let mut pending_timeout_msgs = self.pending_timeout_msgs
                .borrow_mut();
            while let Some((timeout_ms, c)) = pending_timeout_msgs.pop() {
                event_loop.timeout_ms(c, timeout_ms).unwrap();
            }

            let mut pending_msgs = self.pending_msgs.borrow_mut();
            for ref mut m in pending_msgs.iter_mut() {
                m.invoke();
                *self.pending_jobs.borrow_mut() -= 1;
            }
            pending_msgs.clear();

            let mut pending_io_msgs = self.pending_io_msgs.borrow_mut();
            while let Some((evariant, eset, msg)) = pending_io_msgs.pop() {
                let guard = match evariant {
                    EventedVariant::Tcp(ref e) => e.borrow(),
                };
                let evented: &Evented = &*guard;
                let idx = get_unused_idx(&handler.pending_io_msgs, None);
                let token = Token(idx);
                handler.pending_io_msgs.insert(idx, msg);
                event_loop.register(evented, token, eset, PollOpt::level())
                    .unwrap();
            }
            pending_io_msgs.clear();
        }
    }
}

impl Executor for IoService {
    fn post<F : FnOnce() + 'static>(&self, f: F) {
        *self.pending_jobs.borrow_mut() += 1;
        self.pending_msgs.borrow_mut().push(Closure::new(f));
    }
}

impl TimerQueue for IoService {
    fn schedule_timeout<F>(&self, timeout_ms: u64, f: F)
        where F : FnOnce() + 'static {
        *self.pending_jobs.borrow_mut() += 1;
        self.pending_timeout_msgs.borrow_mut().push((timeout_ms,
                                                     Closure::new(f)));
    }
}

// TODO:
//
// - Break implementation into traits.
pub struct TcpSocket {
    io_service: Rc<IoService>,
    stream: Rc<RefCell<tcp::TcpStream>>,
}

impl TcpSocket {
    fn connect_impl<F>(io_service: Rc<IoService>,
                       stream: io::Result<tcp::TcpStream>, f: F)
        where F: FnOnce(io::Result<TcpSocket>) + 'static {
        let stream = match stream {
            Ok(s) => Rc::new(RefCell::new(s)),
            Err(e) => {
                io_service.post(move || f(Err(e)));
                return;
            }
        };
        let stream2 = stream.clone();
        let io_service2 = io_service.clone();
        let cb = move || {
            let s = TcpSocket {
                io_service: io_service2,
                stream: stream2,
            };
            let res = s.stream.borrow().take_socket_error();
            f(res.and(Ok(s)));
        };
        *io_service.pending_jobs.borrow_mut() += 1;
        let io = (EventedVariant::Tcp(stream), EventSet::writable(),
                  Closure::new(cb));
        io_service.pending_io_msgs.borrow_mut().push(io);
    }

    pub fn connect<F>(io_service: Rc<IoService>, addr: &net::SocketAddr, f: F)
        where F: FnOnce(io::Result<TcpSocket>) + 'static {
        Self::connect_impl(io_service, tcp::TcpStream::connect(addr), f)
    }

    pub fn connect_stream<F>(io_service: Rc<IoService>, stream: net::TcpStream,
                             addr: &net::SocketAddr, f: F)
        where F: FnOnce(io::Result<TcpSocket>) + 'static {
        Self::connect_impl(io_service,
                           tcp::TcpStream::connect_stream(stream, addr),
                           f)
    }

    pub fn read<F>(&self, mut buf: Vec<u8>, f: F)
        where F: FnOnce(io::Result<Vec<u8>>) + 'static {
        let stream = self.stream.clone();
        let cb = move || {
            use std::io::Read;
            f(stream.borrow_mut().read(&mut buf).map(|len| {
                buf.truncate(len);
                buf
            }));
        };
        *self.io_service.pending_jobs.borrow_mut() += 1;
        let io = (EventedVariant::Tcp(self.stream.clone()),
                  EventSet::readable(), Closure::new(cb));
        self.io_service.pending_io_msgs.borrow_mut().push(io);
    }
}

impl SocketReactor for IoService {
    type TcpSocket = TcpSocket;
}
