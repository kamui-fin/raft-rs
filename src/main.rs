// Barebones Raft implementation in Rust
// - Raft is a consenus algorithm to sync state machines across a distributed cluster.
// Aim for high test coverage, and for easier testing initially:
// - Develop without any networking (i.e. struct Server)
// - In-memory state machine instead of persistent store
// - Handle InstallSnapshot later on during optimization phase

use std::cmp::min;
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
    Leader,
}

struct Server {
    id: u64,
    state_machine: StateMachine,
    // log entries; each entry contains command for state machine, and term when entry was received by leader (first index is 1)
    log: Log,
    status: ServerStatus,
    available: bool,
    server_state: ServerState,
    // leader-only state
    leader_state: Option<LeaderState>,
}

impl Server {
    // fn handle_request_vote_rpc(&mut self, args: RequestVoteRPC) -> RequestVoteResult {}

    // This RPC call should be sent out in parallel
    //  -> Retry indefinitely
    //
    //  The leader maintains a nextIndex for each follower,
    //  which is the index of the next log entry the leader will
    //  send to that follower. When a leader first comes to power,
    //  it initializes all nextIndex values to the index just after the
    //  last one in its log (11 in Figure 7).
    fn send_append_entries_rpc(&self, dest: &mut Server) -> Option<AppendEntriesResult> {
        // must only be executed by Leader
        match &self.status {
            ServerStatus::Follower | ServerStatus::Candidate => {
                return None;
            }
            _ => {}
        };
        let leader_state = self.leader_state.as_ref().unwrap();
        let starting_from = leader_state.next_index.get(&dest.id).unwrap();
        let mut new_logs = vec![];
        new_logs.clone_from_slice(&self.log.items[(*starting_from as usize)..]);

        let mut rpc = AppendEntriesRPC {
            term: self.server_state.current_term,
            leader_id: self.id,
            leader_commit: self.server_state.commit_index,
            log: new_logs,
            prev_log_index: (starting_from - 1) as i64,
            prev_log_term: self.log.items[(*starting_from as usize) - 1].term,
        };
        let mut result = dest.handle_append_entries_rpc(rpc.clone());

        while !result.success {
            let prev_log_index = rpc.prev_log_index - 1;
            rpc = AppendEntriesRPC {
                prev_log_index,
                prev_log_term: self.log.items[prev_log_index as usize].term,
                ..rpc
            };
            result = dest.handle_append_entries_rpc(rpc.clone());
        }
        Some(result)
    }

    // follower side handling
    fn handle_append_entries_rpc(&mut self, args: AppendEntriesRPC) -> AppendEntriesResult {
        // TODO: Handle heartbeat
        let fail = AppendEntriesResult {
            term: self.server_state.current_term,
            success: false,
        };
        let success = AppendEntriesResult {
            term: self.server_state.current_term,
            success: true,
        };

        match &self.status {
            ServerStatus::Leader | ServerStatus::Candidate => {
                return fail;
            }
            _ => {}
        };

        // 1. Reply false if term < currentTerm (§5.1)
        if args.term < self.server_state.current_term {
            return fail;
        }

        // 2. reply false if log doesn’t contain an entry at prevLogIndex whose term matches prevLogTerm (§5.3)
        if args.prev_log_index > -1
            && self
                .log
                .items
                .get(args.prev_log_index as usize)
                .unwrap()
                .term
                != args.prev_log_term
        {
            return fail;
        }

        // 3. If an existing entry conflicts with a new one (same index but different terms), delete the existing entry and all that follow it (§5.3)

        for log in args.log.iter() {
            if let Some(existing_entry) = self.log.items.get(log.index as usize) {
                if existing_entry.term == log.term {
                    continue;
                } else {
                    self.log.items.drain(log.index as usize..);
                }
            }
            // 4. Append any new entries not already in the log
            self.log.items.push(log.clone());
        }

        // 5. If leaderCommit > commitIndex, set commitIndex = min(leaderCommit, index of last new entry)
        if args.leader_commit > self.server_state.commit_index {
            self.server_state.commit_index =
                min(args.leader_commit, args.log.last().unwrap().index);
        }

        success
    }
}

struct ServerPool<'s> {
    // master list
    servers: Vec<Server>,
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
    next_indices: Vec<u64>,
    // for each server, index of highest log entry known to be replicated on server (initialized to 0, increases monotonically)
    match_indices: Vec<u64>,
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

#[derive(Clone)]
struct AppendEntriesRPC {
    /// leader’s term
    term: u64,
    /// so follower can redirect clients
    leader_id: u64,
    /// index of log entry immediately preceding new ones
    prev_log_index: i64,
    /// term of prevLogIndex entry
    prev_log_term: u64,
    /// log entries to store (empty for heartbeat; may send more than one for efficiency)
    log: Vec<LogItem>,
    /// leader’s commitIndex
    leader_commit: u64,
}

struct AppendEntriesResult {
    /// currentTerm, for candidate to update itself
    term: u64,
    /// true if follower contained entry matching prevLogIndex and prevLogTerm
    success: bool,
}

// e.g. SET x -> 1
#[derive(Clone)]
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

#[derive(Clone)]
struct LogItem {
    command: SetCommand,
    term: u64,
    index: u64,
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
