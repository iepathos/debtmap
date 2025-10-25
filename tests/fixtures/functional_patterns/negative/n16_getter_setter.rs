struct Counter {
    value: i32,
}

impl Counter {
    fn increment(&mut self) {
        self.value += 1;
    }

    fn get(&self) -> i32 {
        self.value
    }
}

fn use_counter() -> i32 {
    let mut c = Counter { value: 0 };
    for _ in 0..10 {
        c.increment();
    }
    c.get()
}
