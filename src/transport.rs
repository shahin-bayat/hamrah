use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::protocol::Message;

pub async fn write_msg<W: AsyncWriteExt + Unpin>(w: &mut W, msg: &Message) -> io::Result<()> {
    let framed = msg.encode();
    w.write_all(&framed).await
}

pub async fn read_msg<R: AsyncReadExt + Unpin>(r: &mut R) -> io::Result<Option<Message>> {
    let mut msg_len_buf = [0u8; 4];
    match r.read_exact(&mut msg_len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    };
    let msg_len = u32::from_be_bytes(msg_len_buf) as usize;

    // TODO: cap msg_len before network exposure (unbounded alloc = DoS)
    let mut payload = vec![0u8; msg_len];
    r.read_exact(&mut payload).await?;
    Ok(Some(Message::decode(&payload)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    #[tokio::test]
    async fn write_then_read_over_stream() {
        let (mut a, mut b) = duplex(1024);

        let msg = Message::Hello { version: 7 };
        write_msg(&mut a, &msg).await.unwrap();
        let got = read_msg(&mut b).await.unwrap().unwrap();

        assert_eq!(got, msg);
    }

    #[tokio::test]
    async fn round_trips_over_tcp() {
        use tokio::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            read_msg(&mut sock).await.unwrap()
        });

        let msg = Message::Hello { version: 42 };
        let mut client = TcpStream::connect(addr).await.unwrap();
        write_msg(&mut client, &msg).await.unwrap();

        let got = server.await.unwrap().unwrap();
        assert_eq!(got, msg);
    }

    #[tokio::test]
    async fn read_returns_none_on_clean_eof() {
        let (a, mut b) = duplex(1024);
        drop(a);
        let got = read_msg(&mut b).await.unwrap();
        assert!(got.is_none());
    }
}
