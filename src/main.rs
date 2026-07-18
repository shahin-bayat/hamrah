use std::{collections::HashSet, env, io, path::PathBuf, sync::Arc, time::Duration};

use hamrah::{identity::Identity, pinning::PinnedPeers, store::Store, sync};
use notify::{RecursiveMode, Watcher, recommended_watcher};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{
    TlsAcceptor, TlsConnector,
    rustls::{
        ClientConfig, ServerConfig,
        crypto::aws_lc_rs,
        pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName},
    },
};
const WINDOW: Duration = Duration::from_millis(300);

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(String::as_str);
    let addr = args
        .get(2)
        .expect("usage: hamrah <send|receive> <addr> <dir>");
    let dir = PathBuf::from(args.get(3).expect("need <dir>"));
    let peer_id = args
        .get(4)
        .expect("usage: hamrah <send|receive> <addr> <dir> <peer-device-id>")
        .clone();

    let identity = Identity::load_or_create(None)?;
    println!("this device: {}", identity.device_id);
    let store = Store::new(dir.join(".hamrah"))?;

    match mode {
        Some("send") => {
            let provider = Arc::new(aws_lc_rs::default_provider());
            let verifier = Arc::new(PinnedPeers::new(HashSet::from([peer_id])));
            let config = ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .map_err(io::Error::other)?
                .dangerous()
                .with_custom_certificate_verifier(verifier)
                .with_client_auth_cert(
                    vec![CertificateDer::from(identity.cert_der.clone())],
                    PrivatePkcs8KeyDer::from(identity.key_der.clone()).into(),
                )
                .map_err(io::Error::other)?;
            let connector = TlsConnector::from(Arc::new(config));
            let server_name = ServerName::try_from("hamrah").map_err(io::Error::other)?;
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
                let tcp = TcpStream::connect(addr).await?;
                let stream = connector.connect(server_name.clone(), tcp).await?;
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
            let provider = Arc::new(aws_lc_rs::default_provider());
            let verifier = Arc::new(PinnedPeers::new(HashSet::from([peer_id])));
            let config = ServerConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .map_err(io::Error::other)?
                .with_client_cert_verifier(verifier)
                .with_single_cert(
                    vec![CertificateDer::from(identity.cert_der.clone())],
                    PrivatePkcs8KeyDer::from(identity.key_der.clone()).into(),
                )
                .map_err(io::Error::other)?;
            let acceptor = TlsAcceptor::from(Arc::new(config));
            let listener = TcpListener::bind(addr).await?;
            loop {
                let (tcp, _) = listener.accept().await?;
                let stream = acceptor.accept(tcp).await?;
                sync::sync(stream, &dir, &store).await?;
                println!("received!");
            }
        }
        _ => eprintln!("usage: hamrah <send|receive> <addr> <dir>"),
    }
    Ok(())
}
