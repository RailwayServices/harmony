use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static CLOCK_MS: AtomicU64 = AtomicU64::new(0);

pub fn now_millis_cached() -> u64 {
    let cached = CLOCK_MS.load(Ordering::Relaxed);
    if cached == 0 {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        CLOCK_MS.store(now, Ordering::Relaxed);
        now
    } else {
        cached
    }
}

pub fn start_clock_ticker() {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(1));
        loop {
            interval.tick().await;
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
            CLOCK_MS.store(now, Ordering::Relaxed);
        }
    });
}

#[derive(Debug, Clone)]
pub struct ActionWindow {
    timestamps: VecDeque<u64>,
    window_ms: u64,
}

impl ActionWindow {
    #[must_use]
    pub fn new(window_secs: u32) -> Self {
        Self { timestamps: VecDeque::with_capacity(32), window_ms: u64::from(window_secs) * 1_000 }
    }

    pub fn push_and_count(&mut self, now_ms: u64) -> usize {
        while let Some(&front) = self.timestamps.front() {
            if now_ms.saturating_sub(front) > self.window_ms {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }
        self.timestamps.push_back(now_ms);
        self.timestamps.len()
    }

    pub fn clear(&mut self) {
        self.timestamps.clear();
    }
}
