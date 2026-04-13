use futures_util::stream::StreamExt;
use lapin::{options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions}, message::Delivery, types::FieldTable, Channel, Connection, ConnectionProperties, Consumer};
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

    pub async fn receive_task(&mut self) -> Option<Result<Delivery, QueueError>> {
        self.consumer.next().await.map(|delivery| delivery.map_err(QueueError::from))
    }

    pub async fn acknowledge(&self, delivery: Delivery) -> Result<(), QueueError> {
        delivery.ack(BasicAckOptions::default()).await?;
        Ok(())
    }
}
