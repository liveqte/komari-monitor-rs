// use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
// use rustls::{
//     ClientConfig, DigitallySignedStruct, DistinguishedName, Error,
//     SignatureScheme,
// };
// use rustls_pki_types::{CertificateDer, ServerName, UnixTime};
// use std::fmt::{Debug, Formatter};
// use std::sync::Arc;
//
// struct NoVerification;
//
// impl Debug for NoVerification {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         f.write_str("NoVerification")
//     }
// }
//
// impl ServerCertVerifier for NoVerification {
//     fn verify_server_cert(
//         &self,
//         _end_entity: &CertificateDer<'_>,
//         _intermediates: &[CertificateDer<'_>],
//         _server_name: &ServerName<'_>,
//         _ocsp_response: &[u8],
//         _now: UnixTime,
//     ) -> Result<ServerCertVerified, Error> {
//         Ok(ServerCertVerified::assertion())
//     }
//
//     fn verify_tls12_signature(
//         &self,
//         _message: &[u8],
//         _cert: &CertificateDer<'_>,
//         _dss: &DigitallySignedStruct,
//     ) -> Result<HandshakeSignatureValid, Error> {
//         Ok(HandshakeSignatureValid::assertion())
//     }
//
//     fn verify_tls13_signature(
//         &self,
//         _message: &[u8],
//         _cert: &CertificateDer<'_>,
//         _dss: &DigitallySignedStruct,
//     ) -> Result<HandshakeSignatureValid, Error> {
//         Ok(HandshakeSignatureValid::assertion())
//     }
//
//     fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
//         vec![
//             SignatureScheme::RSA_PKCS1_SHA1,
//             SignatureScheme::ECDSA_SHA1_Legacy,
//             SignatureScheme::RSA_PKCS1_SHA256,
//             SignatureScheme::ECDSA_NISTP256_SHA256,
//             SignatureScheme::RSA_PKCS1_SHA384,
//             SignatureScheme::ECDSA_NISTP384_SHA384,
//             SignatureScheme::RSA_PKCS1_SHA512,
//             SignatureScheme::ECDSA_NISTP521_SHA512,
//             SignatureScheme::RSA_PSS_SHA256,
//             SignatureScheme::RSA_PSS_SHA384,
//             SignatureScheme::RSA_PSS_SHA512,
//             SignatureScheme::ED25519,
//             SignatureScheme::ED448,
//         ]
//     }
//
//     fn requires_raw_public_keys(&self) -> bool {
//         false
//     }
//
//     fn root_hint_subjects(&self) -> Option<&[DistinguishedName]> {
//         None
//     }
// }
//
// pub fn create_dangerous_config() -> ClientConfig {
//     let verifier = Arc::new(NoVerification {});
//
//     let config = ClientConfig::builder()
//         .dangerous()
//         .with_custom_certificate_verifier(verifier)
//         .with_no_client_auth();
//
//     config
// }
