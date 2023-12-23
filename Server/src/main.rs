use std::collections::HashMap;
use std::{env, io};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

type Tx = mpsc::UnboundedSender<Bytes>;

type Rx = mpsc::UnboundedReceiver<Bytes>;

#[derive(Clone, Debug)]
struct ChatServer {
    peers: HashMap<SocketAddr, Tx>
}

impl ChatServer {
    fn new() -> Self {
        ChatServer {
            peers: HashMap::new(),
        }
    }

    async fn broadcast(&mut self, sender: SocketAddr, message: &Bytes) {
        for peer in self.peers.iter_mut() {
            if *peer.0 != sender {
                let _ = peer.1.send(message.clone());
            }
        }
    }
}

struct Client {
    frame : Framed<TcpStream, LengthDelimitedCodec>,

    rx: Rx,
}

impl Client {
    async fn new(
        state: Arc<Mutex<ChatServer>>,
        framed: Framed<TcpStream, LengthDelimitedCodec>
    ) -> io::Result<Client> {
        let addr = framed.get_ref().peer_addr()?;

        let (tx, rx) = mpsc::unbounded_channel();

        state.lock().await.peers.insert(addr, tx);

        Ok(Client { frame: framed, rx})
    }
}


async fn get_username(frame: &mut Framed<TcpStream, LengthDelimitedCodec> ) -> String {
    let username_prompt = Bytes::from("What is your Username");
    frame.send(username_prompt).await;

    let username = match frame.next().await {
        Some(Ok(msg)) => String::from_utf8_lossy(&msg).to_string(),
        _ => "".to_string(),
    };
    username
}


async fn process(
    state: Arc<Mutex<ChatServer>>,
    stream: TcpStream,
    addr: SocketAddr,) -> Result<(), Box<dyn Error>> {

    let mut frame = Framed::new(stream, LengthDelimitedCodec::new());

    let username = get_username(&mut frame).await;

    let mut client = Client::new(state.clone(), frame).await?;

    {
        let mut state = state.lock().await;
        let msg = Bytes::from(format!("{} has joined the chat", username));
        state.broadcast(addr, &msg).await;
    }

    loop {
        tokio::select! {
            Some(msg) = client.rx.recv() => {
                client.frame.send(msg).await?;
            }
            result = client.frame.next() => match result {
                Some(Ok(msg)) => {
                    let mut state = state.lock().await;
                    let msg = Bytes::from(format!("{}: {}", username, String::from_utf8_lossy(msg.as_ref())));
                    state.broadcast(addr, &msg).await;
                }
                Some(Err(e)) => {
                    eprintln!("{}", e);
                }
                None => break,
            },
        }
    }

    {
        let mut state = state.lock().await;
        state.peers.remove(&addr);
        let msg = Bytes::from(format!("{} has left the chat ", username));
        state.broadcast(addr, &msg).await;
    }

    Ok(())
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let state = Arc::new(Mutex::new(ChatServer::new()));

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:1235".to_string());

    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, addr) = listener.accept().await?;
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = process(state, stream, addr).await {
                println!("Error {}", e);
            }
        });
    }
}
