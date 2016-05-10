extern crate asio;

use std::rc::Rc;
use asio::{IoService, SocketReactor};

fn main() {
    let io_service = Rc::new(IoService::new().unwrap());

    let addr = "127.0.0.1:8080".parse().unwrap();
    <IoService as SocketReactor>::TcpSocket::connect(io_service.clone(), &addr, |res| {
        match res {
            Ok(socket) => {
                println!("Connected");
                socket.read(Vec::with_capacity(1024), move |res| {
                    println!("{:?}", res);
                });
            }
            Err(e) => println!("Error: {:?}", e),
        };
    });

    io_service.run();
}
