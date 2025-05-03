pub mod pow;
pub mod pos;
pub mod types;

// Re-exports
pub use types::*;
pub use pow::*;
pub use pos::*;

/// Enum representing different consensus mechanisms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusType {
    /// Proof of Work
    ProofOfWork,
    
    /// Proof of Stake
    ProofOfStake,
}

impl ConsensusType {
    /// Get the name of the consensus mechanism
    pub fn name(&self) -> &'static str {
        match self {
            ConsensusType::ProofOfWork => "Proof of Work",
            ConsensusType::ProofOfStake => "Proof of Stake",
        }
    }
}

/// Consensus configuration
#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    /// Type of consensus mechanism
    pub consensus_type: ConsensusType,
    
    /// Difficulty adjustment window (for PoW)
    pub pow_difficulty_adjustment_window: u64,
    
    /// Target block time in seconds
    pub target_block_time: u64,
    
    /// Initial difficulty for PoW
    pub pow_initial_difficulty: u64,
    
    /// Minimum stake amount for validators (for PoS)
    pub pos_minimum_stake: u64,
    
    /// Maximum number of validators (for PoS)
    pub pos_max_validators: u32,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            consensus_type: ConsensusType::ProofOfWork,
            pow_difficulty_adjustment_window: 2016, // ~2 weeks at 10 min blocks
            target_block_time: 10 * 60,             // 10 minutes
            pow_initial_difficulty: 1,
            pos_minimum_stake: 32_000_000_000,      // 32 tokens in smallest unit
            pos_max_validators: 10_000,
        }
    }
} 