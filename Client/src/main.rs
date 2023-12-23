use std::error::Error;
use std::net::{SocketAddr};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use futures::stream::{SplitSink, SplitStream};
use tokio::io::{stdin, BufReader, AsyncBufReadExt};
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};


async fn connect(addr: SocketAddr) -> Result<(), Box<dyn Error>> {

    let stream = TcpStream::connect(addr).await?;

    let mut lines = Framed::new(stream, LengthDelimitedCodec::new());

    let (r, w) = lines.split();

    tokio::try_join!(send_message(r), read_message(w));

    Ok(())
}


async fn send_message(mut r: SplitSink<Framed<TcpStream, LengthDelimitedCodec>, Bytes>) -> Result<(), Box<dyn Error>>{
    let stdin = BufReader::new(stdin());
    let mut stdin = stdin.lines();
    while let Some(Ok(line)) = stdin.next_line().await.transpose() {
        let message = Bytes::from(line);
        r.send(message).await?;
    }
    Ok(())
}


async fn read_message(mut w: SplitStream<Framed<TcpStream, LengthDelimitedCodec>>) -> Result<(), Box<dyn Error>>{
    while let Some(Ok(msg)) = w.next().await {
        if Some(msg.clone()).is_some() {
            println!("{}", String::from_utf8_lossy(&msg).to_string());
        }
    }
    Ok(())
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:1235".to_string());

    let addr = addr.parse::<SocketAddr>()?;

    connect(addr).await?;

    Ok(())
}
