pub mod backend_trait;
pub mod docker_backend;
pub mod local_backend;
pub mod ssh_backend;

pub use backend_trait::{BackendType, SpawnConfig, TerminalBackend, TerminalExit, TerminalOutput};
pub use docker_backend::DockerBackend;
pub use local_backend::LocalBackend;
pub use ssh_backend::SshBackend;
