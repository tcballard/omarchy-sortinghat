//! Linux filesystem primitives. Callers must journal durable intent before mutation.

use rustix::fs::{AtFlags, RenameFlags, renameat_with, statat};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};
use thiserror::Error;

pub const MAX_FILE_SIZE: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum FsError {
    #[error("unsafe path")]
    UnsafePath,
    #[error("source is not a single-link regular file")]
    UnsafeSource,
    #[error("file exceeds inspection limit")]
    Oversized,
    #[error("destination exists")]
    Collision,
    #[error("filesystem error: {0}")]
    Io(#[from] io::Error),
    #[error("source identity changed")]
    IdentityChanged,
    #[error("verification failed")]
    VerificationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identity {
    pub device: u64,
    pub inode: u64,
    pub size: u64,
    pub modified_ns: i128,
}

impl Identity {
    pub fn read(path: &Path) -> Result<Self, FsError> {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.file_type().is_file() || metadata.nlink() != 1 {
            return Err(FsError::UnsafeSource);
        }
        if metadata.len() > MAX_FILE_SIZE {
            return Err(FsError::Oversized);
        }
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            size: metadata.len(),
            modified_ns: i128::from(metadata.mtime()) * 1_000_000_000
                + i128::from(metadata.mtime_nsec()),
        })
    }
}

#[derive(Debug, Default)]
pub struct StabilityTracker {
    previous: Option<(Identity, SystemTime)>,
}

impl StabilityTracker {
    pub fn sample(&mut self, path: &Path, settle: Duration) -> Result<bool, FsError> {
        let identity = Identity::read(path)?;
        let now = SystemTime::now();
        let stable = self.previous.is_some_and(|(old, at)| {
            old == identity && now.duration_since(at).unwrap_or_default() >= settle
        });
        self.previous = Some((identity, now));
        Ok(stable)
    }
}

pub fn validate_relative(path: &Path) -> Result<(), FsError> {
    if path.as_os_str().is_empty() || path.as_os_str().len() > 4_096 || path.is_absolute() {
        return Err(FsError::UnsafePath);
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(FsError::UnsafePath);
        }
    }
    Ok(())
}

pub fn validate_beneath(root: &Path, relative: &Path) -> Result<PathBuf, FsError> {
    validate_relative(relative)?;
    let root = root.canonicalize()?;
    let candidate = root.join(relative);
    let parent = candidate
        .parent()
        .ok_or(FsError::UnsafePath)?
        .canonicalize()?;
    if !parent.starts_with(&root) {
        return Err(FsError::UnsafePath);
    }
    Ok(candidate)
}

pub fn same_filesystem_move(
    source: &Path,
    destination: &Path,
    expected: Identity,
) -> Result<(), FsError> {
    if Identity::read(source)? != expected {
        return Err(FsError::IdentityChanged);
    }
    if destination.parent().is_none() {
        return Err(FsError::UnsafePath);
    }
    match renameat_with(
        rustix::fs::CWD,
        source,
        rustix::fs::CWD,
        destination,
        RenameFlags::NOREPLACE,
    ) {
        Ok(()) => {
            File::open(destination.parent().expect("checked"))?.sync_all()?;
            if Identity::read(destination)?.size != expected.size {
                return Err(FsError::VerificationFailed);
            }
            Ok(())
        }
        Err(rustix::io::Errno::EXIST) => Err(FsError::Collision),
        Err(error) => Err(FsError::Io(io::Error::from_raw_os_error(
            error.raw_os_error(),
        ))),
    }
}

pub fn sha256(path: &Path) -> Result<String, FsError> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Copy to an exclusive staging path and verify it. Publication and source retirement are
/// intentionally separate so the journal can persist each state transition.
pub fn verified_stage_copy(
    source: &Path,
    staging: &Path,
    expected: Identity,
) -> Result<String, FsError> {
    if Identity::read(source)? != expected {
        return Err(FsError::IdentityChanged);
    }
    let input = File::open(source)?;
    let output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(staging)
        .map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                FsError::Collision
            } else {
                error.into()
            }
        })?;
    let mut reader = BufReader::new(input);
    let mut writer = BufWriter::new(output);
    let mut hasher = Sha256::new();
    let mut copied = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        copied = copied.checked_add(count as u64).ok_or(FsError::Oversized)?;
        if copied > MAX_FILE_SIZE {
            return Err(FsError::Oversized);
        }
        writer.write_all(&buffer[..count])?;
        hasher.update(&buffer[..count]);
    }
    writer.flush()?;
    writer.get_ref().sync_all()?;
    if copied != expected.size {
        return Err(FsError::IdentityChanged);
    }
    let digest = hex::encode(hasher.finalize());
    if sha256(staging)? != digest {
        return Err(FsError::VerificationFailed);
    }
    Ok(digest)
}

pub fn publish_stage(staging: &Path, destination: &Path) -> Result<(), FsError> {
    match renameat_with(
        rustix::fs::CWD,
        staging,
        rustix::fs::CWD,
        destination,
        RenameFlags::NOREPLACE,
    ) {
        Ok(()) => {
            File::open(destination.parent().ok_or(FsError::UnsafePath)?)?.sync_all()?;
            Ok(())
        }
        Err(rustix::io::Errno::EXIST) => Err(FsError::Collision),
        Err(error) => Err(FsError::Io(io::Error::from_raw_os_error(
            error.raw_os_error(),
        ))),
    }
}

pub fn source_matches(source: &Path, expected: Identity) -> Result<bool, FsError> {
    Ok(Identity::read(source)? == expected)
}

pub fn destination_exists_case_folded(destination: &Path) -> Result<bool, FsError> {
    let name = destination
        .file_name()
        .ok_or(FsError::UnsafePath)?
        .to_string_lossy()
        .to_lowercase();
    let parent = destination.parent().ok_or(FsError::UnsafePath)?;
    for entry in fs::read_dir(parent)? {
        if entry?.file_name().to_string_lossy().to_lowercase() == name {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn path_is_symlink(path: &Path) -> Result<bool, FsError> {
    let stat = statat(rustix::fs::CWD, path, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(|e| FsError::Io(io::Error::from_raw_os_error(e.raw_os_error())))?;
    Ok((stat.st_mode & rustix::fs::FileType::Symlink.as_raw_mode()) != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_fs_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let destination = dir.path().join("dest");
        fs::write(&source, b"new").unwrap();
        fs::write(&destination, b"old").unwrap();
        let result = same_filesystem_move(&source, &destination, Identity::read(&source).unwrap());
        assert!(matches!(result, Err(FsError::Collision)));
        assert_eq!(fs::read(&destination).unwrap(), b"old");
    }

    #[test]
    fn staging_copy_is_verified() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let staging = dir.path().join("stage");
        fs::write(&source, b"payload").unwrap();
        let digest =
            verified_stage_copy(&source, &staging, Identity::read(&source).unwrap()).unwrap();
        assert_eq!(digest, sha256(&source).unwrap());
    }

    #[test]
    fn traversal_and_symlinks_are_rejected() {
        assert!(validate_relative(Path::new("../escape")).is_err());
        let dir = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("/tmp", dir.path().join("link")).unwrap();
        assert!(path_is_symlink(&dir.path().join("link")).unwrap());
    }
}
