use std::path::PathBuf;
use std::sync::Arc;

use rustls::server::{AllowAnyAuthenticatedClient, ClientHello, ResolvesServerCert};
use rustls::sign::{CertifiedKey, Signer};
use rustls::{
    Certificate, PrivateKey, RootCertStore, ServerConfig, SupportedCipherSuite,
    ALL_CIPHER_SUITES,
};
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Error)]
pub enum TlsError {
    #[error("Failed to load certificate: {0}")]
    CertificateLoad(String),

    #[error("Failed to load private key: {0}")]
    KeyLoad(String),

    #[error("Invalid certificate format: {0}")]
    InvalidFormat(String),

    #[error("TLS not configured")]
    NotConfigured,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct TlsCertificate {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub ca_cert_path: Option<PathBuf>,
}

impl TlsCertificate {
    pub fn new(cert_path: impl Into<PathBuf>, key_path: impl Into<PathBuf>) -> Self {
        Self {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
            ca_cert_path: None,
        }
    }

    pub fn with_ca_cert(mut self, ca_cert_path: PathBuf) -> Self {
        self.ca_cert_path = Some(ca_cert_path);
        self
    }
}

#[derive(Debug, Clone)]
pub struct TlsProtocol {
    pub name: String,
    pub version: String,
}

impl Default for TlsProtocol {
    fn default() -> Self {
        Self {
            name: "TLS".to_string(),
            version: "1.3".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TlsCipherSuite {
    pub name: String,
    pub id: u16,
}

impl TlsCipherSuite {
    pub fn default_suites() -> Vec<Self> {
        vec![
            TlsCipherSuite {
                name: "TLS_AES_256_GCM_SHA384".to_string(),
                id: 0x1302,
            },
            TlsCipherSuite {
                name: "TLS_AES_128_GCM_SHA256".to_string(),
                id: 0x1301,
            },
            TlsCipherSuite {
                name: "TLS_CHACHA20_POLY1305_SHA256".to_string(),
                id: 0x1303,
            },
        ]
    }
}

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub certificate: Option<TlsCertificate>,
    pub protocols: Vec<TlsProtocol>,
    pub cipher_suites: Vec<TlsCipherSuite>,
    pub prefer_server_cipher_suite: bool,
    pub session_tickets_disabled: bool,
    pub tickets_per_second: u32,
    pub tickets_per_day: u32,
    pub verify_client: bool,
    pub client_auth_type: ClientAuthType,
    pub alpn_protocols: Vec<Vec<u8>>,
    pub handshaker_timeout_ms: u64,
    pub read_timeout_ms: u64,
    pub write_timeout_ms: u64,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            certificate: None,
            protocols: vec![TlsProtocol::default()],
            cipher_suites: TlsCipherSuite::default_suites(),
            prefer_server_cipher_suite: true,
            session_tickets_disabled: false,
            tickets_per_second: 4,
            tickets_per_day: 64 * 1024,
            verify_client: false,
            client_auth_type: ClientAuthType::None,
            alpn_protocols: vec![
                b"h2".to_vec(),
                b"http/1.1".to_vec(),
            ],
            handshaker_timeout_ms: 5000,
            read_timeout_ms: 30000,
            write_timeout_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientAuthType {
    None,
    Optional,
    Require,
}

impl Default for ClientAuthType {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug)]
pub struct TlsConfigBuilder {
    config: TlsConfig,
}

impl TlsConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: TlsConfig::default(),
        }
    }

    pub fn certificate(mut self, cert: TlsCertificate) -> Self {
        self.config.certificate = Some(cert);
        self
    }

    pub fn protocols(mut self, protocols: Vec<TlsProtocol>) -> Self {
        self.config.protocols = protocols;
        self
    }

    pub fn cipher_suites(mut self, suites: Vec<TlsCipherSuite>) -> Self {
        self.config.cipher_suites = suites;
        self
    }

    pub fn verify_client(mut self, verify: bool) -> Self {
        self.config.verify_client = verify;
        self
    }

    pub fn client_auth_type(mut self, auth_type: ClientAuthType) -> Self {
        self.config.client_auth_type = auth_type;
        self
    }

    pub fn alpn_protocols(mut self, protocols: Vec<String>) -> Self {
        self.config.alpn_protocols = protocols.into_iter().map(|p| p.into_bytes()).collect();
        self
    }

    pub fn handshaker_timeout(mut self, ms: u64) -> Self {
        self.config.handshaker_timeout_ms = ms;
        self
    }

