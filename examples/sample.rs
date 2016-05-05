extern crate executor;

use std::io;
use std::io::Write;
use std::rc::Rc;
use executor::IoService;
use executor::Executor;

fn main() {
    let io_service = Rc::new(IoService::new().unwrap());

    {
        let io_service2 = io_service.clone();
        io_service.schedule_timeout(2000, move || {
            io_service2.post(|| println!("Goodbye cruel world"));
            println!(" World");
        });
    }

    io_service.schedule_timeout(1000, || {
        print!(" big");
        let _ = io::stdout().flush();
    });

    print!("Hello");
    let _ = io::stdout().flush();

    io_service.run();
}
