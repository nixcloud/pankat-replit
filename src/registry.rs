use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone)]
pub struct PubSubRegistry {
    channels: Arc<Mutex<HashMap<String, HashSet<Sender<String>>>>>,
}

impl PubSubRegistry {
    /// Get the singleton instance of PubSubRegistry
    pub fn instance() -> &'static Self {
        static INSTANCE: OnceLock<PubSubRegistry> = OnceLock::new();
        INSTANCE.get_or_init(|| PubSubRegistry::new())
    }

    /// Create a new PubSubRegistry (private)
    fn new() -> Self {
        Self {
            channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a client as a receiver for a specific channel
    pub fn register_receiver(&self, channel: String) -> Receiver<String> {
        let (tx, rx) = mpsc::channel();
        let mut channels = self.channels.lock().unwrap();
        channels.entry(channel).or_default().insert(tx);
        rx
    }

    /// Register a client as a sender for a specific channel
    pub fn register_sender(&self, channel: String) -> Sender<String> {
        let registry = self.clone();
        let (tx, rx) = mpsc::channel();

        // Spawn a thread to listen for messages and broadcast them
        std::thread::spawn(move || {
            for message in rx {
                registry.broadcast(&channel, message);
            }
        });

        tx
    }

    /// Broadcast a message to all receivers of the channel
    fn broadcast(&self, channel: &str, message: String) {
        let channels = self.channels.lock().unwrap();
        if let Some(receivers) = channels.get(channel) {
            for receiver in receivers {
                let _ = receiver.send(message.clone());
            }
        }
    }
}
