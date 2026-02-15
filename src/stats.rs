use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};


pub struct Stats {
    pub(crate) tx_packets: AtomicU64,
    pub(crate) rx_packets: AtomicU64,
    pub(crate) tx_bytes: AtomicU64,
    pub(crate) rx_bytes: AtomicU64,
    pub(crate) dropped: AtomicU64,
}

impl Stats {
    pub(crate) fn new() -> Self {
        Self {
            tx_packets: AtomicU64::new(0),
            rx_packets: AtomicU64::new(0),
            tx_bytes: AtomicU64::new(0),
            rx_bytes: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
        }
    }

    pub fn tx_packets(&self) -> u64 {
        self.tx_packets.load(Ordering::Relaxed)
    }

    pub fn rx_packets(&self) -> u64 {
        self.rx_packets.load(Ordering::Relaxed)
    }

    pub fn tx_bytes(&self) -> u64 {
        self.tx_bytes.load(Ordering::Relaxed)
    }

    pub fn rx_bytes(&self) -> u64 {
        self.rx_bytes.load(Ordering::Relaxed)
    }

    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "tx: {} pkts ({} B) | rx: {} pkts ({} B) | dropped: {}",
            self.tx_packets(),
            self.tx_bytes(),
            self.rx_packets(),
            self.rx_bytes(),
            self.dropped(),
        )
    }
}
