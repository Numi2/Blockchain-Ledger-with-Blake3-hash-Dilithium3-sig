# World Ledger Node Operator Guide

## Table of Contents
1. [Introduction](#introduction)
2. [System Requirements](#system-requirements)
3. [Installation](#installation)
4. [Configuration](#configuration)
5. [Running a Node](#running-a-node)
6. [Node Monitoring](#node-monitoring)
7. [Troubleshooting](#troubleshooting)
8. [Security Best Practices](#security-best-practices)
9. [Validator Operations](#validator-operations)
10. [Upgrading](#upgrading)

## Introduction

This guide will help you set up and operate a World Ledger node. The World Ledger blockchain is designed with simplicity, security, and quantum resistance in mind. Operating a node contributes to the network's decentralization and security.

## System Requirements

### Minimum Requirements
- CPU: 4 cores
- RAM: 8 GB
- Storage: 100 GB SSD
- Network: 10 Mbps broadband
- Operating System: Linux (Ubuntu 20.04+ recommended), macOS, or Windows 10+

### Recommended Requirements
- CPU: 8+ cores
- RAM: 16+ GB
- Storage: 500 GB+ SSD (NVMe preferred)
- Network: 100 Mbps+ broadband
- Operating System: Linux (Ubuntu 22.04 recommended)

### Validator Requirements
- Same as recommended, plus:
- Additional 8 GB RAM
- Hardware security module (HSM) recommended for key storage
- Redundant power and internet connections
- Regular backups

## Installation

### From Binary Releases

1. Download the latest release for your platform from GitHub:
   ```
   curl -LO https://github.com/world-ledger/releases/download/v0.1.0/world-ledger-v0.1.0-linux-x86_64.tar.gz
   ```

2. Extract the archive:
   ```
   tar -xzf world-ledger-v0.1.0-linux-x86_64.tar.gz
   ```

3. Move the binary to a location in your PATH:
   ```
   sudo mv world-ledger /usr/local/bin/
   ```

### Building from Source

1. Install Rust (if not already installed):
   ```
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Clone the repository:
   ```
   git clone https://github.com/world-ledger/world-ledger.git
   cd world-ledger
   ```

3. Build the binary:
   ```
   cargo build --release
   ```

4. The binary will be available at `target/release/world-ledger`

## Configuration

The node configuration is stored in a TOML file. You can create a new configuration with:

```
world-ledger config create
```

This will create a `config.toml` file in the data directory (`~/.world-ledger` by default).

### Important Configuration Options

```toml
[node]
# Node identity and network
identity = "node1"
data_dir = "/path/to/data"
log_level = "info"

[network]
# P2P network settings
listening_address = "0.0.0.0:30333"
public_address = "your-public-ip:30333"
bootstrap_nodes = [
  "/dns4/bootstrap-1.world-ledger.com/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp",
  "/dns4/bootstrap-2.world-ledger.com/tcp/30333/p2p/12D3KooWP2onT3zT3YGjkg6eTmqfBuX19JCvzRLZ5e9vTBYMqyJ3"
]
max_peers = 50

[consensus]
# Consensus settings
type = "pow" # or "pos" for Proof of Stake
mining_enabled = false # Set to true for mining nodes

[rpc]
# JSON-RPC API settings
enabled = true
address = "127.0.0.1:8545"
cors_domains = ["*"]
```

## Running a Node

### Starting a Full Node

```
world-ledger node start
```

### Starting a Mining Node

```
world-ledger node start --mining
```

### Starting a Validator Node (PoS)

```
world-ledger node start --validator --key /path/to/validator/key
```

### Running as a Service

For Linux, you can create a systemd service file:

```
sudo nano /etc/systemd/system/world-ledger.service
```

With the following content:

```
[Unit]
Description=World Ledger Node
After=network.target

[Service]
User=worldledger
Group=worldledger
ExecStart=/usr/local/bin/world-ledger node start
Restart=on-failure
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

Enable and start the service:

```
sudo systemctl enable world-ledger
sudo systemctl start world-ledger
```

## Node Monitoring

### Metrics

The node exposes Prometheus metrics on port 9615 by default. You can configure Prometheus to scrape these metrics.

Important metrics to monitor:
- `world_ledger_block_height` - Current block height
- `world_ledger_peer_count` - Number of connected peers
- `world_ledger_mempool_size` - Number of transactions in the mempool
- `world_ledger_system_memory_bytes` - Memory usage
- `world_ledger_system_cpu_usage` - CPU usage

### Health Checks

The node provides a health endpoint at `/health` on the RPC port. You can use this to check if the node is running properly:

```
curl http://localhost:8545/health
```

Example response:
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime": 12345,
  "peers": 25,
  "synced": true,
  "block_height": 1000000
}
```

### Logs

Logs are written to the data directory by default. You can view them with:

```
tail -f ~/.world-ledger/logs/node.log
```

## Troubleshooting

### Common Issues

#### Node Won't Start

Check the logs for errors:
```
tail ~/.world-ledger/logs/node.log
```

Ensure the data directory is writable:
```
ls -la ~/.world-ledger
```

#### Node Can't Connect to Peers

Check your firewall settings to ensure port 30333 is open.

Verify that your public address is correctly configured.

Try manually adding peers:
```
world-ledger node peer add /ip4/1.2.3.4/tcp/30333/p2p/PEER_ID
```

#### Node Is Syncing Slowly

Check your disk I/O performance:
```
sudo hdparm -t /dev/sda
```

Check network performance:
```
iperf3 -c iperf.worldledger.com
```

#### High CPU or Memory Usage

Check which process is using resources:
```
top -c
```

Consider increasing the `--pruning` value to reduce state size.

## Security Best Practices

### Network Security

1. Run the node behind a firewall, opening only necessary ports (30333 for P2P, 8545 for RPC)
2. Use a reverse proxy like Nginx for RPC access with SSL
3. Limit RPC access to trusted IP addresses
4. Consider using a VPN for remote management

### System Security

1. Keep the operating system updated with security patches
2. Use strong, unique passwords
3. Use SSH key authentication instead of password authentication
4. Disable root SSH access
5. Install and configure fail2ban to prevent brute force attacks

### Key Management

1. Store validator keys in a hardware security module (HSM) if possible
2. Use different keys for different purposes (validator, operator, withdrawal)
3. Keep backups of keys in secure, offline storage
4. Consider using a multi-signature setup for high-value operations

## Validator Operations

### Staking

To stake tokens and become a validator:

```
world-ledger validator stake --amount 32000 --key /path/to/validator/key
```

### Monitoring Validator Performance

Check your validator's performance:

```
world-ledger validator status
```

Example output:
```
Validator: 0x1234...5678
Status: Active
Stake: 32,000 WL
Uptime: 99.8%
Blocks Proposed: 123
Rewards Earned: 45.6 WL
Next Assignment: Slot 5432 (in 2h 15m)
```

### Exiting

To gracefully exit and withdraw your stake:

```
world-ledger validator exit --key /path/to/validator/key
```

Note: There is a cooldown period of 21 days before you can withdraw your stake.

## Upgrading

### Upgrading the Node Software

1. Download the new version
2. Stop the current node:
   ```
   sudo systemctl stop world-ledger
   ```
3. Replace the binary:
   ```
   sudo mv /path/to/new/world-ledger /usr/local/bin/
   ```
4. Restart the node:
   ```
   sudo systemctl start world-ledger
   ```

### Database Migrations

Database migrations happen automatically when you start the node with a new version. Make sure to back up your data directory before upgrading:

```
tar -czf world-ledger-backup-$(date +%Y%m%d).tar.gz ~/.world-ledger
``` 