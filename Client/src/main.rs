use std::error::Error;
use std::net::{SocketAddr};
use std::sync::Arc;
use bytes::Bytes;
use futures::{io, SinkExt, StreamExt};
use futures::stream::SplitSink;
use tokio::io::{stdin, BufReader, AsyncBufReadExt};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

struct Client {
    lines: SplitSink<Framed<TcpStream, LengthDelimitedCodec>, Bytes>,
    read_task: JoinHandle<()>,
}

impl Client {
    async fn new(addr: SocketAddr) -> Result<Client, io::Error> {
        let mut stream = match TcpStream::connect(addr).await {
            Ok(stream) => stream,
            Err(e) => return Err(e),
        };

        let lines = Framed::new(stream, LengthDelimitedCodec::new());

        let (r, mut w) = lines.split();

        let read_task = tokio::spawn(async move {
           while let Some(Ok(msg)) = w.next().await {
                if Some(msg.clone()).is_some() {
                    println!("Message Received {}", String::from_utf8_lossy(&msg).to_string());
                }
           }
        });

        Ok(Client { lines: r, read_task })

    }

    async fn send_message(&mut self, msg: &Bytes) {
        self.lines.send(msg.clone()).await;
    }


}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:1235".to_string());

    let addr = addr.parse::<SocketAddr>()?;

    let mut client = Client::new(addr).await?;

    let stdin = BufReader::new(stdin());
    let mut lines = stdin.lines();

    while let Some(Ok(line)) = lines.next_line().await.transpose() {
        let message = Bytes::from(line);
        client.send_message(&message).await;
    }

    Ok(())
}
