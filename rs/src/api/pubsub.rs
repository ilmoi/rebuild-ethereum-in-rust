use crate::blockchain::block::Block;
use crate::blockchain::blockchain::Blockchain;
use futures_util::stream::StreamExt;
use lapin::{
    options::*, publisher_confirm::Confirmation, types::FieldTable, BasicProperties, Channel,
    Connection, ConnectionProperties, ExchangeKind, Promise, Result,
};
use std::sync::{Arc, Mutex};

pub async fn rabbit_connect() -> Result<Connection> {
    let addr = std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://127.0.0.1:5672/%2f".into());
    let conn = Connection::connect(&addr, ConnectionProperties::default()).await?;
    println!("connected!");

    Ok(conn)
}

pub fn create_ex_if_doesnt_exist(channel: &Channel, exchange: &str) -> Promise<()> {
    channel.exchange_declare(
        exchange,
        ExchangeKind::Fanout, //important for blockchain to be blockchain
        ExchangeDeclareOptions::default(),
        FieldTable::default(),
    )
}

pub async fn rabbit_publish(payload: String, exchange: &str) -> Result<()> {
    let conn = rabbit_connect().await.unwrap();
    let channel_a = conn.create_channel().await?;
    let _ex = create_ex_if_doesnt_exist(&channel_a, exchange);

    let _confirm = channel_a
        .basic_publish(
            exchange, //subscribe tou our exchange
            "", //when using fanout, we don't need to specify routing_key -https://www.rabbitmq.com/tutorials/tutorial-three-python.html
            BasicPublishOptions::default(),
            payload.as_bytes().to_vec(),
            BasicProperties::default(),
        )
        .await?
        .await?;

    println!(">>> published payload: {:?}", &payload);
    Ok(())
}

pub async fn rabbit_consume(
    processor: fn(String, Arc<Mutex<Blockchain>>),
    blockchain: Arc<Mutex<Blockchain>>,
    exchange: &str,
) -> Result<()> {
    let conn = rabbit_connect().await.unwrap();
    let channel_b = conn.create_channel().await?;
    let _ex = create_ex_if_doesnt_exist(&channel_b, exchange); //needed in both, as sometimes this thread will run ahead of producer

    // create a tmp queue
    let q_opts = QueueDeclareOptions {
        exclusive: true,
        ..QueueDeclareOptions::default()
    };
    let queue = channel_b
        .queue_declare(
            "",     //when a name is not specified, a random name is given
            q_opts, //exclusive=true means q will be deleted after, which is what we want
            FieldTable::default(),
        )
        .await?;
    println!("declared a tmp queue: {}", &queue.name().to_string());

    // bind the tmp queue to the exchange, otherwise the exchange won't know to fanout msgs to this q
    channel_b.queue_bind(
        &queue.name().to_string(),
        exchange,
        "", //again no need to specify coz using fanout
        QueueBindOptions::default(),
        FieldTable::default(),
    );

    let mut consumer = channel_b
        .basic_consume(
            &queue.name().to_string(),
            "my_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    while let Some(delivery) = consumer.next().await {
        let (_channel, delivery) = delivery.expect("error in consumer");
        println!("<<< got delivery: {:?}", delivery);
        delivery.ack(BasicAckOptions::default()).await.expect("ack");

        //restore into string and send for processing
        let data = String::from_utf8(delivery.data).unwrap();
        processor(data, blockchain.clone());
    }

    Ok(())
}

pub fn process_block(block: String, blockchain: Arc<Mutex<Blockchain>>) {
    let block_object: Block = serde_json::from_str(&block).unwrap();
    println!("deserialized block: {:?}", block_object);

    let mut blockchain = blockchain.lock().unwrap();
    blockchain.add_block(block_object);
}
