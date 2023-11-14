use std::sync::mpsc::Sender;

use crate::log::SetCommand;

pub struct Client {
    sender: Sender<SetCommand>,
}

impl Client {
    pub fn new(sender: Sender<SetCommand>) -> Self {
        Self { sender }
    }
}
