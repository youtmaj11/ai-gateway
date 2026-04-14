use async_trait::async_trait;
use tokio::sync::{mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender}, Mutex};

use crate::queue::{Queue, QueueError};

pub struct InMemoryQueue {
    sender: UnboundedSender<String>,
    receiver: Mutex<UnboundedReceiver<String>>,
}

impl InMemoryQueue {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded_channel();
        Self { sender, receiver: Mutex::new(receiver) }
    }
}

#[async_trait]
impl Queue for InMemoryQueue {
    async fn enqueue(&self, task: String) -> Result<(), QueueError> {
        self.sender.send(task).map_err(|err| err.into())
    }

    async fn dequeue(&self) -> Result<Option<String>, QueueError> {
        let mut receiver = self.receiver.lock().await;
        Ok(receiver.recv().await)
    }
}
