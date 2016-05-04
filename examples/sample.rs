extern crate executor;

use std::io;
use std::io::Write;
use std::cell::RefCell;
use std::rc::Rc;
use executor::IoService;
use executor::Executor;

fn main() {
    let mut io_service = Rc::new(RefCell::new(IoService::new().unwrap()));

    {
        let io_service2 = io_service.clone();
        io_service.borrow_mut().schedule_timeout(2000, move || {
            println!(" World");
            // TODO: it crashes here
            io_service2.borrow_mut().post(|| println!("Goodbye cruel world"));
        });
    }

    io_service.borrow_mut().schedule_timeout(1000, || {
        print!(" big");
        io::stdout().flush();
    });

    print!("Hello");
    io::stdout().flush();

    io_service.borrow_mut().run();
}
