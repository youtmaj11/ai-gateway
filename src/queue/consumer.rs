use futures_util::stream::StreamExt;
use lapin::{options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions}, message::Delivery, types::FieldTable, Channel, Connection, ConnectionProperties, Consumer};
use crate::queue::QueueError;

pub struct RabbitConsumer {
    _connection: Connection,
    channel: Channel,
    consumer: Consumer,
}

impl RabbitConsumer {
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

        let consumer = channel
            .basic_consume(
                Self::QUEUE_NAME,
                "ai_gateway_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(Self { _connection: connection, channel, consumer })
    }

    pub async fn connect_from_config(config_url: &str) -> Result<Self, QueueError> {
        Self::connect(config_url).await
    }

    pub async fn receive_task(&mut self) -> Option<Result<Delivery, QueueError>> {
        self.consumer.next().await.map(|delivery| delivery.map_err(QueueError::from))
    }

    pub async fn acknowledge(&self, delivery: Delivery) -> Result<(), QueueError> {
        delivery.ack(BasicAckOptions::default()).await?;
        Ok(())
    }
}
