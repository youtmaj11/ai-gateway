use lapin::{options::{BasicPublishOptions, QueueDeclareOptions}, types::FieldTable, BasicProperties, Channel, Connection, ConnectionProperties};
use crate::queue::QueueError;

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

    pub async fn connect_from_config(config_url: &str) -> Result<Self, QueueError> {
        Self::connect(config_url).await
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
