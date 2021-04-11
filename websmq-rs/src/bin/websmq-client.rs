use log::*;
use std::env;
use std::{io::BufRead, thread};
use tokio::sync::mpsc::{self, Receiver};
use websmq_rs::{start_client, Message};

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = env::args().skip(1).next().unwrap();

    let (mut client, rx) = start_client(addr)
        .await
        .expect("Failed to connect to server!");

    tokio::spawn(receive_messages(rx));

    for arg in env::args().skip(2) {
        client.subscribe(arg).await.expect("failed to subscribe");
    }

    let (tx, mut rx) = mpsc::unbounded_channel();

    thread::spawn(move || {
        let stdin = std::io::stdin();
        let stdin = stdin.lock();
        for line in stdin.lines() {
            if let Ok(line) = line {
                if !line.trim().is_empty() {
                    let split: Vec<&str> = line.split(":").collect();
                    if split.len() == 2 {
                        let topic = split[0].trim().to_owned();
                        let message = split[1].trim().to_owned();
                        if let Err(e) = tx.send((topic, message)) {
                            error!("Failed to send user input: {}", e);
                            break;
                        }
                    } else {
                        eprintln!("Input must have the form <topic>:<message>");
                    }
                }
            } else {
                break;
            }
        }
    });

    while let Some(msg) = rx.recv().await {
        let topic = msg.0;
        let payload = encode_payload(&msg.1);
        client
            .publish(topic, payload)
            .await
            .expect("failed to send message");
    }

    info!("Exiting.");
}

async fn receive_messages(mut rx: Receiver<Message>) {
    while let Some(msg) = rx.recv().await {
        match msg {
            Message::Publish(topic, payload) => {
                println!("{}: {}", topic, decode_payload(&payload))
            }
            Message::Subscribe(topic) => info!("Received 'Subscribe' message to topic '{}'", topic),
            Message::Unsubscribe(topic) => {
                info!("Received 'Unsubscribe' message from topic '{}'", topic)
            }
            Message::LastWill(topic, payload) => {
                println!("{}: {}", topic, decode_payload(&payload))
            }
        }
    }
}

fn decode_payload(data: &[u8]) -> String {
    String::from_utf8_lossy(data).to_string()
}

fn encode_payload(msg: &str) -> Vec<u8> {
    msg.as_bytes().to_owned()
}
