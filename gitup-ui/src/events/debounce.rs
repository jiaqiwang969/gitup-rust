use std::time::{Duration, Instant};
use crate::events::types::GraphEvent;

pub struct EventDebouncer {
    pending: Option<GraphEvent>,
    last: Instant,
    window: Duration,
}

impl EventDebouncer {
    pub fn new(window: Duration) -> Self {
        Self { pending: None, last: Instant::now(), window }
    }
    pub fn add(&mut self, ev: GraphEvent) {
        self.pending = Some(ev);
        self.last = Instant::now();
    }
    pub fn take_if_ready(&mut self) -> Option<GraphEvent> {
        if self.pending.is_some() && self.last.elapsed() >= self.window {
            self.pending.take()
        } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    #[test]
    fn debounce_basic() {
        let mut d = EventDebouncer::new(Duration::from_millis(30));
        d.add(GraphEvent::RepositoryChanged);
        assert!(d.take_if_ready().is_none());
        thread::sleep(Duration::from_millis(35));
        assert!(matches!(d.take_if_ready(), Some(GraphEvent::RepositoryChanged)));
    }
}

