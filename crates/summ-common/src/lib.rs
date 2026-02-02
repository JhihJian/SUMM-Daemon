// Error types
pub mod error;
pub use error::{ErrorCode, DaemonError};

// Core data types
pub mod types;
pub use types::{
    SessionStatus, Session, CliState, CliStatus, DaemonConfig, SessionInfo,
};

// IPC protocol
pub mod protocol;
pub use protocol::{Request, Response, DaemonStatusResponse};
