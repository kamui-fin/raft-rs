use serde_derive::{Deserialize, Serialize};

// e.g. SET x -> 1
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetCommand {
    pub key: String,
    pub value: String,
}

// Entry in the log.
// Each entry is uniquely identified by its term and index.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogItem {
    pub command: SetCommand,
    pub term: u64,
    pub index: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Log {
    pub items: Vec<LogItem>,
}
