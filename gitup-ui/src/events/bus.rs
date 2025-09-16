use std::sync::mpsc::{channel, Sender, Receiver};
use crate::events::types::GraphEvent;

pub struct EventBus {
    sender: Sender<GraphEvent>,
    receiver: Receiver<GraphEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self { sender: tx, receiver: rx }
    }
    pub fn sender(&self) -> Sender<GraphEvent> { self.sender.clone() }
    pub fn try_recv(&self) -> Option<GraphEvent> { self.receiver.try_recv().ok() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bus_basic() {
        let bus = EventBus::new();
        bus.sender().send(GraphEvent::WorkingTreeChanged).unwrap();
        assert!(matches!(bus.try_recv(), Some(GraphEvent::WorkingTreeChanged)));
    }
}

