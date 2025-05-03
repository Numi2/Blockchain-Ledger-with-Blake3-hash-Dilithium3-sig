use lazy_static::lazy_static;
use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramVec, 
    IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Registry,
    Encoder, TextEncoder
};
use std::sync::Mutex;
use std::time::{Duration, Instant};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    
    // Block metrics
    pub static ref BLOCK_HEIGHT: IntGauge = IntGauge::new(
        "world_ledger_block_height", 
        "Current block height"
    ).expect("Failed to create block height metric");
    
    pub static ref BLOCK_TIME: Histogram = Histogram::with_opts(
        prometheus::HistogramOpts::new(
            "world_ledger_block_time_seconds", 
            "Time between blocks in seconds"
        ).buckets(vec![1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0])
    ).expect("Failed to create block time metric");
    
    pub static ref BLOCK_TXS: IntCounterVec = IntCounterVec::new(
        prometheus::Opts::new(
            "world_ledger_block_transactions_total", 
            "Total number of transactions in blocks"
        ),
        &["status"] // succeeded, failed
    ).expect("Failed to create block transactions metric");
    
    pub static ref BLOCK_SIZE: Histogram = Histogram::with_opts(
        prometheus::HistogramOpts::new(
            "world_ledger_block_size_bytes", 
            "Block size in bytes"
        ).buckets(vec![1024.0, 4096.0, 16384.0, 65536.0, 262144.0, 1048576.0])
    ).expect("Failed to create block size metric");
    
    // Peer metrics
    pub static ref PEER_COUNT: IntGauge = IntGauge::new(
        "world_ledger_peer_count", 
        "Number of connected peers"
    ).expect("Failed to create peer count metric");
    
    pub static ref PEER_MESSAGE_RECV: IntCounterVec = IntCounterVec::new(
        prometheus::Opts::new(
            "world_ledger_peer_message_recv_total", 
            "Total number of messages received from peers"
        ),
        &["type"] // block, tx, etc.
    ).expect("Failed to create peer message received metric");
    
    pub static ref PEER_MESSAGE_SENT: IntCounterVec = IntCounterVec::new(
        prometheus::Opts::new(
            "world_ledger_peer_message_sent_total", 
            "Total number of messages sent to peers"
        ),
        &["type"] // block, tx, etc.
    ).expect("Failed to create peer message sent metric");
    
    // Transaction pool metrics
    pub static ref MEMPOOL_SIZE: IntGauge = IntGauge::new(
        "world_ledger_mempool_size", 
        "Number of transactions in the mempool"
    ).expect("Failed to create mempool size metric");
    
    pub static ref MEMPOOL_BYTES: IntGauge = IntGauge::new(
        "world_ledger_mempool_bytes", 
        "Size of the mempool in bytes"
    ).expect("Failed to create mempool bytes metric");
    
    pub static ref TX_PROCESSING_TIME: Histogram = Histogram::with_opts(
        prometheus::HistogramOpts::new(
            "world_ledger_tx_processing_time_seconds", 
            "Time to process a transaction in seconds"
        ).buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0])
    ).expect("Failed to create tx processing time metric");
    
    // Consensus metrics
    pub static ref CONSENSUS_ROUNDS: IntCounterVec = IntCounterVec::new(
        prometheus::Opts::new(
            "world_ledger_consensus_rounds_total", 
            "Total number of consensus rounds"
        ),
        &["result"] // success, timeout, etc.
    ).expect("Failed to create consensus rounds metric");
    
    pub static ref CONSENSUS_TIME: Histogram = Histogram::with_opts(
        prometheus::HistogramOpts::new(
            "world_ledger_consensus_time_seconds", 
            "Time to reach consensus in seconds"
        ).buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0])
    ).expect("Failed to create consensus time metric");
    
    // Sync metrics
    pub static ref SYNC_HEIGHT: IntGauge = IntGauge::new(
        "world_ledger_sync_height", 
        "Current sync height"
    ).expect("Failed to create sync height metric");
    
    pub static ref SYNC_TARGET_HEIGHT: IntGauge = IntGauge::new(
        "world_ledger_sync_target_height", 
        "Target sync height"
    ).expect("Failed to create sync target height metric");
    
    pub static ref SYNC_PEERS: IntGauge = IntGauge::new(
        "world_ledger_sync_peers", 
        "Number of sync peers"
    ).expect("Failed to create sync peers metric");
    
    // RPC metrics
    pub static ref RPC_REQUESTS: IntCounterVec = IntCounterVec::new(
        prometheus::Opts::new(
            "world_ledger_rpc_requests_total", 
            "Total number of RPC requests"
        ),
        &["method", "result"] // method name, success/error
    ).expect("Failed to create RPC requests metric");
    
    pub static ref RPC_REQUEST_DURATION: HistogramVec = HistogramVec::new(
        prometheus::HistogramOpts::new(
            "world_ledger_rpc_request_duration_seconds", 
            "RPC request duration in seconds"
        ).buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
        &["method"]
    ).expect("Failed to create RPC request duration metric");
    
    // Contract metrics
    pub static ref CONTRACT_CALLS: IntCounterVec = IntCounterVec::new(
        prometheus::Opts::new(
            "world_ledger_contract_calls_total", 
            "Total number of contract calls"
        ),
        &["status"] // success, revert, error
    ).expect("Failed to create contract calls metric");
    
    pub static ref CONTRACT_GAS_USED: Histogram = Histogram::with_opts(
        prometheus::HistogramOpts::new(
            "world_ledger_contract_gas_used", 
            "Gas used in contract calls"
        ).buckets(vec![1000.0, 5000.0, 10000.0, 50000.0, 100000.0, 500000.0, 1000000.0])
    ).expect("Failed to create contract gas used metric");
    
    // System metrics
    pub static ref SYSTEM_MEMORY_BYTES: IntGauge = IntGauge::new(
        "world_ledger_system_memory_bytes", 
        "System memory usage in bytes"
    ).expect("Failed to create system memory metric");
    
    pub static ref SYSTEM_CPU_USAGE: Gauge = Gauge::new(
        "world_ledger_system_cpu_usage", 
        "System CPU usage (0.0-1.0)"
    ).expect("Failed to create system CPU metric");
    
    pub static ref SYSTEM_DISK_BYTES: IntGauge = IntGauge::new(
        "world_ledger_system_disk_bytes", 
        "System disk usage in bytes"
    ).expect("Failed to create system disk metric");
}

