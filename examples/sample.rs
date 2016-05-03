extern crate executor;

use executor::Executor;

fn main() {
    let mut executor = Executor::new();

    executor.post(|| println!(" World"));

    print!("Hello");

    executor.run();
}
