// Barebones Raft implementation in Rust
// Aim for high test coverage, and for easier testing initially:
// - Develop without any networking (i.e. struct Server)
// - In-memory state machine instead of persistent store

enum NodeState {
    Follower,
    Candidate,
    Leader,
}

enum RPCType {
    AppendEntries,
    RequestVote,
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

fn main() {
    unimplemented!()
}
