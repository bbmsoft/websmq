use log::*;
use std::{io::BufRead, thread};
use tokio::sync::oneshot;
use websmq_rs::start_server;

#[tokio::main]
async fn main() {
    env_logger::init();

    if let Err(e) = start_server("0.0.0.0:8080").await {
        error!("Error starting server: {}", e);
    }

    let (tx, rx) = oneshot::channel();

    thread::spawn(|| {
        let stdin = std::io::stdin();
        let stdin = stdin.lock();
        for _ in stdin.lines() {
            // we don't care about input, we just want to keep the server from dropping until the application is closed
        }
        if let Err(_) = tx.send(()) {
            error!("Failed to send EOF");
        }
    });

    let _ = rx.await;

    info!("Exiting.");
}
