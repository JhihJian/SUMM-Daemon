use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ErrorCode {
    #[error("E001: Init resource not found or inaccessible")]
    E001,
    #[error("E002: Session not found")]
    E002,
    #[error("E003: Session stopped, cannot operate")]
    E003,
    #[error("E004: Archive extraction failed")]
    E004,
    #[error("E005: Process start failed")]
    E005,
    #[error("E006: Message injection failed")]
    E006,
    #[error("E007: Daemon not running")]
    E007,
    #[error("E008: Invalid CLI command")]
    E008,
    #[error("E009: tmux not available")]
    E009,
}

impl ErrorCode {
    pub fn code(&self) -> &'static str {
        match self {
            ErrorCode::E001 => "E001",
            ErrorCode::E002 => "E002",
            ErrorCode::E003 => "E003",
            ErrorCode::E004 => "E004",
            ErrorCode::E005 => "E005",
            ErrorCode::E006 => "E006",
            ErrorCode::E007 => "E007",
            ErrorCode::E008 => "E008",
            ErrorCode::E009 => "E009",
        }
    }
}

#[derive(Debug, Clone, Error)]
pub struct DaemonError {
    pub code: ErrorCode,
    pub message: String,
}

impl DaemonError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn e001(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E001, message)
    }

    pub fn e002(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E002, message)
    }

    pub fn e003(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E003, message)
    }

    pub fn e004(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E004, message)
    }

    pub fn e005(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E005, message)
    }

    pub fn e006(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E006, message)
    }

    pub fn e007(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E007, message)
    }

    pub fn e008(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E008, message)
    }

    pub fn e009(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::E009, message)
    }
}

impl fmt::Display for DaemonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::E001.code(), "E001");
        assert_eq!(ErrorCode::E002.code(), "E002");
    }

    #[test]
    fn test_daemon_error_creation() {
        let err = DaemonError::e002("session not found");
        assert_eq!(err.code, ErrorCode::E002);
        assert_eq!(err.message, "session not found");
    }
}
