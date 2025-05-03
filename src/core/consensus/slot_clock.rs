use crate::types::Slot;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Configuration for slot timing
pub struct SlotConfig {
    /// Duration of each slot in seconds
    pub slot_duration: Duration,
    
    /// Genesis time (start of slot 0)
    pub genesis_time: SystemTime,
}

impl Default for SlotConfig {
    fn default() -> Self {
        Self {
            // 12 second slots by default
            slot_duration: Duration::from_secs(12),
            
            // Default genesis time is the Unix epoch
            genesis_time: UNIX_EPOCH,
        }
    }
}

/// SlotClock provides utilities for slot-based timing
pub struct SlotClock {
    config: SlotConfig,
}

impl SlotClock {
    /// Create a new SlotClock with the given configuration
    pub fn new(config: SlotConfig) -> Self {
        Self { config }
    }

    /// Get the current slot based on system time
    pub fn current_slot(&self) -> Slot {
        let now = SystemTime::now();
        self.slot_at_time(now)
    }

    /// Get the slot at a specific time
    pub fn slot_at_time(&self, time: SystemTime) -> Slot {
        let duration_since_genesis = time
            .duration_since(self.config.genesis_time)
            .unwrap_or(Duration::from_secs(0));
        
        (duration_since_genesis.as_millis() / self.config.slot_duration.as_millis()) as u64
    }

    /// Get the start time of a specific slot
    pub fn slot_start_time(&self, slot: Slot) -> SystemTime {
        let duration = Duration::from_secs(
            slot * self.config.slot_duration.as_secs()
        );
        self.config.genesis_time + duration
    }

    /// Check if the current slot is different from the last slot
    pub fn is_new_slot(&self, last_slot: Slot) -> bool {
        self.current_slot() > last_slot
    }

    /// Get the duration until the start of the next slot
    pub fn duration_until_next_slot(&self) -> Duration {
        let current_slot = self.current_slot();
        let next_slot_start = self.slot_start_time(current_slot + 1);
        
        match next_slot_start.duration_since(SystemTime::now()) {
            Ok(duration) => duration,
            Err(_) => Duration::from_secs(0), // Already in the next slot
        }
    }
} 