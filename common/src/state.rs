use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerStatus {
    Follower,
    Candidate,
    Leader,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerState {
    // Updated on stable storage before responding to RPCs
    // latest term server has seen (initialized to 0 on first boot, increases monotonically)
    pub current_term: u64,
    // candidateId that received vote in current term (or null if none)
    pub voted_for: u64,

    // Volatile state
    // index of highest log entry known to be committed (initialized to 0, increases monotonically)
    pub commit_index: u64,
    // index of highest log entry applied to state machine (initialized to 0, increases monotonically)
    pub last_applied: u64,
}

// Reinitialized after election
// Volatile state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeaderState {
    // for each server, index of the next log entry to send to that server (initialized to leader last log index + 1)
    pub next_indices: HashMap<u64, u64>,
    // for each server, index of highest log entry known to be replicated on server (initialized to 0, increases monotonically)
    pub match_indices: HashMap<u64, u64>,
}
