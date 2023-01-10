use std::time::Instant;

pub struct Timer {
    name: &'static str,
    instant: Instant,
}

impl Timer {
    pub fn new(name: &'static str) -> Self {
        Timer {
            name,
            instant: Instant::now(),
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        println!("{}: finished in {:?}", self.name, self.instant.elapsed())
    }
}
