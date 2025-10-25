use std::thread;

fn spawn_worker() {
    thread::spawn(|| {
        println!("Working");
    });
}
