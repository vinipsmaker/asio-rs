use std::collections::BTreeMap;

pub struct Closure(Box<FnMut()>);

impl Closure {
    pub fn new<F : FnOnce() + 'static>(f: F) -> Closure {
        let mut f = Some(f);
        Closure(Box::new(move || {
            if let Some(f) = f.take() {
                f()
            }
        }))
    }

    pub fn invoke(&mut self) {
        (self.0)()
    }
}

pub fn get_unused_idx<V>(map: &BTreeMap<usize, V>, hint: Option<usize>)
                         -> usize {
    let mut cur = hint.unwrap_or(0);
    while map.contains_key(&cur) {
        cur = cur.wrapping_add(1);
    }
    cur
}
