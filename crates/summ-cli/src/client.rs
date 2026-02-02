use anyhow::{Context, Result};
use summ_common::{Request, Response};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

/// Get the default socket path for the daemon
pub fn socket_path() -> std::path::PathBuf {
    dirs::home_dir()
        .expect("HOME directory not found")
        .join(".summ-daemon/daemon.sock")
}

/// Send a request to the daemon and receive the response
/// Uses length-prefixed framing: [4 bytes length][JSON payload]
pub async fn send_request(request: Request) -> Result<Response> {
    let socket = socket_path();
    let mut stream = UnixStream::connect(&socket)
        .await
        .context(format!("Failed to connect to daemon at {:?}", socket))?;

    // Serialize request to JSON
    let json_bytes = serde_json::to_vec(&request)
        .context("Failed to serialize request")?;

    // Write length prefix (big-endian u32)
    let len = json_bytes.len() as u32;
    stream.write_all(&len.to_be_bytes())
        .await
        .context("Failed to write request length")?;

    // Write JSON payload
    stream.write_all(&json_bytes)
        .await
        .context("Failed to write request payload")?;

    // Read response length
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)
        .await
        .context("Failed to read response length")?;
    let resp_len = u32::from_be_bytes(len_buf) as usize;

    // Read response payload
    let mut resp_buf = vec![0u8; resp_len];
    stream.read_exact(&mut resp_buf)
        .await
        .context("Failed to read response payload")?;

    // Deserialize response
    let response: Response = serde_json::from_slice(&resp_buf)
        .context("Failed to deserialize response")?;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path() {
        let path = socket_path();
        assert!(path.ends_with(".summ-daemon/daemon.sock"));
    }
}
