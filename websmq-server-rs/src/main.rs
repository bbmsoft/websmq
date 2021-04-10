pub mod protocol;

use futures_util::StreamExt;
use log::*;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use tokio::net::ToSocketAddrs;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;

#[tokio::main]
async fn main() {}
