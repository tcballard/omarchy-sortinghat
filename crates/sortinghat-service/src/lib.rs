//! Persistent review queue and daemon orchestration.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use sortinghat_core::{FileFacts, Rule, RuleDecision, evaluate_rules};
use sortinghat_fs::{
    FsError, Identity, StabilityTracker, destination_exists_case_folded, open_beneath, open_root,
    publish_stage, retire_source, same_filesystem_move, sha256, verified_stage_copy,
};
use sortinghat_journal::{Entry, Journal, JournalError, State as JournalState};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

const MAX_ROOTS: usize = 16;
const MAX_PROPOSALS: usize = 1_000;
const MAX_WALK_DIRECTORIES: usize = 25_000;
const MAX_WALK_DEPTH: usize = 32;
const MAX_STATE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("filesystem: {0}")]
    Fs(#[from] FsError),
    #[error("journal: {0}")]
    Journal(#[from] JournalError),
    #[error("I/O: {0}")]
    Io(#[from] io::Error),
    #[error("state is malformed or too large")]
    MalformedState,
    #[error("configured limit reached")]
    Limit,
    #[error("item not found")]
    NotFound,
    #[error("revision conflict")]
    RevisionConflict,
    #[error("operation requires a destination")]
    DestinationRequired,
    #[error("destination collision")]
    Collision,
    #[error("unsafe or unregistered path")]
    UnsafePath,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Root {
    pub id: Uuid,
    pub path: String,
    pub watched: bool,
    pub destination: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DestinationChoice {
    pub root_id: Uuid,
    pub directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Proposal {
    pub id: Uuid,
    pub revision: u64,
    pub source_root_id: Uuid,
    pub source_relative: String,
    pub source_display: String,
    pub destination: Option<DestinationChoice>,
    pub destination_name: Option<String>,
    pub destination_display: Option<String>,
    pub reason: String,
    pub provenance: String,
    pub warning: Option<String>,
    pub status: String,
    pub size: u64,
    pub device: u64,
    pub inode: u64,
    pub modified_ns: i128,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct PersistentState {
    schema_version: u8,
    paused: bool,
    roots: Vec<Root>,
    rules: Vec<Rule>,
    proposals: Vec<Proposal>,
}

pub struct Service {
    state_dir: PathBuf,
    state: PersistentState,
    journal: Journal,
    stability: HashMap<String, StabilityTracker>,
}

impl Service {
    pub fn open(state_dir: &Path) -> Result<Self, ServiceError> {
        fs::create_dir_all(state_dir)?;
        let state_path = state_dir.join("state.json");
        let state = if state_path.exists() {
            let file = File::open(&state_path)?;
            if file.metadata()?.len() > MAX_STATE_BYTES {
                return Err(ServiceError::MalformedState);
            }
            let mut bytes = Vec::new();
            file.take(MAX_STATE_BYTES + 1).read_to_end(&mut bytes)?;
            let state: PersistentState =
                serde_json::from_slice(&bytes).map_err(|_| ServiceError::MalformedState)?;
            if state.schema_version != 1 {
                return Err(ServiceError::MalformedState);
            }
            state
        } else {
            PersistentState {
                schema_version: 1,
                ..PersistentState::default()
            }
        };
        let journal = Journal::open(&state_dir.join("journal.sqlite3"))?;
        let mut service = Self {
            state_dir: state_dir.to_path_buf(),
            state,
            journal,
            stability: HashMap::new(),
        };
        service.recover_without_guessing()?;
        Ok(service)
    }

    pub const fn paused(&self) -> bool {
        self.state.paused
    }

    pub fn set_paused(&mut self, paused: bool) -> Result<(), ServiceError> {
        self.state.paused = paused;
        self.save()
    }

    pub fn roots(&self) -> &[Root] {
        &self.state.roots
    }

    pub fn rules(&self) -> &[Rule] {
        &self.state.rules
    }

    pub fn proposals(&self) -> &[Proposal] {
        &self.state.proposals
    }

    pub fn proposal(&self, id: Uuid) -> Result<&Proposal, ServiceError> {
        self.state
            .proposals
            .iter()
            .find(|proposal| proposal.id == id)
            .ok_or(ServiceError::NotFound)
    }

    pub fn add_root(
        &mut self,
        path: &Path,
        watched: bool,
        destination: bool,
    ) -> Result<Root, ServiceError> {
        if self.state.roots.len() >= MAX_ROOTS || (!watched && !destination) {
            return Err(ServiceError::Limit);
        }
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(ServiceError::UnsafePath);
        }
        let canonical = path.canonicalize()?;
        let display = canonical
            .to_str()
            .ok_or(ServiceError::UnsafePath)?
            .to_owned();
        if self.state.roots.iter().any(|root| root.path == display) {
            return Err(ServiceError::Collision);
        }
        open_root(&canonical)?;
        let root = Root {
            id: Uuid::new_v4(),
            path: display,
            watched,
            destination,
        };
        self.state.roots.push(root.clone());
        self.save()?;
        Ok(root)
    }

    pub fn remove_root(&mut self, id: Uuid) -> Result<(), ServiceError> {
        if self
            .state
            .proposals
            .iter()
            .any(|proposal| proposal.source_root_id == id && proposal.status == "proposed")
        {
            return Err(ServiceError::Collision);
        }
        let before = self.state.roots.len();
        self.state.roots.retain(|root| root.id != id);
        if self.state.roots.len() == before {
            return Err(ServiceError::NotFound);
        }
        self.save()
    }

    pub fn scan_once(&mut self) -> Result<usize, ServiceError> {
        if self.state.paused {
            return Ok(0);
        }
        if self
            .state
            .proposals
            .iter()
            .filter(|proposal| proposal.status == "proposed")
            .count()
            >= MAX_PROPOSALS
        {
            self.state.paused = true;
            self.save()?;
            return Err(ServiceError::Limit);
        }
        let roots = self
            .state
            .roots
            .iter()
            .filter(|root| root.watched)
            .cloned()
            .collect::<Vec<_>>();
        let mut created = 0;
        for root in roots {
            for relative in bounded_files(Path::new(&root.path))? {
                if self.consider(&root, &relative)? {
                    created += 1;
                }
            }
        }
        if created > 0 {
            self.save()?;
        }
        Ok(created)
    }

    pub fn choose_destination(
        &mut self,
        id: Uuid,
        revision: u64,
        root_id: Uuid,
        directory: &str,
    ) -> Result<Proposal, ServiceError> {
        sortinghat_core::validate_relative_directory(directory)
            .map_err(|_| ServiceError::UnsafePath)?;
        let root = self
            .state
            .roots
            .iter()
            .find(|root| root.id == root_id && root.destination)
            .ok_or(ServiceError::UnsafePath)?
            .clone();
        let folder = Path::new(&root.path).join(directory);
        let metadata = fs::symlink_metadata(&folder)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(ServiceError::UnsafePath);
        }
        let index = self.proposal_index(id, revision)?;
        let source_name = decode_relative(&self.state.proposals[index].source_relative)?
            .file_name()
            .ok_or(ServiceError::UnsafePath)?
            .to_owned();
        let destination_path = folder.join(source_name);
        let warning = move_warning(&self.source_path(index)?, &destination_path).ok();
        let destination_token = destination_path.as_os_str().as_bytes();
        let journal_entry = self
            .journal
            .revise_proposal(id, revision, destination_token)?;
        let proposal = &mut self.state.proposals[index];
        proposal.revision = journal_entry.revision;
        proposal.destination = Some(DestinationChoice {
            root_id,
            directory: directory.to_owned(),
        });
        proposal.destination_name = destination_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        proposal.destination_display = Some(display_path(&destination_path));
        proposal.warning = warning;
        let result = proposal.clone();
        self.save()?;
        Ok(result)
    }

    pub fn rename(
        &mut self,
        id: Uuid,
        revision: u64,
        name: &str,
    ) -> Result<Proposal, ServiceError> {
        let name_path = Path::new(name);
        if name.len() > 255
            || name_path.components().count() != 1
            || !matches!(
                name_path.components().next(),
                Some(std::path::Component::Normal(_))
            )
        {
            return Err(ServiceError::UnsafePath);
        }
        let index = self.proposal_index(id, revision)?;
        let choice = self.state.proposals[index]
            .destination
            .clone()
            .ok_or(ServiceError::DestinationRequired)?;
        let root = self
            .state
            .roots
            .iter()
            .find(|root| root.id == choice.root_id && root.destination)
            .ok_or(ServiceError::UnsafePath)?;
        let destination_path = Path::new(&root.path).join(&choice.directory).join(name);
        let entry =
            self.journal
                .revise_proposal(id, revision, destination_path.as_os_str().as_bytes())?;
        self.state.proposals[index].revision = entry.revision;
        self.state.proposals[index].destination_name = Some(name.to_owned());
        self.state.proposals[index].destination_display = Some(display_path(&destination_path));
        let result = self.state.proposals[index].clone();
        self.save()?;
        Ok(result)
    }

    pub fn ignore(&mut self, id: Uuid, revision: u64) -> Result<Proposal, ServiceError> {
        let index = self.proposal_index(id, revision)?;
        let entry = self
            .journal
            .transition(id, revision, JournalState::Ignored)?;
        self.state.proposals[index].revision = entry.revision;
        self.state.proposals[index].status = "ignored".into();
        let result = self.state.proposals[index].clone();
        self.save()?;
        Ok(result)
    }

    pub fn create_rule_from_proposal(
        &mut self,
        id: Uuid,
        priority: i32,
    ) -> Result<Rule, ServiceError> {
        if self.state.rules.len() >= sortinghat_core::MAX_RULES {
            return Err(ServiceError::Limit);
        }
        let proposal = self.proposal(id)?.clone();
        let destination = proposal
            .destination
            .ok_or(ServiceError::DestinationRequired)?;
        let source = decode_relative(&proposal.source_relative)?;
        let extension = source
            .extension()
            .and_then(|value| value.to_str())
            .ok_or(ServiceError::UnsafePath)?;
        let rule = Rule {
            id: Uuid::new_v4(),
            priority,
            predicates: vec![sortinghat_core::Predicate::Extension {
                value: extension.to_ascii_lowercase(),
            }],
            destination_root_id: destination.root_id,
            destination_directory: destination.directory,
        };
        evaluate_rules(
            std::slice::from_ref(&rule),
            &facts_for(&source, proposal.source_root_id, true),
        )
        .map_err(|_| ServiceError::MalformedState)?;
        self.state.rules.push(rule.clone());
        self.save()?;
        Ok(rule)
    }

    pub fn approve(&mut self, id: Uuid, revision: u64) -> Result<Proposal, ServiceError> {
        let index = self.proposal_index(id, revision)?;
        let proposal = self.state.proposals[index].clone();
        let choice = proposal
            .destination
            .clone()
            .ok_or(ServiceError::DestinationRequired)?;
        let source = self.source_path(index)?;
        let destination_root = self
            .state
            .roots
            .iter()
            .find(|root| root.id == choice.root_id && root.destination)
            .ok_or(ServiceError::UnsafePath)?;
        let source_root = self
            .state
            .roots
            .iter()
            .find(|root| root.id == proposal.source_root_id && root.watched)
            .ok_or(ServiceError::UnsafePath)?;
        let relative = decode_relative(&proposal.source_relative)?;
        let root_fd = open_root(Path::new(&source_root.path))?;
        let _verified_fd = open_beneath(&root_fd, &relative, rustix::fs::OFlags::RDONLY)?;
        let folder = Path::new(&destination_root.path).join(&choice.directory);
        let folder_metadata = fs::symlink_metadata(&folder)?;
        if !folder_metadata.is_dir() || folder_metadata.file_type().is_symlink() {
            return Err(ServiceError::UnsafePath);
        }
        let destination_root_fd = open_root(Path::new(&destination_root.path))?;
        let _destination_folder_fd = open_beneath(
            &destination_root_fd,
            Path::new(&choice.directory),
            rustix::fs::OFlags::PATH | rustix::fs::OFlags::DIRECTORY,
        )?;
        let destination = folder.join(
            proposal
                .destination_name
                .as_deref()
                .map(OsString::from)
                .unwrap_or_else(|| {
                    relative
                        .file_name()
                        .expect("validated relative path has a name")
                        .to_owned()
                }),
        );
        if destination_exists_case_folded(&destination)? {
            return Err(ServiceError::Collision);
        }
        let expected = proposal_identity(&proposal);
        let mut entry = self
            .journal
            .transition(id, revision, JournalState::Approved)?;
        self.state.proposals[index].revision = entry.revision;
        if expected.device == folder.metadata()?.dev() {
            same_filesystem_move(&source, &destination, expected)?;
            entry = self
                .journal
                .transition(id, entry.revision, JournalState::Published)?;
        } else {
            entry = self
                .journal
                .transition(id, entry.revision, JournalState::Copying)?;
            let staging = folder.join(format!(".sortinghat-{id}.stage"));
            let digest = verified_stage_copy(&source, &staging, expected)?;
            self.journal.set_sha256(id, &digest)?;
            publish_stage(&staging, &destination)?;
            if sha256(&destination)? != digest {
                return Err(ServiceError::Fs(FsError::VerificationFailed));
            }
            entry = self
                .journal
                .transition(id, entry.revision, JournalState::Published)?;
            retire_source(&source, expected, &digest)?;
        }
        entry = self
            .journal
            .transition(id, entry.revision, JournalState::SourceRemoved)?;
        entry = self
            .journal
            .transition(id, entry.revision, JournalState::Completed)?;
        self.state.proposals[index].revision = entry.revision;
        self.state.proposals[index].status = "completed".into();
        let result = self.state.proposals[index].clone();
        self.save()?;
        Ok(result)
    }

    pub fn undo(&mut self, id: Uuid, revision: u64) -> Result<Proposal, ServiceError> {
        let index = self.proposal_index(id, revision)?;
        if self.state.proposals[index].status != "completed" {
            return Err(ServiceError::RevisionConflict);
        }
        let source = self.source_path(index)?;
        let destination = self.destination_path(index)?;
        if source.exists() || destination_exists_case_folded(&source)? {
            return Err(ServiceError::Collision);
        }
        let expected = Identity::read(&destination)?;
        let mut entry = self
            .journal
            .transition(id, revision, JournalState::Undoing)?;
        same_filesystem_move(&destination, &source, expected)?;
        entry = self
            .journal
            .transition(id, entry.revision, JournalState::Undone)?;
        self.state.proposals[index].revision = entry.revision;
        self.state.proposals[index].status = "undone".into();
        let result = self.state.proposals[index].clone();
        self.save()?;
        Ok(result)
    }

    fn consider(&mut self, root: &Root, relative: &Path) -> Result<bool, ServiceError> {
        if is_partial(relative) {
            return Ok(false);
        }
        let encoded = encode_relative(relative);
        if self.state.proposals.iter().any(|proposal| {
            proposal.source_root_id == root.id
                && proposal.source_relative == encoded
                && proposal.status == "proposed"
        }) {
            return Ok(false);
        }
        let absolute = Path::new(&root.path).join(relative);
        let key = format!("{}:{encoded}", root.id);
        if !self
            .stability
            .entry(key)
            .or_default()
            .sample(&absolute, Duration::from_secs(1))?
        {
            return Ok(false);
        }
        let identity = Identity::read(&absolute)?;
        let mut facts = facts_for(relative, root.id, true);
        facts.verified_mime = sniff_mime(&absolute);
        let decision =
            evaluate_rules(&self.state.rules, &facts).map_err(|_| ServiceError::MalformedState)?;
        let (destination, reason, provenance, mut warning) = match decision {
            RuleDecision::Destination {
                destination,
                rule_ids,
                reason,
            } => (
                Some(DestinationChoice {
                    root_id: destination.root_id,
                    directory: destination.directory,
                }),
                reason,
                format!("rule:{}", rule_ids[0]),
                None,
            ),
            RuleDecision::Tie { reason, .. } => (
                None,
                reason,
                "deterministic_tie".into(),
                Some("Ambiguous: choose a destination before approval".into()),
            ),
            RuleDecision::Abstain { reason } => (
                None,
                reason,
                "deterministic_abstain".into(),
                Some("No rule decided; agent classification was not requested".into()),
            ),
        };
        let destination_display = destination
            .as_ref()
            .and_then(|choice| self.destination_display(choice, relative));
        if let Some(destination_path) = destination
            .as_ref()
            .and_then(|choice| self.destination_path_for(choice, relative))
        {
            warning = move_warning(&absolute, &destination_path).ok();
        }
        let id = Uuid::new_v4();
        let proposal = Proposal {
            id,
            revision: 1,
            source_root_id: root.id,
            source_relative: encoded,
            source_display: display_path(&absolute),
            destination: destination.clone(),
            destination_name: relative
                .file_name()
                .map(|name| name.to_string_lossy().into_owned()),
            destination_display,
            reason,
            provenance,
            warning,
            status: "proposed".into(),
            size: identity.size,
            device: identity.device,
            inode: identity.inode,
            modified_ns: identity.modified_ns,
        };
        let destination_token = destination
            .as_ref()
            .and_then(|choice| self.destination_path_for(choice, relative))
            .map(|path| path.as_os_str().as_bytes().to_vec())
            .unwrap_or_default();
        self.journal.create(&Entry {
            id,
            revision: 1,
            state: JournalState::Proposed,
            source_token: absolute.as_os_str().as_bytes().to_vec(),
            destination_token,
            size: identity.size,
            sha256: None,
        })?;
        self.state.proposals.push(proposal);
        Ok(true)
    }

    fn proposal_index(&self, id: Uuid, revision: u64) -> Result<usize, ServiceError> {
        let index = self
            .state
            .proposals
            .iter()
            .position(|proposal| proposal.id == id)
            .ok_or(ServiceError::NotFound)?;
        if self.state.proposals[index].revision != revision {
            return Err(ServiceError::RevisionConflict);
        }
        Ok(index)
    }

    fn source_path(&self, index: usize) -> Result<PathBuf, ServiceError> {
        let proposal = &self.state.proposals[index];
        let root = self
            .state
            .roots
            .iter()
            .find(|root| root.id == proposal.source_root_id)
            .ok_or(ServiceError::UnsafePath)?;
        Ok(Path::new(&root.path).join(decode_relative(&proposal.source_relative)?))
    }

    fn destination_path(&self, index: usize) -> Result<PathBuf, ServiceError> {
        let proposal = &self.state.proposals[index];
        let relative = decode_relative(&proposal.source_relative)?;
        let choice = proposal
            .destination
            .as_ref()
            .ok_or(ServiceError::DestinationRequired)?;
        let root = self
            .state
            .roots
            .iter()
            .find(|root| root.id == choice.root_id && root.destination)
            .ok_or(ServiceError::UnsafePath)?;
        let name = proposal
            .destination_name
            .as_deref()
            .map(OsString::from)
            .or_else(|| relative.file_name().map(ToOwned::to_owned))
            .ok_or(ServiceError::UnsafePath)?;
        Ok(Path::new(&root.path).join(&choice.directory).join(name))
    }

    fn destination_path_for(&self, choice: &DestinationChoice, relative: &Path) -> Option<PathBuf> {
        let root = self
            .state
            .roots
            .iter()
            .find(|root| root.id == choice.root_id && root.destination)?;
        Some(
            Path::new(&root.path)
                .join(&choice.directory)
                .join(relative.file_name()?),
        )
    }

    fn destination_display(&self, choice: &DestinationChoice, relative: &Path) -> Option<String> {
        self.destination_path_for(choice, relative)
            .map(|path| display_path(&path))
    }

    fn recover_without_guessing(&mut self) -> Result<(), ServiceError> {
        for entry in self.journal.nonterminal()? {
            if matches!(
                entry.state,
                JournalState::Approved
                    | JournalState::Copying
                    | JournalState::Published
                    | JournalState::SourceRemoved
                    | JournalState::Undoing
            ) {
                let updated = self.journal.transition(
                    entry.id,
                    entry.revision,
                    JournalState::NeedsAttention,
                )?;
                if let Some(proposal) = self
                    .state
                    .proposals
                    .iter_mut()
                    .find(|proposal| proposal.id == entry.id)
                {
                    proposal.revision = updated.revision;
                    proposal.status = "needs_attention".into();
                    proposal.warning = Some(
                        "Interrupted operation requires identity/checksum review; recovery did not guess"
                            .into(),
                    );
                }
            }
        }
        self.save()
    }

    fn save(&self) -> Result<(), ServiceError> {
        let body =
            serde_json::to_vec_pretty(&self.state).map_err(|_| ServiceError::MalformedState)?;
        if body.len() as u64 > MAX_STATE_BYTES {
            return Err(ServiceError::Limit);
        }
        let temporary = self.state_dir.join("state.json.next");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(&body)?;
        file.sync_all()?;
        fs::rename(&temporary, self.state_dir.join("state.json"))?;
        File::open(&self.state_dir)?.sync_all()?;
        Ok(())
    }
}

fn bounded_files(root: &Path) -> Result<Vec<PathBuf>, ServiceError> {
    let mut result = Vec::new();
    let mut pending = vec![(PathBuf::new(), 0_usize)];
    let mut directories = 0_usize;
    while let Some((relative, depth)) = pending.pop() {
        directories += 1;
        if directories > MAX_WALK_DIRECTORIES || depth > MAX_WALK_DEPTH {
            return Err(ServiceError::Limit);
        }
        for entry in fs::read_dir(root.join(&relative))? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let child = relative.join(entry.file_name());
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push((child, depth + 1));
            } else if file_type.is_file() {
                result.push(child);
            }
        }
    }
    Ok(result)
}

fn facts_for(relative: &Path, root_id: Uuid, completed: bool) -> FileFacts {
    let extension = relative
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    FileFacts {
        extension,
        verified_mime: None,
        filename: relative
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default(),
        source_root_id: root_id,
        source_directory: relative
            .parent()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| ".".into()),
        completed,
    }
}

fn sniff_mime(path: &Path) -> Option<String> {
    let mut prefix = [0_u8; 16];
    let count = File::open(path).ok()?.read(&mut prefix).ok()?;
    let bytes = &prefix[..count];
    if bytes.starts_with(b"%PDF-") {
        Some("application/pdf".into())
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png".into())
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg".into())
    } else if bytes.starts_with(b"PK\x03\x04") {
        Some("application/zip".into())
    } else {
        None
    }
}

fn is_partial(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    [".part", ".partial", ".crdownload", ".download", ".tmp"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

fn encode_relative(path: &Path) -> String {
    URL_SAFE_NO_PAD.encode(path.as_os_str().as_bytes())
}

fn decode_relative(value: &str) -> Result<PathBuf, ServiceError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| ServiceError::MalformedState)?;
    let path = PathBuf::from(OsString::from_vec(bytes));
    sortinghat_fs::validate_relative(&path).map_err(|_| ServiceError::UnsafePath)?;
    Ok(path)
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .flat_map(char::escape_default)
        .take(4_096)
        .collect()
}

fn proposal_identity(proposal: &Proposal) -> Identity {
    Identity {
        device: proposal.device,
        inode: proposal.inode,
        size: proposal.size,
        modified_ns: proposal.modified_ns,
    }
}

fn move_warning(source: &Path, destination: &Path) -> Result<String, ServiceError> {
    let source_device = Identity::read(source)?.device;
    let destination_device = destination
        .parent()
        .ok_or(ServiceError::UnsafePath)?
        .metadata()?
        .dev();
    if source_device == destination_device {
        Ok("Same-filesystem atomic move; undo is available if neither path changes".into())
    } else {
        Ok(
            "Cross-filesystem verified copy; undo may fail if the original path becomes occupied"
                .into(),
        )
    }
}

trait DeviceMetadata {
    fn dev(&self) -> u64;
}

impl DeviceMetadata for fs::Metadata {
    fn dev(&self) -> u64 {
        std::os::unix::fs::MetadataExt::dev(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn only_explicit_roots_are_scanned_and_partial_files_are_ignored() {
        let temp = tempfile::tempdir().unwrap();
        let watched = temp.path().join("watched");
        let state = temp.path().join("state");
        fs::create_dir(&watched).unwrap();
        fs::write(watched.join("done.pdf"), b"%PDF-payload").unwrap();
        fs::write(watched.join("pending.crdownload"), b"partial").unwrap();
        let mut service = Service::open(&state).unwrap();
        assert_eq!(service.scan_once().unwrap(), 0);
        service.add_root(&watched, true, true).unwrap();
        assert_eq!(service.scan_once().unwrap(), 0);
        thread::sleep(Duration::from_millis(1_050));
        assert_eq!(service.scan_once().unwrap(), 1);
        assert_eq!(service.proposals().len(), 1);
    }

    #[test]
    fn restart_preserves_queue_and_pause_state() {
        let temp = tempfile::tempdir().unwrap();
        let state = temp.path().join("state");
        let mut service = Service::open(&state).unwrap();
        service.set_paused(true).unwrap();
        drop(service);
        assert!(Service::open(&state).unwrap().paused());
    }

    #[test]
    fn invalid_utf8_relative_names_round_trip() {
        let path = PathBuf::from(OsString::from_vec(vec![b'a', 0xff, b'b']));
        assert_eq!(decode_relative(&encode_relative(&path)).unwrap(), path);
    }

    #[test]
    fn approved_move_is_undoable_without_overwrite() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("files");
        let destination = root.join("sorted");
        let state = temp.path().join("state");
        fs::create_dir_all(&destination).unwrap();
        fs::write(root.join("report.pdf"), b"%PDF-payload").unwrap();
        let mut service = Service::open(&state).unwrap();
        let registered = service.add_root(&root, true, true).unwrap();
        service.scan_once().unwrap();
        thread::sleep(Duration::from_millis(1_050));
        service.scan_once().unwrap();
        let proposal = service.proposals()[0].clone();
        let proposal = service
            .choose_destination(proposal.id, proposal.revision, registered.id, "sorted")
            .unwrap();
        let completed = service.approve(proposal.id, proposal.revision).unwrap();
        assert!(!root.join("report.pdf").exists());
        assert!(destination.join("report.pdf").exists());
        service.undo(completed.id, completed.revision).unwrap();
        assert!(root.join("report.pdf").exists());
        assert!(!destination.join("report.pdf").exists());
    }

    #[test]
    fn growing_file_needs_a_new_stable_interval() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("watched");
        fs::create_dir(&root).unwrap();
        let file = root.join("growing.bin");
        fs::write(&file, b"one").unwrap();
        let mut service = Service::open(&temp.path().join("state")).unwrap();
        service.add_root(&root, true, false).unwrap();
        assert_eq!(service.scan_once().unwrap(), 0);
        thread::sleep(Duration::from_millis(1_050));
        fs::write(&file, b"one-two").unwrap();
        assert_eq!(service.scan_once().unwrap(), 0);
        thread::sleep(Duration::from_millis(1_050));
        assert_eq!(service.scan_once().unwrap(), 1);
    }
}
