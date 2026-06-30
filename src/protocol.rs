use std::collections::HashMap;

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

            _ => todo!(),
        };
        let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
        frame.extend_from_slice(&payload);
        frame
    }
}

fn put_str(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(&(s.len() as u32).to_be_bytes());
    buf.extend_from_slice(s.as_bytes());
}
