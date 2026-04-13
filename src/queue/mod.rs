// RabbitMQ producer and consumer interfaces

pub mod consumer;
pub mod producer;

pub use consumer::RabbitConsumer;
pub use producer::RabbitProducer;

pub fn initialize() {
    // placeholder for queue setup
}
