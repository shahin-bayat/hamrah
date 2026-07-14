use std::{
    fs, io,
    path::{Path, PathBuf},
};

use rcgen::{CertifiedKey, generate_simple_self_signed};

use crate::store;

pub struct Identity {
    pub device_id: String, // hex SHA-256 of cert DER
    pub cert_der: Vec<u8>,
    pub key_der: Vec<u8>, // PKCS#8
}

impl Identity {
    pub fn load_or_create(config_path: Option<PathBuf>) -> io::Result<Identity> {
        let dir = config_path
            .or_else(dirs::config_dir)
            .ok_or_else(|| io::Error::other("could not determine config directory"))?
            .join("hamrah");
        let cert_path = dir.join("device.der");
        let key_path = dir.join("device-key.der");

        if !cert_path.exists() || !key_path.exists() {
            fs::create_dir_all(&dir)?;
            let CertifiedKey { cert, signing_key } =
                generate_simple_self_signed(vec!["hamrah".to_string()])
                    .map_err(io::Error::other)?;
            fs::write(&cert_path, cert.der())?;
            write_key_0600(&key_path, signing_key.serialized_der())?
        }

        let cert_der = fs::read(cert_path)?;
        let key_der = fs::read(key_path)?;
        Ok(Self {
            device_id: store::hash(&cert_der),
            cert_der,
            key_der,
        })
    }
}

#[cfg(unix)]
fn write_key_0600(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?
        .write_all(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn generates_then_reloads() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = Some(tmp.path().to_path_buf());

        let a = Identity::load_or_create(dir.clone()).unwrap(); // 1st call: generates
        let b = Identity::load_or_create(dir).unwrap(); // 2nd call: must reload, not regenerate

        assert_eq!(a.device_id, b.device_id);
        assert_eq!(a.cert_der, b.cert_der);
        assert!(!a.device_id.is_empty());
        assert!(tmp.path().join("hamrah/device.der").exists());
        assert!(tmp.path().join("hamrah/device-key.der").exists());
        assert!(
            fs::metadata(tmp.path().join("hamrah/device-key.der"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777
                == 0o600
        )
    }
}
