//! `read_file` and `write_file`: each module only sees its own data folder,
//! `<data root>/<project>/<module path>/`, optionally limited to some
//! sub-folders, within a storage quota.
//!
//! Paths are relative, `/`-separated names. Refused: absolute paths, `.` and
//! `..`, empty or over-long names, control characters, `\` and `:`, Windows
//! device names, the temporary-file suffix, and any symbolic link on the way.
//! Every folder is opened one level at a time with `openat` and `O_NOFOLLOW`
//! from the data root, and files are opened relative to that folder handle:
//! there is no gap between checking a path and using it, so swapping a folder
//! for a link in between cannot redirect the access.
//! Writes go to a temporary file, then are renamed through the call's
//! [`CallToken`], so nothing is written after a timeout.

use std::fs::{self, File};
use std::io::{ErrorKind, Read as _, Write as _};
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};

use rustix::fs::{AtFlags, Mode, OFlags};
use rustix::io::Errno;

use argos_module_policy::Capability;

use crate::{
    CallToken, HostServices, HOST_DENIED, HOST_FORBIDDEN_PATH, HOST_IO, HOST_NOT_FOUND, HOST_QUOTA,
    HOST_TOO_LARGE, MAX_REQUEST_BYTES,
};

/// Suffix of temporary files; module paths may not use it.
const TMP_SUFFIX: &str = ".argos-tmp";
/// Deepest relative path, in names.
const MAX_DEPTH: usize = 16;
/// Longest name, in bytes.
const MAX_NAME: usize = 255;

/// Why a file service could not be set up.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FileSetupError {
    /// The data root is missing or unreadable.
    #[error("data root unavailable: {0}")]
    DataRoot(String),
    /// The module folder would overlap a protected folder (keys, ledgers, anchors).
    #[error("module folder overlaps a protected folder: {0}")]
    Protected(PathBuf),
    /// Not a module path such as `hss/notes` (2 to 8 lowercase segments).
    #[error("invalid module path: {0}")]
    InvalidModule(String),
    /// A part of the module folder is a symbolic link.
    #[error("symbolic link in the module folder: {0}")]
    SymbolicLink(PathBuf),
}

/// What to do with a missing folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Missing {
    /// Reading: a missing folder means a missing file.
    Refuse,
    /// Writing: create it (mode 0700).
    Create,
}

/// The file service of one module.
#[derive(Debug, Clone)]
pub struct FileService {
    root: PathBuf,
    segments: Vec<String>,
    dir: PathBuf,
    subfolders: Option<Vec<String>>,
    quota: u64,
}