/// Initialize metrics system
pub fn init_metrics() {
    // Register metrics
    REGISTRY.register(Box::new(BLOCK_HEIGHT.clone())).unwrap();
    REGISTRY.register(Box::new(BLOCK_TIME.clone())).unwrap();
    REGISTRY.register(Box::new(BLOCK_TXS.clone())).unwrap();
    REGISTRY.register(Box::new(BLOCK_SIZE.clone())).unwrap();
    
    REGISTRY.register(Box::new(PEER_COUNT.clone())).unwrap();
    REGISTRY.register(Box::new(PEER_MESSAGE_RECV.clone())).unwrap();
    REGISTRY.register(Box::new(PEER_MESSAGE_SENT.clone())).unwrap();
    
    REGISTRY.register(Box::new(MEMPOOL_SIZE.clone())).unwrap();
    REGISTRY.register(Box::new(MEMPOOL_BYTES.clone())).unwrap();
    REGISTRY.register(Box::new(TX_PROCESSING_TIME.clone())).unwrap();
    
    REGISTRY.register(Box::new(CONSENSUS_ROUNDS.clone())).unwrap();
    REGISTRY.register(Box::new(CONSENSUS_TIME.clone())).unwrap();
    
    REGISTRY.register(Box::new(SYNC_HEIGHT.clone())).unwrap();
    REGISTRY.register(Box::new(SYNC_TARGET_HEIGHT.clone())).unwrap();
    REGISTRY.register(Box::new(SYNC_PEERS.clone())).unwrap();
    
    REGISTRY.register(Box::new(RPC_REQUESTS.clone())).unwrap();
    REGISTRY.register(Box::new(RPC_REQUEST_DURATION.clone())).unwrap();
    
    REGISTRY.register(Box::new(CONTRACT_CALLS.clone())).unwrap();
    REGISTRY.register(Box::new(CONTRACT_GAS_USED.clone())).unwrap();
    
    REGISTRY.register(Box::new(SYSTEM_MEMORY_BYTES.clone())).unwrap();
    REGISTRY.register(Box::new(SYSTEM_CPU_USAGE.clone())).unwrap();
    REGISTRY.register(Box::new(SYSTEM_DISK_BYTES.clone())).unwrap();
}

/// Timer helper for measuring durations
pub struct MetricsTimer {
    name: String,
    start: Instant,
}

impl MetricsTimer {
    /// Create a new metrics timer
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start: Instant::now(),
        }
    }
    
    /// Stop timer and record to specific histogram
    pub fn stop_and_record_histogram(self, histogram: &Histogram) {
        let duration = self.start.elapsed();
        histogram.observe(duration.as_secs_f64());
    }
    
    /// Stop timer and record to specific histogram with labels
    pub fn stop_and_record_histogram_vec(self, histogram: &HistogramVec, label_values: &[&str]) {
        let duration = self.start.elapsed();
        histogram.with_label_values(label_values).observe(duration.as_secs_f64());
    }
}

/// Get metrics as Prometheus text format
pub fn get_metrics_as_text() -> String {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

/// Update system metrics (memory, CPU, disk)
pub fn update_system_metrics() {
    // In a real implementation, you would collect actual system metrics
    // For this example, we'll just set placeholder values
    
    // Update memory usage
    SYSTEM_MEMORY_BYTES.set(100_000_000); // 100 MB for example
    
    // Update CPU usage
    SYSTEM_CPU_USAGE.set(0.5); // 50% usage for example
    
    // Update disk usage
    SYSTEM_DISK_BYTES.set(1_000_000_000); // 1 GB for example
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_timer() {
        let timer = MetricsTimer::new("test_timer");
        // Simulate work
        std::thread::sleep(Duration::from_millis(10));
        timer.stop_and_record_histogram(&TX_PROCESSING_TIME);
        
        // Check value was recorded (should be at least 0.01 seconds)
        assert!(TX_PROCESSING_TIME.get_sample_sum() > 0.0);
    }
    
    #[test]
    fn test_counter_increment() {
        // Reset counter
        let counter = &BLOCK_TXS.with_label_values(&["succeeded"]);
        
        // Get initial value
        let initial = counter.get();
        
        // Increment counter
        counter.inc();
        
        // Check counter was incremented
        assert_eq!(counter.get(), initial + 1);
    }
} 