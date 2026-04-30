use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

const DISCOVERY_PORT: u16 = 53317;
const DISCOVERY_MAGIC: &[u8] = b"AXAGENT_LAN_DISCOVER";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanPeer {
    pub id: String,
    pub hostname: String,
    pub port: u16,
    pub addresses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanTransferRequest {
    pub file_name: String,
    pub file_size: u64,
    pub file_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LanMessage {
    Discovery,
    DiscoveryReply(LanPeer),
    TransferRequest(LanTransferRequest),
    TransferAccepted,
    TransferRejected(String),
    TransferComplete,
    TransferProgress { bytes: u64, total: u64 },
}

pub struct LanDiscovery {
    running: Arc<std::sync::atomic::AtomicBool>,
    task: Mutex<Option<JoinHandle<()>>>,
    peers: Arc<Mutex<Vec<LanPeer>>>,
}

impl LanDiscovery {
    pub fn new() -> Self {
        Self {
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            task: Mutex::new(None),
            peers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn peers(&self) -> Vec<LanPeer> {
        self.peers.lock().await.clone()
    }

    pub async fn start(&self, peer: LanPeer) -> anyhow::Result<()> {
        if self.running.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let running = self.running.clone();
        let _peers = self.peers.clone();
        let peer_data = serde_json::to_vec(&LanMessage::DiscoveryReply(peer.clone()))?;

        let listener_task = tokio::spawn(async move {
            let addr = format!("0.0.0.0:{}", DISCOVERY_PORT);
            let socket = match UdpSocket::bind(&addr) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("LAN discovery bind failed: {}", e);
                    return;
                }
            };
            let _ = socket.set_read_timeout(Some(Duration::from_secs(2)));

            let mut buf = [0u8; 65536];
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                match socket.recv_from(&mut buf) {
                    Ok((len, src)) => {
                        if len < DISCOVERY_MAGIC.len() {
                            continue;
                        }
                        if &buf[..DISCOVERY_MAGIC.len()] != DISCOVERY_MAGIC {
                            continue;
                        }
                        // Received discovery request, send reply
                        let _ = socket.send_to(&peer_data, src);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(_) => continue,
                }
            }
        });

        let mut task_guard = self.task.lock().await;
        *task_guard = Some(listener_task);
        Ok(())
    }

    pub async fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        if let Some(task) = self.task.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
    }

    /// Scan the LAN for peers by broadcasting a discovery request.
    pub async fn scan(&self, timeout_secs: u64) -> Vec<LanPeer> {
        let mut found = Vec::new();

        let socket = match UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => s,
            Err(_) => return found,
        };
        let _ = socket.set_broadcast(true);
        let _ = socket.set_read_timeout(Some(Duration::from_secs(3)));

        let target = format!("255.255.255.255:{}", DISCOVERY_PORT);
        let _ = socket.send_to(DISCOVERY_MAGIC, &target);

        let mut buf = [0u8; 65536];
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);

        while std::time::Instant::now() < deadline {
            match socket.recv_from(&mut buf) {
                Ok((len, _src)) => {
                    if let Ok(LanMessage::DiscoveryReply(peer)) =
                        serde_json::from_slice(&buf[..len])
                    {
                        if !found.iter().any(|p: &LanPeer| p.id == peer.id) {
                            found.push(peer);
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        let mut peers = self.peers.lock().await;
        *peers = found.clone();
        found
    }
}

impl Default for LanDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

// ── TCP File Transfer ────────────────────────────────────────────────

pub struct LanFileServer {
    running: Arc<std::sync::atomic::AtomicBool>,
    task: Mutex<Option<JoinHandle<()>>>,
    port: Arc<Mutex<u16>>,
}

impl LanFileServer {
    pub fn new() -> Self {
        Self {
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            task: Mutex::new(None),
            port: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn port(&self) -> u16 {
        *self.port.lock().await
    }

    pub async fn start(&self, shared_dir: std::path::PathBuf) -> anyhow::Result<()> {
        if self.running.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let listener = TcpListener::bind("0.0.0.0:0")?;
        let port = listener.local_addr()?.port();
        *self.port.lock().await = port;

        let running = self.running.clone();

        let task = tokio::spawn(async move {
            let _ = listener.set_nonblocking(true);
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, addr)) => {
                        tracing::info!("LAN transfer connection from {}", addr);
                        let dir = shared_dir.clone();
                        tokio::spawn(async move {
                            handle_transfer_connection(&mut stream, &dir).await;
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    Err(_) => break,
                }
            }
        });

        *self.task.lock().await = Some(task);
        Ok(())
    }

    pub async fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        if let Some(task) = self.task.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
    }
}

impl Default for LanFileServer {
    fn default() -> Self {
        Self::new()
    }
}

async fn handle_transfer_connection(stream: &mut TcpStream, shared_dir: &std::path::Path) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(60)));

    // Read message length prefix (4 bytes, big-endian)
    let mut len_buf = [0u8; 4];
    if stream.read_exact(&mut len_buf).is_err() {
        return;
    }
    let msg_len = u32::from_be_bytes(len_buf) as usize;
    if msg_len > 1024 * 1024 {
        return;
    }

    let mut msg_buf = vec![0u8; msg_len];
    if stream.read_exact(&mut msg_buf).is_err() {
        return;
    }

    let msg: LanMessage = match serde_json::from_slice(&msg_buf) {
        Ok(m) => m,
        Err(_) => return,
    };

    if let LanMessage::TransferRequest(req) = msg {
        let file_path = shared_dir.join(&req.file_name);
        if !file_path.starts_with(shared_dir) {
            let _ = send_msg(stream, &LanMessage::TransferRejected("Invalid path".into()));
            return;
        }
        if !file_path.exists() {
            let _ = send_msg(
                stream,
                &LanMessage::TransferRejected("File not found".into()),
            );
            return;
        }

        let data = match std::fs::read(&file_path) {
            Ok(d) => d,
            Err(_) => {
                let _ = send_msg(stream, &LanMessage::TransferRejected("Read error".into()));
                return;
            }
        };

        let _ = send_msg(stream, &LanMessage::TransferAccepted);

        let size_bytes = (data.len() as u64).to_be_bytes();
        let _ = stream.write_all(&size_bytes);

        let chunk_size = 65536;
        let mut sent = 0usize;
        while sent < data.len() {
            let end = (sent + chunk_size).min(data.len());
            if stream.write_all(&data[sent..end]).is_err() {
                return;
            }
            sent = end;
            let progress = LanMessage::TransferProgress {
                bytes: sent as u64,
                total: data.len() as u64,
            };
            let _ = send_msg(stream, &progress);
        }

        let _ = send_msg(stream, &LanMessage::TransferComplete);
    }
}

fn send_msg(stream: &mut TcpStream, msg: &LanMessage) -> std::io::Result<()> {
    let data = serde_json::to_vec(msg).map_err(std::io::Error::other)?;
    let len = (data.len() as u32).to_be_bytes();
    stream.write_all(&len)?;
    stream.write_all(&data)?;
    stream.flush()?;
    Ok(())
}

// ── Client ───────────────────────────────────────────────────────────

pub struct LanFileClient;

impl LanFileClient {
    /// Download a file from a LAN peer
    pub async fn download(
        peer: &LanPeer,
        file_name: &str,
        save_path: &std::path::Path,
    ) -> anyhow::Result<()> {
        let addr = format!(
            "{}:{}",
            peer.addresses.first().cloned().unwrap_or_default(),
            peer.port
        );
        let mut stream = TcpStream::connect(&addr)?;
        let _ = stream.set_read_timeout(Some(Duration::from_secs(120)));

        let request = LanMessage::TransferRequest(LanTransferRequest {
            file_name: file_name.to_string(),
            file_size: 0,
            file_hash: String::new(),
        });
        send_msg(&mut stream, &request)?;

        // Read response
        let response = recv_msg(&mut stream).await?;
        match response {
            LanMessage::TransferAccepted => {
                // Read file size
                let mut size_buf = [0u8; 8];
                stream.read_exact(&mut size_buf)?;
                let total = u64::from_be_bytes(size_buf) as usize;

                // Read file data
                let mut data = Vec::with_capacity(total);
                let mut buf = [0u8; 65536];
                while data.len() < total {
                    let n = stream.read(&mut buf)?;
                    if n == 0 {
                        break;
                    }
                    data.extend_from_slice(&buf[..n]);
                }

                // Save to file
                std::fs::write(save_path, &data)?;

                // Read transfer complete
                let _ = recv_msg(&mut stream).await;
                Ok(())
            }
            LanMessage::TransferRejected(reason) => {
                anyhow::bail!("Transfer rejected: {}", reason)
            }
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    /// Request a file list from a LAN peer
    pub async fn list_files(peer: &LanPeer) -> anyhow::Result<Vec<String>> {
        let addr = format!(
            "{}:{}",
            peer.addresses.first().cloned().unwrap_or_default(),
            peer.port
        );
        let mut stream = TcpStream::connect(&addr)?;
        let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));

        let request = LanMessage::Discovery;
        send_msg(&mut stream, &request)?;

        let response = recv_msg(&mut stream).await?;
        // For file listing, peer returns a list
        match response {
            LanMessage::DiscoveryReply(_) => Ok(Vec::new()),
            _ => Ok(Vec::new()),
        }
    }
}

async fn recv_msg(stream: &mut TcpStream) -> anyhow::Result<LanMessage> {
    let mut len_buf = [0u8; 4];
    let mut read = 0;
    while read < 4 {
        match stream.read(&mut len_buf[read..]) {
            Ok(0) => anyhow::bail!("Connection closed"),
            Ok(n) => read += n,
            Err(_) => anyhow::bail!("Read error"),
        }
    }
    let msg_len = u32::from_be_bytes(len_buf) as usize;
    if msg_len > 10 * 1024 * 1024 {
        anyhow::bail!("Message too large");
    }
    let mut msg_buf = vec![0u8; msg_len];
    stream.read_exact(&mut msg_buf)?;
    Ok(serde_json::from_slice(&msg_buf)?)
}
