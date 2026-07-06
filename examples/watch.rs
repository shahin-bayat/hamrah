use notify::{RecursiveMode, Watcher, recommended_watcher};
use std::{env, path::Path, sync::mpsc, time::Duration};

fn main() -> notify::Result<()> {
    let dir = env::args().nth(1).expect("usage: watch <dir>");
    let (tx, rx) = mpsc::channel();
    let mut watcher = recommended_watcher(tx)?;
    watcher.watch(Path::new(&dir), RecursiveMode::Recursive)?;

    loop {
        let _ = rx.recv()?;
        loop {
            match rx.recv_timeout(Duration::from_millis(1500)) {
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        println!("settled!, sync now.");
    }
}
