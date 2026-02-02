use crate::error::DaemonError;
use crate::types::SessionStatus;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// IPC request types sent from CLI to daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    /// Start a new session
    Start {
        /// CLI command to execute
        cli: String,
        /// Initialization source path (directory, .zip, or .tar.gz)
        init: PathBuf,
        /// Optional custom name for the session
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// Stop a running session
    Stop {
        /// Session ID to stop
        session_id: String,
    },
    /// List all sessions, optionally filtered by status
    List {
        /// Optional status filter (running/idle/stopped)
        #[serde(skip_serializing_if = "Option::is_none")]
        status_filter: Option<SessionStatus>,
    },
    /// Query detailed session status
    Status {
        /// Session ID to query
        session_id: String,
    },
    /// Inject a message into a running session
    Inject {
        /// Target session ID
        session_id: String,
        /// Message to inject
        message: String,
    },
    /// Query daemon status
    DaemonStatus,
}

/// IPC response types sent from daemon to CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    /// Successful response with data payload
    Success {
        /// Response data (JSON value)
        data: serde_json::Value,
    },
    /// Error response
    Error {
        /// Error code (e.g., "E001", "E002")
        code: String,
        /// Error message
        message: String,
    },
}

impl Response {
    /// Create a success response with data
    pub fn success(data: serde_json::Value) -> Self {
        Self::Success { data }
    }

    /// Create an error response from DaemonError
    pub fn error(err: &DaemonError) -> Self {
        Self::Error {
            code: err.code.code().to_string(),
            message: err.message.clone(),
        }
    }
}

/// Daemon status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatusResponse {
    /// Daemon running state
    pub running: bool,
    /// Number of active sessions
    pub session_count: usize,
    /// Daemon version
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SessionStatus;

    #[test]
    fn test_request_serialization() {
        let req = Request::Start {
            cli: "claude".to_string(),
            init: PathBuf::from("/path/to/init"),
            name: Some("test-session".to_string()),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""type":"Start""#));
        assert!(json.contains(r#""cli":"claude""#));
        assert!(json.contains(r#""name":"test-session""#));
    }

    #[test]
    fn test_request_with_none_filter() {
        let req = Request::List {
            status_filter: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""type":"List""#));
        assert!(!json.contains(r#""status_filter""#));
    }

    #[test]
    fn test_response_success_creation() {
        let data = serde_json::json!(["item1", "item2"]);
        let resp = Response::success(data.clone());

        match resp {
            Response::Success { data: d } => {
                assert!(d.is_array());
                assert_eq!(d.as_array().unwrap().len(), 2);
            }
            _ => panic!("Expected Success response"),
        }
    }

    #[test]
    fn test_response_error_creation() {
        use crate::error::ErrorCode;

        let err = DaemonError::new(ErrorCode::E002, "session not found");
        let resp = Response::error(&err);

        match resp {
            Response::Error { code, message } => {
                assert_eq!(code, "E002");
                assert_eq!(message, "session not found");
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[test]
    fn test_request_list_with_status_filter() {
        let req = Request::List {
            status_filter: Some(SessionStatus::Running),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""type":"List""#));
        assert!(json.contains(r#""status_filter":"running""#));
    }

    #[test]
    fn test_request_inject() {
        let req = Request::Inject {
            session_id: "session-123".to_string(),
            message: "test message".to_string(),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""type":"Inject""#));
        assert!(json.contains(r#""session_id":"session-123""#));
        assert!(json.contains(r#""message":"test message""#));
    }

    #[test]
    fn test_daemon_status_response_serialization() {
        let status = DaemonStatusResponse {
            running: true,
            session_count: 4,
            version: "0.1.0".to_string(),
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains(r#""running":true"#));
        assert!(json.contains(r#""session_count":4"#));
        assert!(json.contains(r#""version":"0.1.0""#));
    }

    #[test]
    fn test_request_deserialization() {
        let json = r#"{"type":"Status","session_id":"test-session"}"#;
        let req: Request = serde_json::from_str(json).unwrap();

        match req {
            Request::Status { session_id } => {
                assert_eq!(session_id, "test-session");
            }
            _ => panic!("Expected Status request"),
        }
    }

    #[test]
    fn test_response_deserialization() {
        let json = r#"{"type":"Success","data":{"result":"ok"}}"#;
        let resp: Response = serde_json::from_str(json).unwrap();

        match resp {
            Response::Success { data } => {
                assert_eq!(data["result"], "ok");
            }
            _ => panic!("Expected Success response"),
        }
    }

    #[test]
    fn test_response_error_deserialization() {
        let json = r#"{"type":"Error","code":"E002","message":"not found"}"#;
        let resp: Response = serde_json::from_str(json).unwrap();

        match resp {
            Response::Error { code, message } => {
                assert_eq!(code, "E002");
                assert_eq!(message, "not found");
            }
            _ => panic!("Expected Error response"),
        }
    }
}
