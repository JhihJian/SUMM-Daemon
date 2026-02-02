// Integration tests for SUMM Daemon
// These tests verify protocol serialization, error handling, and session lifecycle

use summ_common::{DaemonConfig, Request, Response, Session, SessionStatus, SessionInfo, DaemonError};
use tempfile::TempDir;

/// Test protocol request serialization
#[test]
fn test_protocol_serialization() {
    let req = Request::Start {
        cli: "claude".to_string(),
        init: std::path::PathBuf::from("/tmp/test-init"),
        name: Some("test-session".to_string()),
    };

    let json = serde_json::to_string(&req).expect("Failed to serialize");
    assert!(json.contains(r#""type":"Start""#));
    assert!(json.contains(r#""cli":"claude""#));
    assert!(json.contains(r#""name":"test-session""#));

    // Verify deserialization works
    let req2: Request = serde_json::from_str(&json).expect("Failed to deserialize");
    match req2 {
        Request::Start { cli, init, name } => {
            assert_eq!(cli, "claude");
            assert_eq!(init, std::path::PathBuf::from("/tmp/test-init"));
            assert_eq!(name, Some("test-session".to_string()));
        }
        _ => panic!("Expected Start request"),
    }
}

/// Test response serialization
#[test]
fn test_response_serialization() {
    let resp = Response::Success {
        data: serde_json::json!({"session_id": "test123"}),
    };

    let json = serde_json::to_string(&resp).expect("Failed to serialize");
    assert!(json.contains(r#""type":"Success""#));
    assert!(json.contains(r#""session_id":"test123""#));

    // Verify deserialization
    let resp2: Response = serde_json::from_str(&json).expect("Failed to deserialize");
    match resp2 {
        Response::Success { data } => {
            assert_eq!(data["session_id"], "test123");
        }
        _ => panic!("Expected Success response"),
    }
}

/// Test error response format matches expected schema
#[test]
fn test_error_response_format() {
    let err = DaemonError::e002("Session not found");
    let resp = Response::error(&err);

    let json = serde_json::to_string(&resp).expect("Failed to serialize");

    // Verify error format matches the expected output
    assert!(json.contains(r#""type":"Error""#));
    assert!(json.contains(r#""code":"E002""#));
    assert!(json.contains(r#""message":"Session not found""#));

    // Verify deserialization
    let resp2: Response = serde_json::from_str(&json).expect("Failed to deserialize");
    match resp2 {
        Response::Error { code, message } => {
            assert_eq!(code, "E002");
            assert_eq!(message, "Session not found");
        }
        _ => panic!("Expected Error response"),
    }
}

/// Test all request types can be serialized
#[test]
fn test_all_request_types_serialization() {
    let init_path = std::path::PathBuf::from("/tmp/test");

    let requests = vec![
        Request::Start {
            cli: "claude".to_string(),
            init: init_path.clone(),
            name: Some("test".to_string()),
        },
        Request::Stop {
            session_id: "sess123".to_string(),
        },
        Request::List {
            status_filter: None,
        },
        Request::List {
            status_filter: Some(SessionStatus::Running),
        },
        Request::Status {
            session_id: "sess456".to_string(),
        },
        Request::Inject {
            session_id: "sess789".to_string(),
            message: "test message".to_string(),
        },
        Request::DaemonStatus,
    ];

    for req in requests {
        let json = serde_json::to_string(&req).expect(&format!("Failed to serialize {:?}", req));
        let req2: Request = serde_json::from_str(&json).expect(&format!("Failed to deserialize {:?}", json));
        // Verify round-trip works
        let json2 = serde_json::to_string(&req2).expect("Failed to serialize round-trip");
        assert_eq!(json, json2, "Round-trip serialization mismatch");
    }
}

/// Test session metadata save and load
#[test]
fn test_session_metadata_persistence() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workdir = temp_dir.path();

    let session = Session {
        session_id: "test001".to_string(),
        tmux_session: "summ-test001".to_string(),
        name: "Test Session".to_string(),
        cli: "claude-code".to_string(),
        workdir: workdir.to_path_buf(),
        init_source: std::path::PathBuf::from("/tmp/init"),
        status: SessionStatus::Running,
        pid: Some(12345),
        created_at: chrono::Utc::now(),
        last_activity: chrono::Utc::now(),
    };

    // Save metadata
    let meta_path = workdir.join("meta.json");
    let json = serde_json::to_string_pretty(&session).expect("Failed to serialize");
    std::fs::write(&meta_path, json).expect("Failed to write metadata");

    // Load metadata
    let content = std::fs::read_to_string(&meta_path).expect("Failed to read metadata");
    let loaded: Session = serde_json::from_str(&content).expect("Failed to deserialize");

    assert_eq!(loaded.session_id, session.session_id);
    assert_eq!(loaded.tmux_session, session.tmux_session);
    assert_eq!(loaded.name, session.name);
    assert_eq!(loaded.cli, session.cli);
    assert_eq!(loaded.status, session.status);
}

/// Test daemon config default values
#[test]
fn test_daemon_config_defaults() {
    let config = DaemonConfig::default();
    assert!(config.sessions_dir.ends_with(".summ-daemon/sessions"));
    assert!(config.logs_dir.ends_with(".summ-daemon/logs"));
    assert!(config.socket_path.ends_with(".summ-daemon/daemon.sock"));
    assert_eq!(config.cleanup_retention_hours, 24);
    assert_eq!(config.tmux_prefix, "summ-");
}

/// Test session status serialization
#[test]
fn test_session_status_serialization() {
    let statuses = vec![
        SessionStatus::Running,
        SessionStatus::Idle,
        SessionStatus::Stopped,
    ];

    for status in statuses {
        let json = serde_json::to_string(&status).expect("Failed to serialize");
        let loaded: SessionStatus = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(status, loaded);
    }
}

/// Test full session creation (simulated, without tmux)
#[test]
fn test_session_creation_simulated() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = DaemonConfig {
        sessions_dir: temp_dir.path().join("sessions"),
        logs_dir: temp_dir.path().join("logs"),
        socket_path: temp_dir.path().join("daemon.sock"),
        cleanup_retention_hours: 24,
        tmux_prefix: "summ-".to_string(),
    };

    // Create session directory structure
    let session_dir = config.sessions_dir.join("test_session");
    std::fs::create_dir_all(&session_dir).expect("Failed to create session dir");
    std::fs::create_dir_all(session_dir.join("workspace")).expect("Failed to create workspace");
    std::fs::create_dir_all(session_dir.join("runtime")).expect("Failed to create runtime");

    // Verify directories exist
    assert!(session_dir.exists());
    assert!(session_dir.join("workspace").exists());
    assert!(session_dir.join("runtime").exists());
}

/// Test SessionInfo conversion from Session
#[test]
fn test_session_info_conversion() {
    let session = Session {
        session_id: "sess123".to_string(),
        tmux_session: "summ-sess123".to_string(),
        name: "Test".to_string(),
        cli: "claude".to_string(),
        workdir: std::path::PathBuf::from("/tmp/test"),
        init_source: std::path::PathBuf::from("/tmp/init"),
        status: SessionStatus::Idle,
        pid: None,
        created_at: chrono::Utc::now(),
        last_activity: chrono::Utc::now(),
    };

    // Clone values before the move
    let session_id = session.session_id.clone();
    let name = session.name.clone();
    let cli = session.cli.clone();
    let status = session.status.clone();

    let info: SessionInfo = session.into();

    assert_eq!(info.session_id, session_id);
    assert_eq!(info.name, name);
    assert_eq!(info.cli, cli);
    assert_eq!(info.status, status);
}
