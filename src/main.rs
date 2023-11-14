// Barebones Raft implementation in Rust
// - Raft is a consenus algorithm to sync state machines across a distributed cluster.
// Aim for high test coverage, and for easier testing initially:
// - Develop without any networking (i.e. struct Server)
// - In-memory state machine instead of persistent store
// - Handle InstallSnapshot later on during optimization phase

mod client;
mod log;
mod node;

use std::sync::mpsc::channel;

use client::Client;
use log::SetCommand;

fn main() {
    // mpsc for client requests
    let (sender, receiver) = channel::<SetCommand>();
    let client = Client::new(sender.clone());
}
