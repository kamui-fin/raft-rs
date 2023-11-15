use std::{cell::RefCell, cmp::min, collections::HashMap};

use crate::log::{Log, LogItem, SetCommand, StateMachine};

// Can only be called by candidate state
// TODO: Add RPC validation
pub struct RequestVoteRPC {
    // candidate's term
    pub term: u64,
    // candidate requesting vote
    pub candidate_id: u64,
    // index of candidate’s last log entry (5.4)
    pub last_log_index: u64,
    // term of candidate’s last log entry (5.4)
    pub last_log_term: u64,
}

pub struct RequestVoteResult {
    // currentTerm, for candidate to update itself
    pub term: u64,
    // true means candidate received vote
    pub vote_granted: bool,
}

#[derive(Clone)]
pub struct AppendEntriesRPC {
    /// leader’s term
    pub term: u64,
    /// so follower can redirect clients
    pub leader_id: u64,
    /// index of log entry immediately preceding new ones
    pub prev_log_index: i64,
    /// term of prevLogIndex entry
    pub prev_log_term: u64,
    /// log entries to store (empty for heartbeat; may send more than one for efficiency)
    pub log: Vec<LogItem>,
    /// leader’s commitIndex
    pub leader_commit: u64,
}

pub struct AppendEntriesResult {
    /// currentTerm, for candidate to update itself
    pub term: u64,
    /// true if follower contained entry matching prevLogIndex and prevLogTerm
    pub success: bool,
}

pub enum ServerStatus {
    Follower,
    Candidate,
    Leader,
}

pub struct Server {
    pub id: u64,
    pub state_machine: StateMachine,
    // log entries; each entry contains command for state machine, and term when entry was received by leader (first index is 1)
    pub log: Log,
    pub status: ServerStatus,
    pub available: bool,
    pub server_state: ServerState,
    // leader-only state
    pub leader_state: Option<LeaderState>,
}

pub struct ServerPool<'s> {
    // master list
    pub servers: Vec<Server>,
    // pointer into servers, O(1) leader assignments
    pub leader: Option<&'s Server>,
}

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
pub struct LeaderState {
    // for each server, index of the next log entry to send to that server (initialized to leader last log index + 1)
    pub next_indices: HashMap<u64, u64>,
    // for each server, index of highest log entry known to be replicated on server (initialized to 0, increases monotonically)
    pub match_indices: HashMap<u64, u64>,
}

enum ApplyResult {
    Applied,
    Unchanged,
}

impl Server {
    // If commitIndex > lastApplied: increment lastApplied, apply log[lastApplied] to state machine (§5.3)
    // TODO: Unsure where to call
    fn checked_apply(&mut self) -> ApplyResult {
        if self.server_state.commit_index > self.server_state.last_applied {
            self.server_state.last_applied += 1;
            let log = &self.log.items[self.server_state.last_applied as usize];
            log.command.execute(&mut self.state_machine);
            ApplyResult::Applied
        } else {
            ApplyResult::Unchanged
        }
    }

    fn append_command(&mut self, command: SetCommand) {
        /*
            - Client sends command to leader
            - Leader appends command to log
                - Then, sends out AppendEntries RPCs to followers
                    - Retry until success
            - After committed, leader executes command in state machine and returns to client
            - From here, each follower also executes the commands
        */

        self.log.items.push(LogItem {
            command,
            term: self.server_state.current_term,
            index: self.log.items.len() as u64,
        })

        // TODO:
        // If last log index ≥ nextIndex for a follower: send
        //    AppendEntries RPC with log entries starting at nextIndex
        //    • If successful: update nextIndex and matchIndex for
        //    follower (§5.3)
        // need Vec<Server> access here
    }

    // fn handle_request_vote_rpc(&mut self, args: RequestVoteRPC) -> RequestVoteResult {}

    // This RPC call should be sent out in parallel
    //  -> Retry indefinitely
    //
    //  The leader maintains a nextIndex for each follower,
    //  which is the index of the next log entry the leader will
    //  send to that follower. When a leader first comes to power,
    //  it initializes all nextIndex values to the index just after the
    //  last one in its log (11 in Figure 7).
    fn send_append_entries_rpc(&mut self, dest: &mut Server) -> Option<AppendEntriesResult> {
        // must only be executed by Leader
        match &self.status {
            ServerStatus::Follower | ServerStatus::Candidate => {
                return None;
            }
            _ => {}
        };
        let leader_state = self.leader_state.as_ref().unwrap();

        let starting_from = leader_state.next_indices.get(&dest.id).unwrap();
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

        // If RPC request or response contains term T > currentTerm: set currentTerm = T, convert to follower (§5.1)
        if result.term > self.server_state.current_term {
            self.server_state.current_term = result.term;
            self.status = ServerStatus::Follower;
        }

        // If there exists an N such that N > commitIndex, a majority
        // of matchIndex[i] ≥ N, and log[N].term == currentTerm:
        // set commitIndex = N (§5.3, §5.4).

        for n in (self.server_state.commit_index..self.log.items.len() as u64).rev() {
            let is_majority = leader_state
                .match_indices
                .keys()
                .map(|v| (v >= &n) as u64)
                .sum::<u64>()
                >= (leader_state.match_indices.len() / 2) as u64;

            if is_majority && self.log.items[n as usize].term == self.server_state.current_term {
                self.server_state.commit_index = n;
                break;
            }
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

        // If RPC request or response contains term T > currentTerm: set currentTerm = T, convert to follower (§5.1)
        if args.term > self.server_state.current_term {
            self.server_state.current_term = args.term;
            self.status = ServerStatus::Follower;
        }

        // This RPC can only be handled by followers
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

        if args.log.is_empty() {
            self.heartbeat();
            return success;
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

    fn heartbeat(&self) {
        // prevent election timeouts
        todo!()
    }
}
