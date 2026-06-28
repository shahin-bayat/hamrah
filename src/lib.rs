//! devsync content store — hash → blob storage.

use sha2::{Digest, Sha256};
use std::{fmt::Write, fs::create_dir_all, io, path::PathBuf};

pub struct Store {
    objects_dir: PathBuf,
}

impl Store {
    pub fn new(root: PathBuf) -> io::Result<Store> {
        let objects_dir = root.join("objects");
        create_dir_all(&objects_dir)?;
        Ok(Store { objects_dir })
    }

    pub fn write(&self, bytes: &[u8]) -> io::Result<String> {
        let h = hash(bytes);
        let path = self.objects_dir.join(&h);
        if path.exists() {
            return Ok(h);
        }
        std::fs::write(&path, bytes)?;
        Ok(h)
    }

    pub fn read(&self, hash: &str) -> io::Result<Vec<u8>> {
        let path = self.objects_dir.join(hash);
        std::fs::read(self.objects_dir.join(&path))
    }
}

pub fn hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();

    let mut s = String::new();
    for byte in digest {
        write!(s, "{:02x}", byte).unwrap(); // in-memory String write can't fail
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_known_vectors() {
        assert_eq!(
            hash(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hash(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn write_then_read_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().to_path_buf()).unwrap();

        let bytes: Vec<u8> = vec![12, 2, 43, 6, 3];
        let h = store.write(&bytes).unwrap();

        let result = store.read(&h).unwrap();

        assert_eq!(bytes, result)
    }
}
