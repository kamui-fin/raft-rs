// Barebones Raft implementation in Rust
// - Raft is a consenus algorithm to sync state machines across a distributed cluster.
// Aim for high test coverage, and for easier testing initially:
// - Develop without any networking (i.e. struct Server)
// - In-memory state machine instead of persistent store
// - Handle InstallSnapshot later on during optimization phase

use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};

// Can be a simple HashMap<string,string> for now
struct StateMachine {
    state: HashMap<String, String>,
}

impl StateMachine {
    fn set(&mut self, key: String, value: String) {
        self.state.insert(key, value);
    }
}

enum ServerStatus {
    Follower,
    Candidate,
    Leader { state: LeaderState },
}

struct Server<'s> {
    id: u64,
    server_state: ServerState,
    state_machine: StateMachine,
    // log entries; each entry contains command for state machine, and term when entry was received by leader (first index is 1)
    log: Log,
    status: ServerStatus,
    available: bool,
    // references to other servers
    servers: &'s Vec<&'s Server<'s>>,
}

impl Server<'_> {
    // fn handle_request_vote_rpc(&mut self, args: RequestVoteRPC) -> RequestVoteResult {}
    fn handle_append_entries_rpc(&mut self, args: AppendEntriesRPC) -> AppendEntriesResult {
        let fail = AppendEntriesResult {
            term: self.server_state.current_term,
            success: false,
        };

        // 1. Reply false if term < currentTerm (§5.1)
        if args.term < self.server_state.current_term {
            return fail;
        }

        // 2. reply false if log doesn’t contain an entry at prevLogIndex whose term matches prevLogTerm (§5.3)
        if self
            .log
            .items
            .get(args.prev_log_index as usize)
            .unwrap()
            .term
            != args.prev_log_term
        {
            return fail;
        }

        // 3. If an existing entry conflicts with a new one (same index but different terms),
        // delete the existing entry and all that follow it (§5.3)

        // 4. Append any new entries not already in the log

        // 5. If leaderCommit > commitIndex, set commitIndex = min(leaderCommit, index of last new entry)

        return fail;
    }
}

struct ServerPool<'s> {
    // master list
    servers: Vec<&'s Server<'s>>,
    // pointer into servers, O(1) leader assignments
    leader: Option<&'s Server<'s>>,
}

struct ServerState {
    // Updated on stable storage before responding to RPCs
    // latest term server has seen (initialized to 0 on first boot, increases monotonically)
    current_term: u64,
    // candidateId that received vote in current term (or null if none)
    voted_for: u64,

    // Volatile state
    // index of highest log entry known to be committed (initialized to 0, increases monotonically)
    commit_index: u64,
    // index of highest log entry applied to state machine (initialized to 0, increases monotonically)
    last_applied: u64,
}

// Reinitialized after election
// Volatile state
struct LeaderState {
    // for each server, index of the next log entry to send to that server (initialized to leader last log index + 1)
    next_index: Vec<u64>,
    // for each server, index of highest log entry known to be replicated on server (initialized to 0, increases monotonically)
    match_index: Vec<u64>,
}

trait Rpc<T> {
    type Output;

    // ran on the follower (receiver) side
    fn execute_rpc(&self, args: T, dest_id: u64) -> Self::Output;
}

impl Rpc<RequestVoteRPC> for ServerPool<'_> {
    type Output = Option<()>;

    fn execute_rpc(&self, args: RequestVoteRPC, dest_id: u64) -> Self::Output {
        if let Some(server) = self.servers.iter().find(|item| item.id == dest_id) {
            // Some(server.handle_request_vote_rpc())
            Some(())
        } else {
            None
        }
    }
}

impl Rpc<AppendEntriesRPC> for ServerPool<'_> {
    type Output = Option<()>;

    fn execute_rpc(&self, args: AppendEntriesRPC, dest_id: u64) -> Self::Output {
        if let Some(server) = self.servers.iter().find(|item| item.id == dest_id) {
            // Some(server.handle_append_entries_rpc())
            Some(())
        } else {
            None
        }
    }
}

// Can only be called by candidate state
// TODO: Add RPC validation
struct RequestVoteRPC {
    // candidate's term
    term: u64,
    // candidate requesting vote
    candidate_id: u64,
    // index of candidate’s last log entry (5.4)
    last_log_index: u64,
    // term of candidate’s last log entry (5.4)
    last_log_term: u64,
}

struct RequestVoteResult {
    // currentTerm, for candidate to update itself
    term: u64,
    // true means candidate received vote
    vote_granted: bool,
}

struct AppendEntriesRPC {
    // leader’s term
    term: u64,
    // so follower can redirect clients
    leader_id: u64,
    // index of log entry immediately preceding new ones
    prev_log_index: u64,
    // term of prevLogIndex entry
    prev_log_term: u64,
    //log entries to store (empty for heartbeat; may send more than one for efficiency)
    log: Log,
    // leader’s commitIndex
    leader_commit: u64,
}

struct AppendEntriesResult {
    // currentTerm, for candidate to update itself
    term: u64,
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

impl SetCommand {
    // sets key to value, updates if exists else creates new
    fn execute(&self, state_machine: &mut StateMachine) {
        state_machine.set(self.key.clone(), self.value.clone())
    }
}

struct LogItem {
    command: SetCommand,
    term: u64,
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
    // mpsc for client requests
    let (sender, receiver) = channel::<SetCommand>();
    let client = Client::new(sender.clone());
}
