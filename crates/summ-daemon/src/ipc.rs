// summ-daemon/src/ipc.rs
// IPC protocol handler for Unix socket communication
use anyhow::{Context, Result};
use summ_common::{Request, Response};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

const MAX_REQUEST_SIZE: usize = 16 * 1024 * 1024; // 16MB

/// Read a length-prefixed JSON request from the stream
pub async fn read_request(stream: &mut UnixStream) -> Result<Request> {
    // Read the 4-byte length prefix (big-endian u32)
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .await
        .context("Failed to read request length")?;

    let len = u32::from_be_bytes(len_buf) as usize;

    // Validate length
    if len > MAX_REQUEST_SIZE {
        anyhow::bail!(
            "Request size {} exceeds maximum allowed size of {}",
            len,
            MAX_REQUEST_SIZE
        );
    }

    if len == 0 {
        anyhow::bail!("Received empty request");
    }

    // Read the JSON payload
    let mut buf = vec![0u8; len];
    stream
        .read_exact(&mut buf)
        .await
        .context("Failed to read request payload")?;

    // Parse JSON
    let request: Request = serde_json::from_slice(&buf)
        .context("Failed to parse request JSON")?;

    Ok(request)
}

/// Write a length-prefixed JSON response to the stream
pub async fn write_response(stream: &mut UnixStream, response: &Response) -> Result<()> {
    // Serialize response to JSON
    let json_bytes = serde_json::to_vec(response)
        .context("Failed to serialize response")?;

    // Validate response size
    if json_bytes.len() > MAX_REQUEST_SIZE {
        anyhow::bail!(
            "Response size {} exceeds maximum allowed size of {}",
            json_bytes.len(),
            MAX_REQUEST_SIZE
        );
    }

    // Write length prefix (big-endian u32)
    let len = json_bytes.len() as u32;
    let len_buf = len.to_be_bytes();
    stream
        .write_all(&len_buf)
        .await
        .context("Failed to write response length")?;

    // Write JSON payload
    stream
        .write_all(&json_bytes)
        .await
        .context("Failed to write response payload")?;

    // Flush to ensure data is sent
    stream
        .flush()
        .await
        .context("Failed to flush response")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use summ_common::{Request, Response};
    use std::path::PathBuf;
    use tokio::net::{UnixListener, UnixStream};
    use tokio::task;

    #[tokio::test]
    async fn test_request_serialization() {
        let request = Request::Start {
            cli: "claude".to_string(),
            init: PathBuf::from("/path/to/init"),
            name: Some("test-session".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(r#""type":"Start""#));
        assert!(json.contains(r#""cli":"claude""#));
        assert!(json.contains(r#""name":"test-session""#));
    }

    #[tokio::test]
    async fn test_response_serialization() {
        let response = Response::Success {
            data: serde_json::json!({"session_id": "test123"}),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(r#""type":"Success""#));
        assert!(json.contains(r#""session_id":"test123""#));
    }

    #[tokio::test]
    async fn test_length_prefix_encoding() {
        // Test that length prefix is correctly encoded
        let len: u32 = 1234;
        let bytes = len.to_be_bytes();
        let decoded = u32::from_be_bytes(bytes);
        assert_eq!(len, decoded);
    }

    #[tokio::test]
    async fn test_max_request_size() {
        // MAX_REQUEST_SIZE should be 16MB
        assert_eq!(MAX_REQUEST_SIZE, 16 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_read_write_request_response() {
        let temp_dir = tempfile::tempdir().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        // Create listener
        let listener = UnixListener::bind(&socket_path).unwrap();

        // Spawn server task
        let server_handle = task::spawn(async move {
            let mut stream = listener.accept().await.unwrap().0;
            let request = read_request(&mut stream).await.unwrap();
            match request {
                Request::DaemonStatus => {
                    let response = Response::Success {
                        data: serde_json::json!({"running": true, "version": "0.1.0"}),
                    };
                    write_response(&mut stream, &response).await.unwrap();
                }
                _ => panic!("Unexpected request"),
            }
        });

        // Connect and send request
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();

        let request = Request::DaemonStatus;
        let json_bytes = serde_json::to_vec(&request).unwrap();
        let len = json_bytes.len() as u32;

        // Write length prefix and payload
        stream.write_all(&len.to_be_bytes()).await.unwrap();
        stream.write_all(&json_bytes).await.unwrap();
        stream.flush().await.unwrap();

        // Read response
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.unwrap();
        let response_len = u32::from_be_bytes(len_buf) as usize;

        let mut response_buf = vec![0u8; response_len];
        stream.read_exact(&mut response_buf).await.unwrap();

        let response: Response = serde_json::from_slice(&response_buf).unwrap();
        match response {
            Response::Success { data } => {
                assert_eq!(data["running"], true);
                assert_eq!(data["version"], "0.1.0");
            }
            _ => panic!("Expected Success response"),
        }

        // Wait for server to finish
        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_read_request_with_zero_length() {
        let temp_dir = tempfile::tempdir().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let listener = UnixListener::bind(&socket_path).unwrap();

        let server_handle = task::spawn(async move {
            let mut stream = listener.accept().await.unwrap().0;
            let result = read_request(&mut stream).await;
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("empty"));
        });

        let mut stream = UnixStream::connect(&socket_path).await.unwrap();

        // Send zero-length request
        stream.write_all(&0u32.to_be_bytes()).await.unwrap();
        stream.flush().await.unwrap();

        server_handle.await.unwrap();
    }
}
