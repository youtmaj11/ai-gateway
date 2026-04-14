use async_trait::async_trait;
use std::fmt;

pub mod consumer;
pub mod producer;
pub mod in_memory;

pub use in_memory::InMemoryQueue;
use consumer::RabbitConsumer;
use producer::RabbitProducer;
use crate::config::{Config, QueueBackend};

#[derive(Debug)]
pub enum QueueError {
    Lapin(lapin::Error),
    ChannelClosed,
    InvalidMessage,
    MissingUrl,
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueueError::Lapin(err) => write!(f, "RabbitMQ error: {err}"),
            QueueError::ChannelClosed => write!(f, "queue channel closed"),
            QueueError::InvalidMessage => write!(f, "invalid queue message payload"),
            QueueError::MissingUrl => write!(f, "missing RabbitMQ URL"),
        }
    }
}

impl std::error::Error for QueueError {}

impl From<lapin::Error> for QueueError {
    fn from(error: lapin::Error) -> Self {
        QueueError::Lapin(error)
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for QueueError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        QueueError::ChannelClosed
    }
}

#[async_trait]
pub trait Queue: Send + Sync {
    async fn enqueue(&self, task: String) -> Result<(), QueueError>;
    async fn dequeue(&self) -> Result<Option<String>, QueueError>;
}

pub struct RabbitQueue {
    producer: RabbitProducer,
    consumer: tokio::sync::Mutex<RabbitConsumer>,
}

impl RabbitQueue {
    pub async fn new(url: &str) -> Result<Self, QueueError> {
        if url.is_empty() {
            return Err(QueueError::MissingUrl);
        }

        let producer = RabbitProducer::connect(url).await?;
        let consumer = RabbitConsumer::connect(url).await?;

        Ok(Self {
            producer,
            consumer: tokio::sync::Mutex::new(consumer),
        })
    }
}

#[async_trait]
impl Queue for RabbitQueue {
    async fn enqueue(&self, task: String) -> Result<(), QueueError> {
        self.producer.publish_task(&task).await
    }

    async fn dequeue(&self) -> Result<Option<String>, QueueError> {
        let mut consumer = self.consumer.lock().await;

        if let Some(result) = consumer.receive_task().await {
            let delivery = result?;
            let payload = String::from_utf8(delivery.data.clone())
                .map_err(|_| QueueError::InvalidMessage)?;
            consumer.acknowledge(delivery).await?;
            Ok(Some(payload))
        } else {
            Ok(None)
        }
    }
}

pub async fn create_queue(config: &Config) -> Result<Box<dyn Queue>, QueueError> {
    match config.queue_backend {
        QueueBackend::InMemory => Ok(Box::new(InMemoryQueue::new())),
        QueueBackend::RabbitMQ => Ok(Box::new(RabbitQueue::new(&config.rabbitmq_url).await?)),
    }
}
