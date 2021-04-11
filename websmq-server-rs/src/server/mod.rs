mod data_structures;

use crate::{deserialize_message, serialize_message, Message};
use futures_util::{
    stream::{SplitSink, StreamExt},
    SinkExt,
};
use log::*;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    io::Result,
    net::SocketAddr,
    sync::Arc,
};
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
};
use tokio_tungstenite::WebSocketStream;

use self::data_structures::TopicTree;

#[derive(Debug, Clone)]
struct Client {
    sender: Sender<Message>,
    addr: SocketAddr,
}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.addr.eq(&other.addr)
    }
}

impl Eq for Client {}

impl Hash for Client {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.addr.hash(state)
    }
}

#[derive(Debug, Default)]
struct ClientRegistry {
    clients: HashMap<Client, HashSet<String>>,
    subscriptions: TopicTree<Client>,
}

impl ClientRegistry {
    async fn process_raw_message(
        &mut self,
        msg: tokio_tungstenite::tungstenite::Message,
        client: Client,
    ) {
        match msg {
            tungstenite::Message::Text(text) => {
                info!("Received text message: {:?}", text);
                // TODO
            }
            tungstenite::Message::Binary(data) => {
                info!("Received binary message: {:?}", data);
                self.process_binary_message(data, client).await;
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
                self.remove_client(client);
            }
        }
    }

    async fn process_binary_message(&mut self, buffer: Vec<u8>, client: Client) {
        match deserialize_message(&buffer) {
            Ok(msg) => self.process_message(msg, client).await,
            Err(e) => error!("Received malformed binary message: {}", e),
        }
    }

    async fn process_message(&mut self, msg: Message, client: Client) {
        match msg {
            Message::Publish(topic, payload) => self.publish(client, topic, payload).await,
            Message::Subscribe(topic) => self.subscribe(client, topic).await,
            Message::Unsubscribe(topic) => self.unsubscribe(client, topic).await,
            Message::LastWill(topic, payload) => {
                self.register_last_will(client, topic, payload).await
            }
        }
    }

    async fn publish(&mut self, _client: Client, topic: String, payload: Vec<u8>) {
        let path: Vec<String> = topic.split("/").map(|s| s.to_owned()).collect();
        let clients = self.subscriptions.get_matching_clients(&path);
        let mut offline_clients = Vec::new();
        for client in clients {
            let msg = Message::Publish(topic.clone(), payload.clone());
            if let Err(e) = client.sender.send(msg).await {
                error!("Error sending message to client: {}", e);
                offline_clients.push(client.clone());
            }
        }
        for client in offline_clients {
            self.remove_client(client);
        }
    }

    async fn subscribe(&mut self, client: Client, topic: String) {
        info!("Subscribing client {:?} to topic '{}'", client, topic);
        // TODO check if this topic is already covered by a wildcard topic
        let subscribed_topics = self.clients.get_mut(&client);
        let subscription_added = if let Some(topics) = subscribed_topics {
            topics.insert(topic.clone())
        } else {
            let mut topics = HashSet::new();
            topics.insert(topic.clone());
            self.clients.insert(client.clone(), topics);
            true
        };
        if subscription_added {
            let path: Vec<String> = topic.split("/").map(|s| s.to_owned()).collect();
            self.subscriptions.insert_client(&path, client);
        }
    }

    async fn unsubscribe(&mut self, client: Client, topic: String) {
        if let Some(topics) = self.clients.get_mut(&client) {
            topics.remove(&topic);
            if topics.is_empty() {
                self.clients.remove(&client);
            }
        }
        let path: Vec<String> = topic.split("/").map(|s| s.to_owned()).collect();
        self.subscriptions.remove_client(&path, &client);
    }

    async fn register_last_will(&mut self, client: Client, topic: String, payload: Vec<u8>) {
        // TODO
    }

    async fn send_last_will(&mut self, client: Client) {
        // TODO
    }

    fn remove_client(&mut self, client: Client) {
        if let Some(topics) = self.clients.remove(&client) {
            for topic in topics {
                let path: Vec<String> = topic.split("/").map(|s| s.to_owned()).collect();
                self.subscriptions.remove_client(&path, &client);
                info!(
                    "Unsubscribing client {} from topic '{}'",
                    client.addr, topic
                );
            }
        }
    }
}

pub async fn start_server<A: ToSocketAddrs>(addr: A) -> Result<()> {
    let socket = TcpListener::bind(&addr).await?;
    tokio::spawn(run(socket));
    Ok(())
}

async fn run(socket: TcpListener) {
    let registry = Arc::new(Mutex::new(ClientRegistry::default()));
    while let Ok((stream, _)) = socket.accept().await {
        let reg = registry.clone();
        tokio::spawn(accept_connection(reg, stream));
    }
}

async fn accept_connection(clients: Arc<Mutex<ClientRegistry>>, stream: TcpStream) {
    let addr = match stream.peer_addr() {
        Ok(addr) => addr,
        Err(e) => {
            error!("Could not get client's address: {}", e);
            return;
        }
    };

    info!("Accepting client connection from {} ...", addr);

    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(stream) => stream,
        Err(e) => {
            error!(
                "Could not establish web socket connection with client {}: {}",
                addr, e
            );
            return;
        }
    };

    info!("New client connected: {}", addr);

    let (write, mut read) = ws_stream.split();
    let (tx, rx) = mpsc::channel(1_000);

    let client = Client { sender: tx, addr };

    tokio::spawn(forward_messages_to_client(rx, write));

    while let Some(Ok(msg)) = read.next().await {
        clients
            .lock()
            .await
            .process_raw_message(msg, client.clone())
            .await;
    }

    let mut clients = clients.lock().await;
    clients.send_last_will(client.clone()).await;
    clients.remove_client(client);
}

async fn forward_messages_to_client(
    mut rx: Receiver<Message>,
    mut write: SplitSink<WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>,
) {
    while let Some(msg) = rx.recv().await {
        let data = serialize_message(&msg);
        let ws_message = tokio_tungstenite::tungstenite::Message::Binary(data);
        // TODO should this be optimized to not flush after each message?
        if let Err(e) = write.send(ws_message).await {
            error!("Error sending message to client: {}", e);
            break;
        }
    }
}
