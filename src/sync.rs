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

enum WriterCmd {
    Send(Message),
    Close,
}

pub async fn sync<S: AsyncRead + AsyncWriteExt + Unpin>(
    stream: S,
    root: &Path,
    store: &Store,
) -> io::Result<()> {
    let (mut rd, mut wr) = tokio::io::split(stream);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<WriterCmd>();

    let mine = manifest::build(root, store)?;

    queue_cmd(&tx, WriterCmd::Send(Message::Hello { version: 1 }))?;
    queue_cmd(&tx, WriterCmd::Send(Message::Manifest(mine.clone())))?;

    let writer = async move {
        while let Some(out) = rx.recv().await {
            match out {
                WriterCmd::Send(msg) => transport::write_msg(&mut wr, &msg).await?,
                WriterCmd::Close => break,
            }
        }
        wr.shutdown().await
    };

    let reader = async move {
        let mut request: HashMap<String, String> = HashMap::new();
        let mut received = 0usize;

        loop {
            match transport::read_msg(&mut rd).await? {
                Some(Message::Manifest(theirs)) => {
                    let d = manifest::diff(&mine, &theirs);
                    request = d.request;
                    let want: Vec<String> = request.values().cloned().collect();

                    queue_cmd(&tx, WriterCmd::Send(Message::WantBlobs(want)))?;
                }
                Some(Message::WantBlobs(hashes)) => {
                    for h in hashes {
                        let bytes = store.read(&h)?;
                        queue_cmd(&tx, WriterCmd::Send(Message::Blob { hash: h, bytes }))?;
                    }
                    queue_cmd(&tx, WriterCmd::Close)?;
                }
                Some(Message::Blob { bytes, .. }) => {
                    store.write(&bytes)?;
                    received += 1;
                }
                Some(_) => {}
                None => break,
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
        Ok(merged)
    };

    let (merged, written) = tokio::join!(reader, writer);
    let merged = merged?;
    written?;
    apply(&merged, store, root)
}

fn queue_cmd(tx: &tokio::sync::mpsc::UnboundedSender<WriterCmd>, out: WriterCmd) -> io::Result<()> {
    tx.send(out)
        .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "writer closed"))
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

        let (a, b) = tokio::io::duplex(4096);

        let (rs, rr) = tokio::join!(
            sync(a, a_dir.path(), &a_store),
            sync(b, b_dir.path(), &b_store),
        );
        rs.unwrap();
        rr.unwrap();

        assert_eq!(fs::read(b_dir.path().join("a.txt")).unwrap(), b"hello");
        assert_eq!(fs::read(a_dir.path().join("b.txt")).unwrap(), b"world");
    }
}
