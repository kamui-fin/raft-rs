// e.g. SET x -> 1
#[derive(Clone, Debug)]
pub struct SetCommand {
    pub key: String,
    pub value: String,
}

// Entry in the log.
// Each entry is uniquely identified by its term and index.
#[derive(Clone, Debug)]
pub struct LogItem {
    pub command: SetCommand,
    pub term: u64,
    pub index: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Log {
    pub items: Vec<LogItem>,
}
