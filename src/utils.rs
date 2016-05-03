pub struct Closure(Box<FnMut() + Send>);

impl Closure {
    pub fn new<F : FnOnce() + Send + 'static>(f: F) -> Closure {
        let mut f = Some(f);
        Closure(Box::new(move || {
            if let Some(f) = f.take() {
                f()
            }
        }))
    }

    pub fn invoke(mut self) {
        (self.0)()
    }
}