impl FileService {
    /// Sets up the data folder of `module` under `data_root`.
    ///
    /// `subfolders`, if given, are the only top-level folders the module may
    /// use. `protected` lists folders that must never be reachable (keys,
    /// ledgers, anchors): the module folder may neither be inside one nor
    /// contain one.
    ///
    /// # Errors
    /// [`FileSetupError`].
    pub fn new(
        data_root: &Path,
        module: &str,
        subfolders: Option<Vec<String>>,
        quota: u64,
        protected: &[PathBuf],
    ) -> Result<Self, FileSetupError> {
        let segments: Vec<&str> = module.split('/').collect();
        let valid = (2..=8).contains(&segments.len())
            && segments.iter().all(|s| {
                !s.is_empty()
                    && s.len() <= 64
                    && s.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            });
        if !valid {
            return Err(FileSetupError::InvalidModule(module.to_owned()));
        }
        let root = data_root
            .canonicalize()
            .map_err(|e| FileSetupError::DataRoot(e.to_string()))?;
        let mut dir = root.clone();
        for segment in &segments {
            dir.push(segment);
            if fs::symlink_metadata(&dir).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err(FileSetupError::SymbolicLink(dir));
            }
        }
        for folder in protected {
            let folder = folder.canonicalize().unwrap_or_else(|_| folder.clone());
            if dir.starts_with(&folder) || folder.starts_with(&dir) {
                return Err(FileSetupError::Protected(folder));
            }
        }
        Ok(Self {
            root,
            segments: segments.iter().map(|s| (*s).to_owned()).collect(),
            dir,
            subfolders,
            quota,
        })
    }

    /// The module's data folder.
    pub fn folder(&self) -> &Path {
        &self.dir
    }

    /// Checks a relative path; returns its folder names and its file name.
    fn names(&self, raw: &[u8]) -> Result<(Vec<String>, String), i32> {
        let text = std::str::from_utf8(raw).map_err(|_| HOST_FORBIDDEN_PATH)?;
        let mut names: Vec<String> = text.split('/').map(str::to_owned).collect();
        if names.len() > MAX_DEPTH || !names.iter().all(|n| allowed_name(n)) {
            return Err(HOST_FORBIDDEN_PATH);
        }
        if let Some(allowed) = &self.subfolders {
            if names.len() < 2 || !allowed.iter().any(|a| *a == names[0]) {
                return Err(HOST_FORBIDDEN_PATH);
            }
        }
        let file = names.pop().ok_or(HOST_FORBIDDEN_PATH)?;
        Ok((names, file))
    }

    /// Opens the module folder, then `folders`, one level at a time without
    /// following any link; creates missing folders if `create`.
    fn open_folder(&self, folders: &[String], missing: Missing) -> Result<OwnedFd, i32> {
        let dir_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let mut fd = rustix::fs::open(&self.root, dir_flags, Mode::empty()).map_err(errno)?;
        for name in self.segments.iter().chain(folders) {
            let next = match rustix::fs::openat(&fd, name.as_str(), dir_flags, Mode::empty()) {
                Err(Errno::NOENT) if missing == Missing::Create => {
                    rustix::fs::mkdirat(&fd, name.as_str(), Mode::from_raw_mode(0o700))
                        .map_err(errno)?;
                    rustix::fs::openat(&fd, name.as_str(), dir_flags, Mode::empty())
                }
                other => other,
            };
            fd = next.map_err(errno)?;
        }
        Ok(fd)
    }

    /// Reads a file of at most [`MAX_REQUEST_BYTES`].
    fn read(&self, request: &[u8]) -> Result<Vec<u8>, i32> {
        let (folders, name) = self.names(request)?;
        let dir = self.open_folder(&folders, Missing::Refuse)?;
        let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let file = File::from(
            rustix::fs::openat(&dir, name.as_str(), flags, Mode::empty()).map_err(errno)?,
        );
        if !file.metadata().map_err(|e| status(&e))?.is_file() {
            return Err(HOST_FORBIDDEN_PATH);
        }
        let mut bytes = Vec::new();
        let limit = u64::try_from(MAX_REQUEST_BYTES).unwrap_or(u64::MAX);
        file.take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| status(&e))?;
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(HOST_TOO_LARGE);
        }
        Ok(bytes)
    }

    /// Writes a file. Request: path length (2 bytes, big-endian), path, content.
    fn write(&self, request: &[u8], token: &CallToken) -> Result<Vec<u8>, i32> {
        let (len, rest) = request.split_at_checked(2).ok_or(HOST_FORBIDDEN_PATH)?;
        let len = usize::from(u16::from_be_bytes([len[0], len[1]]));
        let (raw_path, content) = rest.split_at_checked(len).ok_or(HOST_FORBIDDEN_PATH)?;
        let (folders, name) = self.names(raw_path)?;
        let dir = self.open_folder(&folders, Missing::Create)?;
        // Quota: everything stored, minus the file being replaced, plus the new content.
        let existing = rustix::fs::statat(&dir, name.as_str(), AtFlags::SYMLINK_NOFOLLOW).ok();
        if existing.is_some_and(|st| !rustix::fs::FileType::from_raw_mode(st.st_mode).is_file()) {
            // Never replace a link or a folder, even inside the module folder.
            return Err(HOST_FORBIDDEN_PATH);
        }
        let old = existing.map_or(0, |st| u64::try_from(st.st_size).unwrap_or(0));
        let new = u64::try_from(content.len()).map_err(|_| HOST_QUOTA)?;
        let used = usage(&self.dir).map_err(|_| HOST_IO)?;
        if used.saturating_sub(old).saturating_add(new) > self.quota {
            return Err(HOST_QUOTA);
        }
        let tmp = format!("{name}{TMP_SUFFIX}");
        let flags =
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let mut file = File::from(
            rustix::fs::openat(&dir, tmp.as_str(), flags, Mode::from_raw_mode(0o600))
                .map_err(errno)?,
        );
        let written = file.write_all(content).and_then(|()| file.sync_all());
        drop(file);
        let remove = || rustix::fs::unlinkat(&dir, tmp.as_str(), AtFlags::empty());
        if written.is_err() {
            let _ = remove();
            return Err(HOST_IO);
        }
        if let Some(Ok(())) =
            token.commit(|| rustix::fs::renameat(&dir, tmp.as_str(), &dir, name.as_str()))
        {
            Ok(Vec::new())
        } else {
            let _ = remove();
            Err(HOST_IO)
        }
    }
}

impl HostServices for FileService {
    fn call(
        &self,
        capability: Capability,
        request: &[u8],
        token: &CallToken,
    ) -> Result<Vec<u8>, i32> {
        match capability {
            Capability::ReadFiles => self.read(request),
            Capability::WriteFiles => self.write(request, token),
            Capability::Network | Capability::Ai | Capability::Model => Err(HOST_DENIED),
        }
    }
}

/// A name a module may use in a path.
fn allowed_name(name: &str) -> bool {
    const DEVICES: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let device = DEVICES.contains(&stem.as_str())
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit());
    !name.is_empty()
        && name != "."
        && name != ".."
        && name.len() <= MAX_NAME
        && !name.ends_with(TMP_SUFFIX)
        && !device
        && !name
            .chars()
            .any(|c| c.is_control() || c == '\\' || c == ':')
}

/// Maps a file system error to a host status.
fn status(error: &std::io::Error) -> i32 {
    match error.kind() {
        ErrorKind::NotFound => HOST_NOT_FOUND,
        _ => HOST_IO,
    }
}

/// Maps an `openat` error: missing is not found, a link or a non-folder on
/// the way is a refused path.
fn errno(error: Errno) -> i32 {
    match error {
        Errno::NOENT => HOST_NOT_FOUND,
        Errno::LOOP | Errno::NOTDIR | Errno::EXIST => HOST_FORBIDDEN_PATH,
        _ => HOST_IO,
    }
}

/// Bytes stored in `dir` (0 if it does not exist yet); links are not followed.
fn usage(dir: &Path) -> std::io::Result<u64> {
    let mut total = 0_u64;
    let mut pending = vec![dir.to_path_buf()];
    while let Some(folder) = pending.pop() {
        let entries = match fs::read_dir(&folder) {
            Ok(entries) => entries,
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        for entry in entries {
            let entry = entry?;
            let meta = fs::symlink_metadata(entry.path())?;
            if meta.is_dir() {
                pending.push(entry.path());
            } else {
                total = total.saturating_add(meta.len());
            }
        }
    }
    Ok(total)
}
