use std::collections::HashMap;

// Can be a simple HashMap<string,string> for now
pub struct StateMachine {
    state: HashMap<String, String>,
}

impl StateMachine {
    pub fn set(&mut self, key: String, value: String) {
        self.state.insert(key, value);
    }
}

// e.g. SET x -> 1
#[derive(Clone)]
pub struct SetCommand {
    key: String,
    value: String,
}

impl SetCommand {
    // sets key to value, updates if exists else creates new
    pub fn execute(&self, state_machine: &mut StateMachine) {
        state_machine.set(self.key.clone(), self.value.clone())
    }
}

#[derive(Clone)]
pub struct LogItem {
    pub command: SetCommand,
    pub term: u64,
    pub index: u64,
}

pub struct Log {
    pub items: Vec<LogItem>,
}
