use std::{env, io, path::PathBuf, time::Duration};

use hamrah::{store::Store, sync};
use notify::{RecursiveMode, Watcher, recommended_watcher};
use tokio::net::{TcpListener, TcpStream};

const WINDOW: Duration = Duration::from_millis(300);

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(String::as_str);
    let addr = args
        .get(2)
        .expect("usage: hamrah <send|receive> <addr> <dir>");
    let dir = PathBuf::from(args.get(3).expect("need <dir>"));

    let store = Store::new(dir.join(".hamrah"))?;

    match mode {
        Some("send") => {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
            let mut watcher = recommended_watcher(move |_res| {
                // notify is a doorbell: ignore what changed, just ping. sync re-scans the whole dir
                let _ = tx.send(());
            })
            .map_err(io::Error::other)?;

            watcher
                .watch(&dir, RecursiveMode::Recursive)
                .map_err(io::Error::other)?;
            loop {
                let stream = TcpStream::connect(addr).await?;
                sync::sync(stream, &dir, &store).await?;
                println!("synced");

                if rx.recv().await.is_none() {
                    break;
                }

                // only stop when the watcher storm settled
                while let Ok(Some(_)) = tokio::time::timeout(WINDOW, rx.recv()).await {}
            }
        }
        Some("receive") => {
            let listener = TcpListener::bind(addr).await?;
            loop {
                let (stream, _) = listener.accept().await?;
                sync::sync(stream, &dir, &store).await?;
                println!("received!");
            }
        }
        _ => eprintln!("usage: hamrah <send|receive> <addr> <dir>"),
    }
    Ok(())
}
