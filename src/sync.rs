use std::{collections::HashMap, fs, io, path::Path};

use tokio::io::{AsyncRead, AsyncWriteExt};

use crate::{
    manifest::{self},
    protocol::Message,
    store::Store,
    transport,
};

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

pub async fn sync<S: AsyncRead + AsyncWriteExt + Unpin>(
    stream: &mut S,
    root: &Path,
    store: &Store,
) -> io::Result<()> {
    let mine = manifest::build(root, store)?;
    let mut request: HashMap<String, String> = HashMap::new();
    let mut received = 0usize;

    transport::write_msg(stream, &Message::Hello { version: 1 }).await?;
    transport::write_msg(stream, &Message::Manifest(mine.clone())).await?;

    loop {
        match transport::read_msg(stream).await? {
            Some(Message::Manifest(theirs)) => {
                let d = manifest::diff(&mine, &theirs);
                request = d.request;
                let want: Vec<String> = request.values().cloned().collect();
                transport::write_msg(stream, &Message::WantBlobs(want)).await?;
            }
            Some(Message::WantBlobs(hashes)) => {
                for h in hashes {
                    let bytes = store.read(&h)?;
                    transport::write_msg(stream, &Message::Blob { hash: h, bytes }).await?;
                }
                // serving is our final send (our WantBlobs already went out on their Manifest)
                stream.shutdown().await?;
            }
            Some(Message::Blob { bytes, .. }) => {
                store.write(&bytes)?;
                received += 1;
            }
            Some(_) => {}  // Hello
            None => break, // peer closed
        }
    }

    if received != request.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "truncated sync",
        ));
    }

    let mut merged = mine;
    merged.extend(request);
    apply(&merged, store, root)
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
    async fn sync_converges_two_peers() {
        let a_dir = tempfile::tempdir().unwrap();
        let a_store = Store::new(a_dir.path().to_path_buf()).unwrap();
        fs::write(a_dir.path().join("a.txt"), b"hello").unwrap();

        let b_dir = tempfile::tempdir().unwrap();
        let b_store = Store::new(b_dir.path().to_path_buf()).unwrap();
        fs::write(b_dir.path().join("b.txt"), b"world").unwrap();

        let (mut a, mut b) = tokio::io::duplex(4096);

        let (rs, rr) = tokio::join!(
            sync(&mut a, a_dir.path(), &a_store),
            sync(&mut b, b_dir.path(), &b_store),
        );
        rs.unwrap();
        rr.unwrap();

        assert_eq!(fs::read(b_dir.path().join("a.txt")).unwrap(), b"hello");
        assert_eq!(fs::read(a_dir.path().join("b.txt")).unwrap(), b"world");
    }
}
