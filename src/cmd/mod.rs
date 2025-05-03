pub mod node;
pub mod wallet;
pub mod tx;
pub mod config;
pub mod keys;
pub mod chain;
pub mod debug;

use clap::{Parser, Subcommand};
use serde::{Serialize, Deserialize};

/// World Ledger blockchain command line interface
#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Optional config file path
    #[clap(short, long, value_name = "FILE")]
    pub config: Option<String>,
    
    /// Verbose output
    #[clap(short, long)]
    pub verbose: bool,
    
    /// JSON output
    #[clap(long)]
    pub json: bool,
    
    /// Subcommand
    #[clap(subcommand)]
    pub command: Commands,
}

/// CLI subcommands
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the blockchain node
    #[clap(subcommand)]
    Node(node::NodeCommands),
    
    /// Wallet operations
    #[clap(subcommand)]
    Wallet(wallet::WalletCommands),
    
    /// Transaction operations
    #[clap(subcommand)]
    Tx(tx::TxCommands),
    
    /// Config operations
    #[clap(subcommand)]
    Config(config::ConfigCommands),
    
    /// Key management
    #[clap(subcommand)]
    Keys(keys::KeyCommands),
    
    /// Chain operations
    #[clap(subcommand)]
    Chain(chain::ChainCommands),
    
    /// Debug operations
    #[clap(subcommand)]
    Debug(debug::DebugCommands),
}

/// Run the CLI
pub fn run() -> Result<(), String> {
    let cli = Cli::parse();
    
    // Handle global options
    if cli.verbose {
        // Set up verbose logging
    }
    
    // Process command
    match cli.command {
        Commands::Node(cmd) => node::handle_command(cmd, cli.json),
        Commands::Wallet(cmd) => wallet::handle_command(cmd, cli.json),
        Commands::Tx(cmd) => tx::handle_command(cmd, cli.json),
        Commands::Config(cmd) => config::handle_command(cmd, cli.json),
        Commands::Keys(cmd) => keys::handle_command(cmd, cli.json),
        Commands::Chain(cmd) => chain::handle_command(cmd, cli.json),
        Commands::Debug(cmd) => debug::handle_command(cmd, cli.json),
    }
}

/// Common CLI response format
#[derive(Debug, Serialize, Deserialize)]
pub struct CliResponse<T> {
    /// Success or failure
    pub success: bool,
    
    /// Optional error message
    pub error: Option<String>,
    
    /// Result data
    pub result: Option<T>,
}
