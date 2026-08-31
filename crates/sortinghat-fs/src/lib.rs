//! Linux filesystem primitives. Callers must journal durable intent before mutation.

use rustix::fd::{AsFd, OwnedFd};
use rustix::fs::{
    AtFlags, Mode, OFlags, RenameFlags, ResolveFlags, Timespec, Timestamps, fchmod, fstat, fsync,
    futimens, open, openat, openat2, renameat_with, unlinkat,
};
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::os::unix::ffi::OsStrExt;
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
    if path.as_os_str().is_empty()
        || path.as_os_str().as_bytes().len() > 4_096
        || path.is_absolute()
    {
        return Err(FsError::UnsafePath);
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(FsError::UnsafePath);
        }
    }
    Ok(())
}

pub fn open_root(root: &Path) -> Result<OwnedFd, FsError> {
    open(
        root,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))
}

pub fn open_beneath(root: &OwnedFd, relative: &Path, flags: OFlags) -> Result<OwnedFd, FsError> {
    validate_relative(relative)?;
    openat2(
        root,
        relative,
        flags | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
        ResolveFlags::BENEATH
            | ResolveFlags::NO_SYMLINKS
            | ResolveFlags::NO_MAGICLINKS
            | ResolveFlags::NO_XDEV,
    )
    .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))
}

pub fn identity_fd<Fd: AsFd>(fd: Fd) -> Result<Identity, FsError> {
    let metadata = fstat(fd)
        .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
    if metadata.st_nlink != 1 || metadata.st_size < 0 {
        return Err(FsError::UnsafeSource);
    }
    let size = u64::try_from(metadata.st_size).map_err(|_| FsError::Oversized)?;
    if size > MAX_FILE_SIZE {
        return Err(FsError::Oversized);
    }
    Ok(Identity {
        device: metadata.st_dev,
        inode: metadata.st_ino,
        size,
        modified_ns: i128::from(metadata.st_mtime) * 1_000_000_000
            + i128::from(metadata.st_mtime_nsec),
    })
}

pub fn device_fd<Fd: AsFd>(fd: Fd) -> Result<u64, FsError> {
    Ok(fstat(fd)
        .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?
        .st_dev)
}

