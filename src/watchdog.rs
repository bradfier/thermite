// IO Stall Monitor
extern crate log;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Instant, Duration};

pub fn watch(last_io: Arc<Mutex<Instant>>, threshold: u64, interval: u64) {
    loop {
        let since_last: Duration = {
            let last = *last_io.lock().unwrap();
            let now = Instant::now();
            now - last
        };

        if since_last.as_secs() > threshold {
            warn!("IO stalled for {} seconds.", since_last.as_secs())
        }

        thread::sleep(Duration::from_secs(interval));
    }
}
