// Barebones Raft implementation in Rust
// Aim for high test coverage, and for easier testing initially:
// - Develop without any networking (i.e. struct Server)
// - In-memory state machine instead of persistent store

use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::thread;

// Can be a simple HashMap<string,string> for now
struct StateMachine {
    state: HashMap<String, String>,
}

struct ServerState {
    // Updated on stable storage before responding to RPCs
    // latest term server has seen (initialized to 0 on first boot, increases monotonically)
    current_term: usize,
    // candidateId that received vote in current term (or null if none)
    voted_for: usize,
    // log entries; each entry contains command for state machine, and term when entry was received by leader (first index is 1)
    log: Log,

    // Volatile state
    // index of highest log entry known to be committed (initialized to 0, increases monotonically)
    commit_index: usize,
    // index of highest log entry applied to state machine (initialized to 0, increases monotonically)
    last_applied: usize,
}

// Reinitialized after election
// Volatile state
struct LeaderState {
    // for each server, index of the next log entry to send to that server (initialized to leader last log index + 1)
    next_index: Vec<usize>,
    // for each server, index of highest log entry known to be replicated on server (initialized to 0, increases monotonically)
    match_index: Vec<usize>,
}

trait Rpc {
    type Output;

    // &self contains arguments
    fn execute(&self) -> Self::Output;
}

struct RequestVoteRPC {
    // candidate's term
    term: usize,
    // candidate requesting vote
    candidate_id: usize,
    // index of candidate’s last log entry (5.4)
    last_log_index: usize,
    // term of candidate’s last log entry (5.4)
    last_log_term: usize,
}

struct RequestVoteResult {
    // currentTerm, for candidate to update itself
    term: usize,
    // true means candidate received vote
    vote_granted: bool,
}

struct AppendEntriesRPC {
    // leader’s term
    term: usize,
    // so follower can redirect clients
    leader_id: usize,
    // index of log entry immediately preceding new ones
    prev_log_index: usize,
    // term of prevLogIndex entry
    prev_log_term: usize,
    //log entries to store (empty for heartbeat; may send more than one for efficiency)
    log: Log,
    // leader’s commitIndex
    leader_commit: usize,
}

struct AppendEntriesResult {
    // currentTerm, for candidate to update itself
    term: usize,
    // true if follower contained entry matching prevLogIndex and prevLogTerm
    success: bool,
}

enum NodeState {
    Follower,
    Candidate,
    Leader,
}

// e.g. SET x -> 1
struct SetCommand {
    key: String,
    value: String,
}

struct LogItem {
    command: SetCommand,
    term: usize,
}

struct Log {
    items: Vec<LogItem>,
}

struct Client {
    sender: Sender<SetCommand>,
}

impl Client {
    fn new(sender: Sender<SetCommand>) -> Self {
        Self { sender }
    }
}

fn main() {
    let (sender, receiver) = channel::<SetCommand>();

    let client = Client::new(sender.clone());
}
