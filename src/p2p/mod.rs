// P2P networking module

// libp2p integration
pub mod libp2p;

// Protocol definitions and handlers
pub mod protocol;

// Light client support
pub mod light;

// Main network service that ties everything together
pub mod service;

pub use protocol::*;
