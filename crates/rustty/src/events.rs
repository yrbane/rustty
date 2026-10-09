//! Les réveils envoyés au thread d'interface par les threads lecteurs et le
//! veilleur de configuration.

use std::sync::Arc;

use crate::tab::TermId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UserEvent {
    TermUpdated(TermId),
    TermTitle(TermId, String),
    TermBell(TermId),
    TermModes(TermId),
    SetClipboard(String),
    PtyEof(TermId),
    ConfigChanged,
}

/// Ce qu'un thread tient pour réveiller l'interface.
pub trait Wake: Send + Sync + 'static {
    fn wake(&self, event: UserEvent);
}

impl Wake for winit::event_loop::EventLoopProxy<UserEvent> {
    fn wake(&self, event: UserEvent) {
        // Boucle fermée : l'application s'arrête, l'événement n'a plus d'importance.
        let _ = self.send_event(event);
    }
}

impl Wake for std::sync::mpsc::Sender<UserEvent> {
    fn wake(&self, event: UserEvent) {
        let _ = self.send(event);
    }
}

pub type Waker = Arc<dyn Wake>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_channel_is_a_waker() {
        let (tx, rx) = std::sync::mpsc::channel();
        let waker: Arc<dyn Wake> = Arc::new(tx);
        waker.wake(UserEvent::ConfigChanged);
        assert_eq!(rx.recv().unwrap(), UserEvent::ConfigChanged);
    }
}
