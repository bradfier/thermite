// Copyright 2015 Thermite Developers. See the LICENSE
// file at the top-level directory of this distribution

// IO Stall Monitor
extern crate log;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Instant, Duration};

pub fn watch(last_io: Arc<Mutex<HashMap<String, Instant>>>, threshold: u64, interval: u64) {
    loop {

        for (key, value) in last_io.lock().unwrap().iter_mut() {
            let since_last: Duration = {
                let last = *value;
                let now = Instant::now();
                now - last
            };

            if since_last.as_secs() > threshold {
                warn!("IO stalled on {} for {} seconds.",
                      *key,
                      since_last.as_secs())
            }
        }

        thread::sleep(Duration::from_secs(interval));
    }
}
