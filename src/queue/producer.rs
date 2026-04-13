use lapin::{options::{BasicPublishOptions, QueueDeclareOptions}, types::FieldTable, BasicProperties, Channel, Connection, ConnectionProperties};
use std::fmt;

#[derive(Debug)]
pub enum QueueError {
    Lapin(lapin::Error),
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueueError::Lapin(err) => write!(f, "RabbitMQ error: {err}"),
        }
    }
}

impl std::error::Error for QueueError {}

impl From<lapin::Error> for QueueError {
    fn from(error: lapin::Error) -> Self {
        QueueError::Lapin(error)
    }
}

pub struct RabbitProducer {
    channel: Channel,
}

impl RabbitProducer {
    const QUEUE_NAME: &'static str = "agent_tasks";

    pub async fn connect(url: &str) -> Result<Self, QueueError> {
        let connection = Connection::connect(url, ConnectionProperties::default()).await?;
        let channel = connection.create_channel().await?;

        channel
            .queue_declare(
                Self::QUEUE_NAME,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(Self { channel })
    }

    pub async fn publish_task(&self, task: &str) -> Result<(), QueueError> {
        let confirmation = self
            .channel
            .basic_publish(
                "",
                Self::QUEUE_NAME,
                BasicPublishOptions::default(),
                task.as_bytes(),
                BasicProperties::default(),
            )
            .await?;

        confirmation.await?;
        Ok(())
    }
}
