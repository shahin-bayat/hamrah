use std::{env, path::PathBuf};

use devsync::{store::Store, sync};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(String::as_str);
    let addr = args
        .get(2)
        .expect("usage: devsync <send|receive> <addr> <dir>");
    let dir = PathBuf::from(args.get(3).expect("need <dir>"));

    let store = Store::new(dir.join(".devsync"))?;

    match mode {
        Some("send") => {
            let mut stream = TcpStream::connect(addr).await?;
            sync::send(&mut stream, &dir, &store).await?;
        }
        Some("receive") => {
            let listener = TcpListener::bind(addr).await?;
            let (mut stream, _) = listener.accept().await?;
            sync::receive(&mut stream, &dir, &store).await?;
        }
        _ => eprintln!("usage: devsync <send|receive> <addr> <dir>"),
    }
    Ok(())
}
