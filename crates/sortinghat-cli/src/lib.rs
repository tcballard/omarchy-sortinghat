use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

pub const MAX_IPC_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Request {
    pub schema_version: u8,
    pub correlation_id: Uuid,
    pub action: String,
    pub proposal_id: Option<Uuid>,
    pub revision: Option<u64>,
    pub argument: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Response {
    pub schema_version: u8,
    pub correlation_id: Uuid,
    pub ok: bool,
    pub state: String,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("I/O: {0}")]
    Io(#[from] io::Error),
    #[error("invalid or oversized response")]
    InvalidResponse,
    #[error("correlation mismatch")]
    CorrelationMismatch,
}

pub fn call(socket: &Path, request: &Request) -> Result<Response, ProtocolError> {
    let body = serde_json::to_vec(request).map_err(|_| ProtocolError::InvalidResponse)?;
    if body.len() >= MAX_IPC_BYTES {
        return Err(ProtocolError::InvalidResponse);
    }
    let mut stream = UnixStream::connect(socket)?;
    stream.write_all(&body)?;
    stream.write_all(b"\n")?;
    let mut reader = BufReader::new(stream).take((MAX_IPC_BYTES + 1) as u64);
    let mut response = Vec::new();
    reader.read_until(b'\n', &mut response)?;
    if response.len() > MAX_IPC_BYTES {
        return Err(ProtocolError::InvalidResponse);
    }
    let response: Response =
        serde_json::from_slice(&response).map_err(|_| ProtocolError::InvalidResponse)?;
    if response.schema_version != 1 {
        return Err(ProtocolError::InvalidResponse);
    }
    if response.correlation_id != request.correlation_id {
        return Err(ProtocolError::CorrelationMismatch);
    }
    Ok(response)
}
