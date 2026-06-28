//! devsync content store — hash → blob storage.

use sha2::{Digest, Sha256};
use std::fmt::Write;

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
}