pub fn same_filesystem_move_at<SFd: AsFd, DFd: AsFd>(
    source_parent: SFd,
    source_name: &OsStr,
    destination_parent: DFd,
    destination_name: &OsStr,
    expected: Identity,
) -> Result<(), FsError> {
    let source_fd = openat2(
        &source_parent,
        source_name,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
    if identity_fd(&source_fd)? != expected {
        return Err(FsError::IdentityChanged);
    }
    match renameat_with(
        &source_parent,
        source_name,
        &destination_parent,
        destination_name,
        RenameFlags::NOREPLACE,
    ) {
        Ok(()) => {
            fsync(&source_parent)
                .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
            fsync(&destination_parent)
                .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
            let destination_fd = openat2(
                &destination_parent,
                destination_name,
                OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
                Mode::empty(),
                ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
            )
            .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
            if identity_fd(destination_fd)? != expected {
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

pub fn verified_stage_copy_at<SFd: AsFd, DFd: AsFd>(
    source: SFd,
    destination_parent: DFd,
    staging_name: &OsStr,
    expected: Identity,
) -> Result<String, FsError> {
    if identity_fd(&source)? != expected {
        return Err(FsError::IdentityChanged);
    }
    let source_stat = fstat(&source)
        .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
    let output = openat(
        &destination_parent,
        staging_name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::RUSR | Mode::WUSR,
    )
    .map_err(|error| {
        if error == rustix::io::Errno::EXIST {
            FsError::Collision
        } else {
            FsError::Io(io::Error::from_raw_os_error(error.raw_os_error()))
        }
    })?;
    let mut reader = BufReader::new(File::from(
        rustix::io::fcntl_dupfd_cloexec(source, 0)
            .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?,
    ));
    let mut writer = BufWriter::new(File::from(output));
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
    fchmod(
        writer.get_ref(),
        Mode::from_bits_truncate(source_stat.st_mode as _),
    )
    .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
    futimens(
        writer.get_ref(),
        &Timestamps {
            last_access: Timespec {
                tv_sec: source_stat.st_atime,
                tv_nsec: source_stat.st_atime_nsec as _,
            },
            last_modification: Timespec {
                tv_sec: source_stat.st_mtime,
                tv_nsec: source_stat.st_mtime_nsec as _,
            },
        },
    )
    .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
    writer.get_ref().sync_all()?;
    if copied != expected.size {
        return Err(FsError::IdentityChanged);
    }
    let digest = hex::encode(hasher.finalize());
    let staged = openat2(
        &destination_parent,
        staging_name,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
    if sha256_file(File::from(staged))? != digest {
        return Err(FsError::VerificationFailed);
    }
    Ok(digest)
}

pub fn publish_stage_at<DFd: AsFd>(
    destination_parent: DFd,
    staging_name: &OsStr,
    destination_name: &OsStr,
) -> Result<(), FsError> {
    match renameat_with(
        &destination_parent,
        staging_name,
        &destination_parent,
        destination_name,
        RenameFlags::NOREPLACE,
    ) {
        Ok(()) => fsync(destination_parent)
            .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error()))),
        Err(rustix::io::Errno::EXIST) => Err(FsError::Collision),
        Err(error) => Err(FsError::Io(io::Error::from_raw_os_error(
            error.raw_os_error(),
        ))),
    }
}

pub fn verify_at<DFd: AsFd>(
    parent: DFd,
    name: &OsStr,
    expected_sha256: &str,
) -> Result<(), FsError> {
    let fd = openat2(
        parent,
        name,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
    if sha256_file(File::from(fd))? == expected_sha256 {
        Ok(())
    } else {
        Err(FsError::VerificationFailed)
    }
}

pub fn retire_source_at<SFd: AsFd>(
    source_parent: SFd,
    source_name: &OsStr,
    expected: Identity,
    expected_sha256: &str,
) -> Result<(), FsError> {
    let fd = openat2(
        &source_parent,
        source_name,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
    if identity_fd(&fd)? != expected || sha256_file(File::from(fd))? != expected_sha256 {
        return Err(FsError::IdentityChanged);
    }
    unlinkat(&source_parent, source_name, AtFlags::empty())
        .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
    fsync(source_parent)
        .map_err(|error| FsError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
    Ok(())
}

fn sha256_file(file: File) -> Result<String, FsError> {
    let mut reader = BufReader::new(file);
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
    Ok(fs::symlink_metadata(path)?.file_type().is_symlink())
}

pub fn retire_source(
    source: &Path,
    expected: Identity,
    expected_sha256: &str,
) -> Result<(), FsError> {
    if Identity::read(source)? != expected || sha256(source)? != expected_sha256 {
        return Err(FsError::IdentityChanged);
    }
    fs::remove_file(source)?;
    File::open(source.parent().ok_or(FsError::UnsafePath)?)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

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
    fn descriptor_relative_copy_publish_and_retire_preserves_a_verified_copy() {
        let dir = tempfile::tempdir().unwrap();
        let source_dir = dir.path().join("source-dir");
        let destination_dir = dir.path().join("destination-dir");
        fs::create_dir(&source_dir).unwrap();
        fs::create_dir(&destination_dir).unwrap();
        let source = source_dir.join("payload");
        fs::write(&source, b"verified payload").unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o640)).unwrap();
        let expected = Identity::read(&source).unwrap();
        let source_parent = open_root(&source_dir).unwrap();
        let destination_parent = open_root(&destination_dir).unwrap();
        let source_fd = open_beneath(&source_parent, Path::new("payload"), OFlags::RDONLY).unwrap();
        let digest = verified_stage_copy_at(
            &source_fd,
            &destination_parent,
            OsStr::new(".stage"),
            expected,
        )
        .unwrap();
        publish_stage_at(
            &destination_parent,
            OsStr::new(".stage"),
            OsStr::new("payload"),
        )
        .unwrap();
        verify_at(&destination_parent, OsStr::new("payload"), &digest).unwrap();
        retire_source_at(&source_parent, OsStr::new("payload"), expected, &digest).unwrap();
        assert!(!source.exists());
        assert_eq!(
            fs::read(destination_dir.join("payload")).unwrap(),
            b"verified payload"
        );
        assert_eq!(
            fs::metadata(destination_dir.join("payload"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
    }

    #[test]
    fn traversal_and_symlinks_are_rejected() {
        assert!(validate_relative(Path::new("../escape")).is_err());
        let dir = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("/tmp", dir.path().join("link")).unwrap();
        assert!(path_is_symlink(&dir.path().join("link")).unwrap());
    }

    #[test]
    fn hardlinks_and_oversized_sparse_files_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("original");
        let linked = dir.path().join("linked");
        fs::write(&original, b"data").unwrap();
        fs::hard_link(&original, &linked).unwrap();
        assert!(matches!(
            Identity::read(&original),
            Err(FsError::UnsafeSource)
        ));

        let oversized = dir.path().join("oversized");
        File::create(&oversized)
            .unwrap()
            .set_len(MAX_FILE_SIZE + 1)
            .unwrap();
        assert!(matches!(
            Identity::read(&oversized),
            Err(FsError::Oversized)
        ));
    }

    #[test]
    fn case_folded_collision_is_detected() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("REPORT.PDF"), b"existing").unwrap();
        assert!(destination_exists_case_folded(&dir.path().join("report.pdf")).unwrap());
    }
}