    pub fn build(self) -> Result<ServerConfig, TlsError> {
        let cert = self.config.certificate.ok_or(TlsError::NotConfigured)?;

        let cert_data = std::fs::read(&cert.cert_path)
            .map_err(|e| TlsError::CertificateLoad(format!("{}: {}", cert.cert_path.display(), e)))?;

        let key_data = std::fs::read(&cert.key_path)
            .map_err(|e| TlsError::KeyLoad(format!("{}: {}", cert.key_path.display(), e)))?;

        let certs = cert_data
            .split(|b| b == &0x30)
            .filter(|p| !p.is_empty())
            .map(|p| Certificate(p.to_vec()))
            .collect::<Vec<_>>();

        if certs.is_empty() {
            return Err(TlsError::InvalidFormat("No certificates found".to_string()));
        }

        let key = if key_data.starts_with(b"-----BEGIN") {
            let pem_parser = rustls_pemfile::pkcs8_private_keys(&mut key_data.as_slice())
                .map_err(|e| TlsError::KeyLoad(e.to_string()))?;
            if pem_parser.is_empty() {
                return Err(TlsError::InvalidFormat("No private key found".to_string()));
            }
            PrivateKey(pem_parser[0].secret.clone())
        } else {
            PrivateKey(key_data)
        };

        let signer = Signer::new(&rustls::PKCS8_SIGNING_ALCOS, &key)
            .map_err(|e| TlsError::KeyLoad(e.to_string()))?;

        let certified_key = CertifiedKey::new(certs, Arc::new(signer));

        let cipher_suites = self
            .config
            .cipher_suites
            .iter()
            .filter_map(|cs| {
                ALL_CIPHER_SUITES
                    .iter()
                    .find(|s| s.suite() == cs.id)
                    .cloned()
            })
            .collect::<Vec<_>>();

        let client_auth = match self.config.client_auth_type {
            ClientAuthType::None => AllowAnyAuthenticatedClient::new(root_cert_store(&cert)?),
            ClientAuthType::Optional => AllowAnyAuthenticatedClient::new(root_cert_store(&cert)?),
            ClientAuthType::Require => AllowAnyAuthenticatedClient::new(root_cert_store(&cert)?),
        };

        let mut server_config = ServerConfig::builder()
            .cipher_suites(&cipher_suites)
            .authenticated_builder(rustls::server::AllowAnyAuthenticatedClient::new(
                RootCertStore::empty(),
            ))
            .unwrap_or_else(|_| {
                ServerConfig::builder()
                    .cipher_suites(&cipher_suites)
                    .with_cert_repository(Arc::new(rustls::server::allow_any_authenticated_client(
                        RootCertStore::empty(),
                    )))
                    .unwrap()
            })
            .with_certified_key(Arc::new(certified_key))
            .with_client_auth(self.config.verify_client);

        server_config.alpn_protocols = self.config.alpn_protocols;
        server_config.prefer_server_ciphers = self.config.prefer_server_cipher_suite;

        if self.config.session_tickets_disabled {
            server_config.session_storage = Arc::new(rustls::server::session::SessionMemoryStore::new());
            server_config.ticket_suite = None;
        }

        Ok(server_config)
    }

    pub fn build_simple(self) -> Result<ServerConfig, TlsError> {
        let cert = self.config.certificate.ok_or(TlsError::NotConfigured)?;

        let cert_data = std::fs::read(&cert.cert_path)
            .map_err(|e| TlsError::CertificateLoad(format!("{}: {}", cert.cert_path.display(), e)))?;

        let key_data = std::fs::read(&cert.key_path)
            .map_err(|e| TlsError::KeyLoad(format!("{}: {}", cert.key_path.display(), e)))?;

        let certs = vec![Certificate(cert_data)];
        let key = PrivateKey(key_data);

        let mut config = ServerConfig::with_cert(certs, key)
            .map_err(|e| TlsError::CertificateLoad(e.to_string()))?;

        config.alpn_protocols = self.config.alpn_protocols;

        Ok(config)
    }
}

impl Default for TlsConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn root_cert_store(cert: &TlsCertificate) -> Result<RootCertStore, TlsError> {
    let mut store = RootCertStore::empty();

    if let Some(ca_path) = &cert.ca_cert_path {
        let ca_data = std::fs::read(ca_path)
            .map_err(|e| TlsError::CertificateLoad(format!("{}: {}", ca_path.display(), e)))?;
        let ca_certs = ca_data
            .split(|b| b == &0x30)
            .filter(|p| !p.is_empty())
            .map(|p| Certificate(p.to_vec()));
        store.add(ca_certs)
            .map_err(|e| TlsError::CertificateLoad(e.to_string()))?;
    }

    Ok(store)
}

pub struct DynamicTlsConfig {
    config: Arc<RwLock<Option<ServerConfig>>>,
    reload_interval: std::time::Duration,
}

impl DynamicTlsConfig {
    pub fn new(reload_interval: std::time::Duration) -> Self {
        Self {
            config: Arc::new(RwLock::new(None)),
            reload_interval,
        }
    }

    pub async fn update(&self, new_config: ServerConfig) {
        let mut config = self.config.write().await;
        *config = Some(new_config);
    }

    pub async fn get_config(&self) -> Option<ServerConfig> {
        let config = self.config.read().await;
        config.clone()
    }

    pub fn start_reload_task(&self, cert_path: PathBuf, key_path: PathBuf) {
        let config = self.config.clone();
        let interval = self.reload_interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                tracing::debug!("Checking TLS certificate for reload");
            }
        });
    }
}

impl Clone for DynamicTlsConfig {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            reload_interval: self.reload_interval,
        }
    }
}

pub struct TlsCertificateValidator {
    min_key_size: usize,
    allowed_hash_algorithms: Vec<String>,
    check_validity: bool,
}

impl TlsCertificateValidator {
    pub fn new() -> Self {
        Self {
            min_key_size: 2048,
            allowed_hash_algorithms: vec![
                "SHA256".to_string(),
                "SHA384".to_string(),
                "SHA512".to_string(),
            ],
            check_validity: true,
        }
    }

    pub fn min_key_size(mut self, size: usize) -> Self {
        self.min_key_size = size;
        self
    }

    pub fn allowed_hash_algorithms(mut self, algs: Vec<String>) -> Self {
        self.allowed_hash_algorithms = algs;
        self
    }

    pub fn validate(&self, cert: &Certificate) -> Result<(), TlsError> {
        Ok(())
    }
}

impl Default for TlsCertificateValidator {
    fn default() -> Self {
        Self::new()
    }
}