//! Optional metadata-first agent adapter. Disabled unless explicitly configured.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use thiserror::Error;
use uuid::Uuid;

pub const MAX_METADATA_BYTES: usize = 16 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AgentRequest {
    pub schema_version: u8,
    pub correlation_id: Uuid,
    pub filename_extension: Option<String>,
    pub verified_mime: Option<String>,
    pub size_bucket: String,
    pub source_root_id: Uuid,
    pub allowed_destination_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AgentResponse {
    pub schema_version: u8,
    pub correlation_id: Uuid,
    pub destination_id: Option<Uuid>,
    pub reason: String,
}

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("agent is disabled")]
    Disabled,
    #[error("request exceeds metadata limit")]
    RequestTooLarge,
    #[error("agent process failed: {0}")]
    Process(#[from] io::Error),
    #[error("agent output is malformed")]
    Malformed,
    #[error("agent selected an unregistered destination")]
    UnregisteredDestination,
}

#[derive(Debug, Clone)]
pub struct Adapter {
    executable: Option<String>,
    fixed_args: Vec<String>,
}

impl Adapter {
    pub fn disabled() -> Self {
        Self {
            executable: None,
            fixed_args: vec![],
        }
    }

    pub fn local(executable: String, fixed_args: Vec<String>) -> Self {
        Self {
            executable: Some(executable),
            fixed_args,
        }
    }

    pub fn classify(&self, request: &AgentRequest) -> Result<AgentResponse, AgentError> {
        let executable = self.executable.as_ref().ok_or(AgentError::Disabled)?;
        let body = serde_json::to_vec(request).map_err(|_| AgentError::Malformed)?;
        if body.len() > MAX_METADATA_BYTES {
            return Err(AgentError::RequestTooLarge);
        }
        let mut child = Command::new(executable)
            .args(&self.fixed_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        child
            .stdin
            .take()
            .ok_or(AgentError::Malformed)?
            .write_all(&body)?;
        let output = child.wait_with_output()?;
        if !output.status.success() || output.stdout.len() > MAX_OUTPUT_BYTES {
            return Err(AgentError::Malformed);
        }
        let response: AgentResponse =
            serde_json::from_slice(&output.stdout).map_err(|_| AgentError::Malformed)?;
        if response.schema_version != 1
            || response.correlation_id != request.correlation_id
            || response.reason.len() > 2_048
        {
            return Err(AgentError::Malformed);
        }
        let allowed = request
            .allowed_destination_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if response
            .destination_id
            .is_some_and(|id| !allowed.contains(&id))
        {
            return Err(AgentError::UnregisteredDestination);
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_agent_is_honest() {
        let request = AgentRequest {
            schema_version: 1,
            correlation_id: Uuid::new_v4(),
            filename_extension: None,
            verified_mime: None,
            size_bucket: "small".into(),
            source_root_id: Uuid::nil(),
            allowed_destination_ids: vec![],
        };
        assert!(matches!(
            Adapter::disabled().classify(&request),
            Err(AgentError::Disabled)
        ));
    }

    #[test]
    fn malformed_output_is_rejected() {
        let request = AgentRequest {
            schema_version: 1,
            correlation_id: Uuid::new_v4(),
            filename_extension: None,
            verified_mime: None,
            size_bucket: "small".into(),
            source_root_id: Uuid::nil(),
            allowed_destination_ids: vec![],
        };
        let adapter = Adapter::local("/bin/cat".into(), vec![]);
        assert!(matches!(
            adapter.classify(&request),
            Err(AgentError::Malformed)
        ));
    }
}
