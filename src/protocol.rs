use std::{
    collections::HashMap,
    io::{self, Cursor, Read},
};

#[derive(Debug, PartialEq)]
pub enum Message {
    Hello { version: u16 },
    Manifest(HashMap<String, String>),
    WantBlobs(Vec<String>),
    Blob { hash: String, bytes: Vec<u8> },
    Deleted(String),
}

impl Message {
    pub fn encode(&self) -> Vec<u8> {
        let payload = match self {
            Self::Hello { version } => {
                let mut p = vec![0u8]; // tag 0
                p.extend_from_slice(&version.to_be_bytes());
                p
            }
            Self::Manifest(map) => {
                let mut p = vec![1u8]; // tag 1
                p.extend_from_slice(&(map.len() as u32).to_be_bytes());
                for (path, hash) in map {
                    put_str(&mut p, path);
                    put_str(&mut p, hash);
                }
                p
            }
            Self::WantBlobs(hashes) => {
                let mut p = vec![2u8]; // tag 2
                p.extend_from_slice(&(hashes.len() as u32).to_be_bytes());
                for hash in hashes {
                    put_str(&mut p, hash);
                }
                p
            }
            Self::Blob { hash, bytes } => {
                let mut p = vec![3u8];
                put_str(&mut p, hash);
                p.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
                p.extend_from_slice(bytes);
                p
            }
            Self::Deleted(path) => {
                let mut p = vec![4u8];
                put_str(&mut p, path);
                p
            }
        };
        let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
        frame.extend_from_slice(&payload);
        frame
    }

    pub fn decode(payload: &[u8]) -> io::Result<Message> {
        let tag = payload[0];
        match tag {
            0 => {
                let mut cur = Cursor::new(&payload[1..]);
                let mut version_buf = [0u8; 2];
                cur.read_exact(&mut version_buf)?;
                let version = u16::from_be_bytes(version_buf);
                Ok(Message::Hello { version })
            }
            1 => {
                let mut cur = Cursor::new(&payload[1..]);
                let mut count_buf = [0u8; 4];
                cur.read_exact(&mut count_buf)?;
                let count = u32::from_be_bytes(count_buf);

                let mut map = HashMap::new();
                for _ in 0..count {
                    let path = get_str(&mut cur)?;
                    let hash = get_str(&mut cur)?;
                    map.insert(path, hash);
                }
                Ok(Message::Manifest(map))
            }
            2 => {
                let mut cur = Cursor::new(&payload[1..]);
                let mut hashes_len_buf = [0u8; 4];
                cur.read_exact(&mut hashes_len_buf)?;
                let hashes_len = u32::from_be_bytes(hashes_len_buf);

                let mut hashes = Vec::new();
                for _ in 0..hashes_len {
                    let hash = get_str(&mut cur)?;
                    hashes.push(hash);
                }
                Ok(Message::WantBlobs(hashes))
            }
            3 => {
                let mut cur = Cursor::new(&payload[1..]);
                let hash = get_str(&mut cur)?;
                let bytes = get_bytes(&mut cur)?;
                Ok(Message::Blob { hash, bytes })
            }
            4 => {
                let mut cur = Cursor::new(&payload[1..]);
                let path = get_str(&mut cur)?;
                Ok(Message::Deleted(path))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "invalid tag")),
        }
    }
}

fn put_str(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(&(s.len() as u32).to_be_bytes());
    buf.extend_from_slice(s.as_bytes());
}

fn get_bytes(r: &mut impl io::Read) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

fn get_str(r: &mut impl io::Read) -> io::Result<String> {
    let buf = get_bytes(r)?;
    String::from_utf8(buf).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad utf8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_round_trips() {
        let msg = Message::Hello { version: 1 };
        let framed = msg.encode();
        let decoded = Message::decode(&framed[4..]).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn manifest_round_trips() {
        let mut map = HashMap::new();
        map.insert("src/main.rs".to_string(), "a3f8".to_string());
        map.insert("README.md".to_string(), "9c12".to_string());
        let msg = Message::Manifest(map);

        let framed = msg.encode();
        let decoded = Message::decode(&framed[4..]).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn want_blobs_round_trips() {
        let msg = Message::WantBlobs(vec!["a3f8".to_string(), "9c12".to_string()]);
        let framed = msg.encode();
        let decoded = Message::decode(&framed[4..]).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn blob_round_trips() {
        let msg = Message::Blob {
            hash: "a3f8".to_string(),
            bytes: vec![0, 255, 10, 200, 0],
        };
        let framed = msg.encode();
        let decoded = Message::decode(&framed[4..]).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn deleted_round_trips() {
        let msg = Message::Deleted("src/old.rs".to_string());
        let framed = msg.encode();
        let decoded = Message::decode(&framed[4..]).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn rejects_unknown_tag() {
        assert!(Message::decode(&[99]).is_err());
    }
}
