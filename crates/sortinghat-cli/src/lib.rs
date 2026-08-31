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
    pub data: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("I/O: {0}")]
    Io(#[from] io::Error),
    #[error("invalid or oversized response")]
    InvalidResponse,
    #[error("correlation mismatch")]
    CorrelationMismatch,
    #[error("invalid request")]
    InvalidRequest,
}

pub fn call(socket: &Path, request: &Request) -> Result<Response, ProtocolError> {
    validate_request(request)?;
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

pub fn validate_request(request: &Request) -> Result<(), ProtocolError> {
    if request.schema_version != 1
        || request.correlation_id.is_nil()
        || request.action.is_empty()
        || request.action.len() > 64
        || !request
            .action
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
        || request
            .argument
            .as_ref()
            .is_some_and(|value| value.len() > 16 * 1024)
    {
        return Err(ProtocolError::InvalidRequest);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> Request {
        Request {
            schema_version: 1,
            correlation_id: Uuid::new_v4(),
            action: "status".into(),
            proposal_id: None,
            revision: None,
            argument: None,
        }
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let body = format!(
            r#"{{"schema_version":1,"correlation_id":"{}","action":"status","proposal_id":null,"revision":null,"argument":null,"extra":true}}"#,
            Uuid::new_v4()
        );
        assert!(serde_json::from_str::<Request>(&body).is_err());
    }

    #[test]
    fn version_nil_correlation_and_oversized_arguments_are_rejected() {
        let mut invalid = request();
        invalid.schema_version = 2;
        assert!(validate_request(&invalid).is_err());
        invalid.schema_version = 1;
        invalid.correlation_id = Uuid::nil();
        assert!(validate_request(&invalid).is_err());
        invalid.correlation_id = Uuid::new_v4();
        invalid.argument = Some("x".repeat(16 * 1024 + 1));
        assert!(validate_request(&invalid).is_err());
    }
}
