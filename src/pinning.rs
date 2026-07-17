use std::collections::HashSet;

use tokio_rustls::rustls::{
    CertificateError, DigitallySignedStruct, DistinguishedName, Error, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{
        WebPkiSupportedAlgorithms, aws_lc_rs, verify_tls12_signature, verify_tls13_signature,
    },
    pki_types::{CertificateDer, ServerName, UnixTime},
    server::danger::{ClientCertVerified, ClientCertVerifier},
};

use crate::store;

#[derive(Debug)]
pub struct PinnedPeers {
    pinned: HashSet<String>,
    signature_verification_algorithms: WebPkiSupportedAlgorithms,
}

impl PinnedPeers {
    pub fn new(pinned: HashSet<String>) -> Self {
        Self {
            pinned,
            signature_verification_algorithms: aws_lc_rs::default_provider()
                .signature_verification_algorithms,
        }
    }

    fn is_pinned(&self, cert_der: &[u8]) -> bool {
        self.pinned.contains(&store::hash(cert_der))
    }
}

impl ServerCertVerifier for PinnedPeers {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        if self.is_pinned(end_entity.as_ref()) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(Error::InvalidCertificate(
                CertificateError::ApplicationVerificationFailure,
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(message, cert, dss, &self.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(message, cert, dss, &self.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.signature_verification_algorithms.supported_schemes()
    }
}

impl ClientCertVerifier for PinnedPeers {
    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, Error> {
        if self.is_pinned(end_entity.as_ref()) {
            Ok(ClientCertVerified::assertion())
        } else {
            Err(Error::InvalidCertificate(
                CertificateError::ApplicationVerificationFailure,
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(message, cert, dss, &self.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(message, cert, dss, &self.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.signature_verification_algorithms.supported_schemes()
    }

    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_cert_matches_others_dont() {
        let trusted = store::hash(b"pretend-cert-bytes");
        let peers = PinnedPeers::new(HashSet::from([trusted]));

        assert!(peers.is_pinned(b"pretend-cert-bytes"));
        assert!(!peers.is_pinned(b"some-other-cert"));
    }
}
