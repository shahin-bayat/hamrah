use std::{collections::HashMap, fs, io, path::Path};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::{manifest, protocol::Message, store::Store, transport};

fn apply(manifest: &HashMap<String, String>, store: &Store, dest: &Path) -> io::Result<()> {
    for (rel_path, hash) in manifest {
        let path = dest.join(rel_path);
        let bytes = store.read(hash)?;

        // nested files:
        // dest: code , path: code/rust/hamrah/src/main.rs
        // parent = code/rust/hamrah/src
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)?;
    }

    Ok(())
}

pub async fn send<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    root: &Path,
    store: &Store,
) -> io::Result<()> {
    transport::write_msg(stream, &Message::Hello { version: 1 }).await?;

    let m = manifest::build(root, store)?;
    transport::write_msg(stream, &Message::Manifest(m)).await?;

    let wanted = match transport::read_msg(stream).await? {
        Message::WantBlobs(hashes) => hashes,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "expected WantBlobs",
            ));
        }
    };

    for h in wanted {
        let bytes = store.read(&h)?;
        transport::write_msg(stream, &Message::Blob { hash: h, bytes }).await?;
    }
    Ok(())
}

pub async fn receive<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    root: &Path,
    store: &Store,
) -> io::Result<()> {
    match transport::read_msg(stream).await? {
        Message::Hello { version: _ } => {}
        _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "expected Hello")),
    };

    let manifest = match transport::read_msg(stream).await? {
        Message::Manifest(m) => m,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "expected Manifest",
            ));
        }
    };

    let missing: Vec<String> = manifest
        .values()
        .filter(|h| !store.has(h))
        .cloned()
        .collect();

    transport::write_msg(stream, &Message::WantBlobs(missing.clone())).await?;

    for _ in 0..missing.len() {
        match transport::read_msg(stream).await? {
            Message::Blob { hash: _, bytes } => {
                store.write(&bytes)?;
            }
            _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "expected Blob")),
        }
    }

    apply(&manifest, store, root)
}

#[cfg(test)]
mod tests {

    use crate::manifest;

    use super::*;

    #[test]
    fn apply_materializes_tree() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("a.txt"), b"Hello from a!").unwrap();
        fs::create_dir_all(src.path().join("sub")).unwrap();
        fs::write(src.path().join("sub/b.txt"), b"Hello from b!").unwrap();

        let store_dir = tempfile::tempdir().unwrap();
        let store = Store::new(store_dir.path().to_path_buf()).unwrap();
        let m = manifest::build(src.path(), &store).unwrap();

        let dest = tempfile::tempdir().unwrap();
        apply(&m, &store, dest.path()).unwrap();

        assert_eq!(
            fs::read(dest.path().join("a.txt")).unwrap(),
            b"Hello from a!"
        );
        assert_eq!(
            fs::read(dest.path().join("sub/b.txt")).unwrap(),
            b"Hello from b!"
        );
    }

    #[tokio::test]
    async fn send_receive_syncs_tree() {
        let s_dir = tempfile::tempdir().unwrap();
        let s_store = Store::new(s_dir.path().to_path_buf()).unwrap();
        fs::write(s_dir.path().join("a.txt"), b"hello").unwrap();
        fs::create_dir_all(s_dir.path().join("sub")).unwrap();
        fs::write(s_dir.path().join("sub/b.txt"), b"world").unwrap();

        let r_dir = tempfile::tempdir().unwrap();
        let r_store = Store::new(r_dir.path().to_path_buf()).unwrap();

        let (mut a, mut b) = tokio::io::duplex(4096);

        let (rs, rr) = tokio::join!(
            send(&mut a, s_dir.path(), &s_store),
            receive(&mut b, r_dir.path(), &r_store),
        );
        rs.unwrap();
        rr.unwrap();

        assert_eq!(fs::read(r_dir.path().join("a.txt")).unwrap(), b"hello");
        assert_eq!(fs::read(r_dir.path().join("sub/b.txt")).unwrap(), b"world");
    }
}
