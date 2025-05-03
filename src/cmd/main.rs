use std::env;
use std::process;

fn main() {
    println!("World Ledger blockchain node starting...");
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }
    
    // Process commands
    match args[1].as_str() {
        "node" => start_node(),
        "validator" => start_validator(),
        "account" => handle_account_command(&args),
        "version" => print_version(),
        "help" => print_usage(),
        _ => {
            println!("Unknown command: {}", args[1]);
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    println!("Usage: world-ledger <command> [options]");
    println!("Commands:");
    println!("  node             Start a full node");
    println!("  validator        Start a validator node");
    println!("  account <cmd>    Manage accounts");
    println!("  version          Print version information");
    println!("  help             Print this help message");
}

fn print_version() {
    println!("World Ledger v0.1.0");
    println!("A simple, scalable, resilient blockchain base layer");
}

fn start_node() {
    println!("Starting full node...");
    // TODO: Initialize and start a full node
    // For now, this is just a placeholder
}

fn start_validator() {
    println!("Starting validator node...");
    // TODO: Initialize and start a validator node
    // For now, this is just a placeholder
}

fn handle_account_command(args: &[String]) {
    if args.len() < 3 {
        println!("Usage: world-ledger account <create|list|import|export>");
        process::exit(1);
    }
    
    match args[2].as_str() {
        "create" => create_account(),
        "list" => list_accounts(),
        "import" => import_account(),
        "export" => export_account(),
        _ => {
            println!("Unknown account command: {}", args[2]);
            process::exit(1);
        }
    }
}

fn create_account() {
    println!("Creating new account...");
    // TODO: Generate keypair and create account
    // For now, this is just a placeholder
    
    // Generate a keypair using our dilithium implementation
    let (public_key, _private_key) = world_ledger::core::crypto::dilithium::generate_keypair();
    
    println!("New account created with public key: {:?}", &public_key[0..32]);
}

fn list_accounts() {
    println!("Listing accounts...");
    // TODO: List all accounts
    // For now, this is just a placeholder
}

fn import_account() {
    println!("Importing account...");
    // TODO: Import account from file
    // For now, this is just a placeholder
}

fn export_account() {
    println!("Exporting account...");
    // TODO: Export account to file
    // For now, this is just a placeholder
}
