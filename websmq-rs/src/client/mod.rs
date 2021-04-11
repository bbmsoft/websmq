use crate::{deserialize_message, serialize_message, Message};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use log::*;
use std::{fmt::Display, io};
use tokio::{
    net::TcpStream,
    sync::mpsc::{self, error::SendError, Receiver, Sender},
};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

#[derive(Debug)]
pub enum ClientError {
    IoError(io::Error),
    UrlParseError(url::ParseError),
    WsError(tungstenite::Error),
}

impl From<io::Error> for ClientError {
    fn from(e: io::Error) -> Self {
        ClientError::IoError(e)
    }
}

impl From<url::ParseError> for ClientError {
    fn from(e: url::ParseError) -> Self {
        ClientError::UrlParseError(e)
    }
}

impl From<tungstenite::Error> for ClientError {
    fn from(e: tungstenite::Error) -> Self {
        ClientError::WsError(e)
    }
}

impl Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::IoError(e) => e.fmt(f),
            ClientError::UrlParseError(e) => e.fmt(f),
            ClientError::WsError(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for ClientError {}

pub struct Client {
    write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::Message>,
}

impl Client {
    pub async fn publish(&mut self, topic: String, payload: Vec<u8>) -> Result<(), ClientError> {
        let msg = Message::Publish(topic, payload);
        let payload = serialize_message(&msg);
        let ws_message = tungstenite::Message::Binary(payload);
        // TODO should this be optimized to not flush after each message?
        self.write.send(ws_message).await?;
        Ok(())
    }

    pub async fn subscribe(&mut self, topic: String) -> Result<(), ClientError> {
        let msg = Message::Subscribe(topic);
        let payload = serialize_message(&msg);
        let ws_message = tungstenite::Message::Binary(payload);
        // TODO should this be optimized to not flush after each message?
        self.write.send(ws_message).await?;
        Ok(())
    }

    pub async fn unsubscribe(&mut self, topic: String) -> Result<(), ClientError> {
        let msg = Message::Unsubscribe(topic);
        let payload = serialize_message(&msg);
        let ws_message = tungstenite::Message::Binary(payload);
        // TODO should this be optimized to not flush after each message?
        self.write.send(ws_message).await?;
        Ok(())
    }

    pub async fn set_last_will(
        &mut self,
        topic: String,
        payload: Vec<u8>,
    ) -> Result<(), ClientError> {
        let msg = Message::LastWill(topic, payload);
        let payload = serialize_message(&msg);
        let ws_message = tungstenite::Message::Binary(payload);
        // TODO should this be optimized to not flush after each message?
        self.write.send(ws_message).await?;
        Ok(())
    }
}

pub async fn start_client(
    addr: impl AsRef<str>,
) -> Result<(Client, Receiver<Message>), ClientError> {
    let url = url::Url::parse(addr.as_ref())?;
    let (ws_stream, _) = connect_async(url).await?;
    let (write, read) = ws_stream.split();

    let client = Client { write };
    let (tx, rx) = mpsc::channel(1_000);

    tokio::spawn(process_raw_messages(tx, read));

    Ok((client, rx))
}

async fn process_raw_messages(
    tx: Sender<Message>,
    mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                if let Err(e) = process_raw_message(msg, tx.clone()).await {
                    error!("Error forwarding ws message to client: {}", e);
                    break;
                }
            }
            Err(e) => {
                error!("Error receiving ws message: {}", e);
                break;
            }
        }
    }
}

async fn process_raw_message(
    msg: tungstenite::Message,
    tx: Sender<Message>,
) -> Result<(), SendError<Message>> {
    match msg {
        tungstenite::Message::Text(text) => {
            info!("Received text message: {:?}", text);
            // TODO
        }
        tungstenite::Message::Binary(data) => {
            info!("Received binary message: {:?}", data);
            process_binary_message(data, tx).await?;
        }
        tungstenite::Message::Ping(data) => {
            info!("Received ping message: {:?}", data);
            // TODO
        }
        tungstenite::Message::Pong(data) => {
            info!("Received pong message: {:?}", data);
            // TODO
        }
        tungstenite::Message::Close(reason) => {
            info!("Received close message: {:?}", reason);
            // TODO
        }
    }
    Ok(())
}

async fn process_binary_message(
    buffer: Vec<u8>,
    tx: Sender<Message>,
) -> Result<(), SendError<Message>> {
    match deserialize_message(&buffer) {
        Ok(msg) => tx.send(msg).await?,
        Err(e) => error!("Received malformed binary message: {}", e),
    }
    Ok(())
}
