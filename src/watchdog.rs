// IO Stall Monitor

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Instant, Duration};

pub fn watch(last_io: Arc<Mutex<Instant>>, threshold: u64, interval: u64) {
    loop {
        let since_last: Duration = {
            let now = Instant::now();
            let last = *last_io.lock().unwrap();
            now - last
        };

        if since_last.as_secs() > threshold {
            println!("IO stalled for {} seconds.", since_last.as_secs())
        }

        thread::sleep(Duration::from_secs(interval));
    }
}
