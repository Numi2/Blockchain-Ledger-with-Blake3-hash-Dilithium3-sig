// Benchmarking for state transitions to ensure optimal performance

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use world_ledger::core::state::utxo::{UTXOSet, UTXORef};
use world_ledger::types::{Address, Hash, Transaction, TxIn, TxOut};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::time::{Duration, Instant};

/// Generate a random hash
fn random_hash(rng: &mut ChaCha20Rng) -> Hash {
    let mut hash = [0u8; 32];
    rng.fill_bytes(&mut hash);
    hash
}

/// Generate a random address
fn random_address(rng: &mut ChaCha20Rng) -> Address {
    let mut addr = [0u8; 20];
    rng.fill_bytes(&mut addr);
    addr
}

/// Generate a random transaction
fn generate_transaction(rng: &mut ChaCha20Rng, utxo_refs: Vec<UTXORef>, num_outputs: usize) -> Transaction {
    let mut inputs = Vec::with_capacity(utxo_refs.len());
    let mut outputs = Vec::with_capacity(num_outputs);
    
    // Create inputs
    for utxo_ref in utxo_refs {
        inputs.push(TxIn {
            utxo_id: utxo_ref.to_id(),
        });
    }
    
    // Create outputs
    let value_per_output = 1000 / num_outputs as u64;
    for _ in 0..num_outputs {
        outputs.push(TxOut {
            value: value_per_output,
            recipient: random_address(rng),
        });
    }
    
    Transaction {
        from: random_address(rng),
        to: Some(random_address(rng)),
        value: value_per_output,
        gas_limit: 21000,
        gas_price: 1_000_000_000, // 1 gwei
        nonce: 0,
        data: Vec::new(),
        inputs,
        outputs,
    }
}

/// Create a UTXO set with initial UTXOs
fn create_utxo_set(num_utxos: usize) -> (UTXOSet, Vec<UTXORef>, Vec<Hash>) {
    let mut rng = ChaCha20Rng::seed_from_u64(12345);
    let mut utxo_set = UTXOSet::new();
    let mut utxo_refs = Vec::with_capacity(num_utxos);
    let mut utxo_ids = Vec::with_capacity(num_utxos);
    
    // Create initial UTXOs
    for i in 0..num_utxos {
        let tx_hash = random_hash(&mut rng);
        let owner = random_address(&mut rng);
        let index = i as u32;
        
        let utxo_ref = UTXORef {
            tx_hash,
            index,
        };
        
        let id = utxo_set.add_utxo(tx_hash, index, 1000, owner);
        
        utxo_refs.push(utxo_ref);
        utxo_ids.push(id);
    }
    
    (utxo_set, utxo_refs, utxo_ids)
}

/// Benchmark UTXO set creation
fn bench_utxo_set_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("utxo_set_creation");
    
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let (set, _, _) = create_utxo_set(size);
                black_box(set)
            });
        });
    }
    
    group.finish();
}

/// Benchmark transaction application
fn bench_transaction_application(c: &mut Criterion) {
    let mut group = c.benchmark_group("transaction_application");
    
    for tx_size in [1, 10, 100].iter() {
        for utxo_set_size in [1000, 10000].iter() {
            group.bench_with_input(
                BenchmarkId::new("tx_size", format!("{}_{}", tx_size, utxo_set_size)),
                &(*tx_size, *utxo_set_size),
                |b, &(tx_size, utxo_set_size)| {
                    // Create UTXO set
                    let (mut set, utxo_refs, _) = create_utxo_set(utxo_set_size);
                    
                    // Create a transaction using first tx_size UTXOs
                    let mut rng = ChaCha20Rng::seed_from_u64(12345);
                    let tx = generate_transaction(&mut rng, utxo_refs[0..tx_size].to_vec(), 2);
                    
                    // Benchmark applying the transaction
                    b.iter(|| {
                        let mut test_set = set.clone();
                        black_box(test_set.apply_transaction(&tx)).unwrap();
                    });
                },
            );
        }
    }
    
    group.finish();
}

/// Benchmark Merkle root computation
fn bench_merkle_root(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_root");
    
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            // Create UTXO set
            let (set, _, _) = create_utxo_set(size);
            
            // Benchmark Merkle root calculation
            b.iter(|| {
                black_box(set.merkle_root())
            });
        });
    }
    
    group.finish();
}

/// Benchmark Merkle proof generation and verification
fn bench_merkle_proof(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_proof");
    
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("generate", size), size, |b, &size| {
            // Create UTXO set
            let (set, _, utxo_ids) = create_utxo_set(size);
            
            // Use middle UTXO for proof
            let middle_idx = size / 2;
            let id = &utxo_ids[middle_idx];
            
            // Benchmark proof generation
            b.iter(|| {
                black_box(set.generate_proof(id))
            });
        });
        
        group.bench_with_input(BenchmarkId::new("verify", size), size, |b, &size| {
            // Create UTXO set
            let (set, _, utxo_ids) = create_utxo_set(size);
            
            // Use middle UTXO for proof
            let middle_idx = size / 2;
            let id = &utxo_ids[middle_idx];
            
            // Generate proof
            let proof = set.generate_proof(id).unwrap();
            
            // Benchmark proof verification
            b.iter(|| {
                black_box(proof.verify())
            });
        });
    }
    
    group.finish();
}

/// Manual performance testing
pub fn run_manual_benchmarks() {
    println!("Running manual performance tests for state transitions");
    
    // Test UTXO set creation
    println!("\nUTXO Set Creation:");
    for size in [1_000, 10_000, 100_000].iter() {
        let start = Instant::now();
        let (set, _, _) = create_utxo_set(*size);
        let duration = start.elapsed();
        
        println!("  {} UTXOs: {:.2?}", size, duration);
        
        // Test Merkle root computation
        let start = Instant::now();
        let _root = set.merkle_root();
        let duration = start.elapsed();
        
        println!("  Merkle root for {} UTXOs: {:.2?}", size, duration);
    }
    
    // Test transaction application
    println!("\nTransaction Application:");
    let (mut set, utxo_refs, _) = create_utxo_set(10_000);
    
    for tx_size in [1, 10, 100].iter() {
        let mut rng = ChaCha20Rng::seed_from_u64(12345);
        let tx = generate_transaction(&mut rng, utxo_refs[0..*tx_size].to_vec(), 2);
        
        let start = Instant::now();
        set.apply_transaction(&tx).unwrap();
        let duration = start.elapsed();
        
        println!("  {} inputs: {:.2?}", tx_size, duration);
    }
}

criterion_group!(
    name = utxo_benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = bench_utxo_set_creation, bench_transaction_application, bench_merkle_root, bench_merkle_proof
);
criterion_main!(utxo_benches);
