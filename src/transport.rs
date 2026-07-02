use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::protocol::Message;

pub async fn write_msg<W: AsyncWriteExt + Unpin>(w: &mut W, msg: &Message) -> io::Result<()> {
    let framed = msg.encode();
    w.write_all(&framed).await
}

pub async fn read_msg<R: AsyncReadExt + Unpin>(r: &mut R) -> io::Result<Message> {
    let mut msg_len_buf = [0u8; 4];
    r.read_exact(&mut msg_len_buf).await?;
    let msg_len = u32::from_be_bytes(msg_len_buf) as usize;

    let mut payload = vec![0u8; msg_len];
    r.read_exact(&mut payload).await?;
    Message::decode(&payload)
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
        let got = read_msg(&mut b).await.unwrap();

        assert_eq!(got, msg);
    }
}
