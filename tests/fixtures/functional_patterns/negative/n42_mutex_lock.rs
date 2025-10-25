use std::sync::Mutex;

fn increment_shared(counter: &Mutex<i32>) {
    let mut value = counter.lock().unwrap();
    *value += 1;
}
