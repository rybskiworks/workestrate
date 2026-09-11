//! Durable desired state only. Broker observations and live runtime authority
//! are deliberately reconstructed through their authenticated native adapters.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{LaunchState, SshController};
use crate::control_plane::types::{ControlError, DesiredSshState, LaunchRef, OpaqueId};
use crate::microsandbox::plan::{CredentialBinding, SshGrantPlan};

const VERSION: u16 = 2;
const MAX_STATE_BYTES: usize = 8 * 1024 * 1024;
const MAX_LAUNCHES: usize = 4096;
const MAX_GRANTS: usize = 256;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedState {
    version: u16,
    launches: Vec<SavedLaunch>,
    retired: BTreeSet<LaunchRef>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedLaunch {
    launch: LaunchRef,
    credentials: Vec<SshGrantPlan>,
    desired: DesiredSshState,
    effective_digest: Option<OpaqueId>,
}

/// The stable ownership file is kernel-locked and never unlinked. All data is
/// under an explicitly selected private directory, never derived from a caller.
#[derive(Debug)]
pub(super) struct DesiredStore {
    directory: PathBuf,
    directory_file: File,
    owner: File,
    owner_uid: u32,
    published: Cell<Option<FileStamp>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct FileStamp(u64, u64, u64, i64, i64);

fn stamp(metadata: &std::fs::Metadata) -> FileStamp {
    FileStamp(
        metadata.dev(),
        metadata.ino(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
    )
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid custody desired state")
}

fn private_regular(metadata: &std::fs::Metadata, uid: u32) -> io::Result<()> {
    if !metadata.is_file()
        || metadata.uid() != uid
        || metadata.mode() & 0o077 != 0
        || metadata.nlink() != 1
    {
        return Err(invalid());
    }
    Ok(())
}

fn same_file(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    (left.dev(), left.ino()) == (right.dev(), right.ino())
}

impl DesiredStore {
    fn acquire(directory: &Path, owner_uid: u32) -> io::Result<Self> {
        let metadata = std::fs::symlink_metadata(directory)?;
        if !metadata.is_dir() || metadata.uid() != owner_uid || metadata.mode() & 0o077 != 0 {
            return Err(invalid());
        }
        let directory_file = File::open(directory)?;
        if !same_file(&metadata, &directory_file.metadata()?) {
            return Err(invalid());
        }
        let path = directory.join("custody.lock");
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => private_regular(&metadata, owner_uid)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let owner = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&path)?;
        private_regular(&owner.metadata()?, owner_uid)?;
        if !same_file(&std::fs::symlink_metadata(&path)?, &owner.metadata()?) {
            return Err(invalid());
        }
        owner.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => io::Error::new(
                io::ErrorKind::WouldBlock,
                "custody desired state is already owned",
            ),
            std::fs::TryLockError::Error(error) => error,
        })?;
        let store = Self {
            directory: directory.to_owned(),
            directory_file,
            owner,
            owner_uid,
            published: Cell::new(None),
        };
        store.check_owner()?;
        Ok(store)
    }

    fn check_owner(&self) -> io::Result<()> {
        let directory = std::fs::symlink_metadata(&self.directory)?;
        if !directory.is_dir()
            || directory.uid() != self.owner_uid
            || directory.mode() & 0o077 != 0
            || !same_file(&directory, &self.directory_file.metadata()?)
        {
            return Err(invalid());
        }
        let lock = std::fs::symlink_metadata(self.directory.join("custody.lock"))?;
        private_regular(&lock, self.owner_uid)?;
        if !same_file(&lock, &self.owner.metadata()?) {
            return Err(invalid());
        }
        Ok(())
    }

    pub(super) fn check_current(&self) -> io::Result<()> {
        self.check_owner()?;
        match (
            self.published.get(),
            std::fs::symlink_metadata(self.directory.join("custody.json")),
        ) {
            (Some(expected), Ok(metadata)) => {
                private_regular(&metadata, self.owner_uid)?;
                if stamp(&metadata) != expected {
                    return Err(invalid());
                }
                Ok(())
            }
            (None, Err(error)) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            _ => Err(invalid()),
        }
    }

    fn read(&self) -> io::Result<SavedState> {
        self.check_owner()?;
        let path = self.directory.join("custody.json");
        let metadata = std::fs::symlink_metadata(&path)?;
        private_regular(&metadata, self.owner_uid)?;
        if metadata.len() > MAX_STATE_BYTES as u64 {
            return Err(invalid());
        }
        let file = File::open(&path)?;
        if !same_file(&metadata, &file.metadata()?) {
            return Err(invalid());
        }
        let mut bytes = Vec::new();
        file.take((MAX_STATE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_STATE_BYTES {
            return Err(invalid());
        }
        self.check_owner()?;
        if stamp(&std::fs::symlink_metadata(&path)?) != stamp(&metadata) {
            return Err(invalid());
        }
        let saved = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
        self.published.set(Some(stamp(&metadata)));
        Ok(saved)
    }

    pub(super) fn save(&self, controller: &SshController) -> io::Result<()> {
        self.check_current()?;
        if controller.launches.len() > MAX_LAUNCHES
            || controller.retired.len() > MAX_LAUNCHES
            || controller
                .launches
                .values()
                .any(|state| state.ceiling.len() > MAX_GRANTS)
        {
            return Err(invalid());
        }
        let state = SavedState {
            version: VERSION,
            launches: controller
                .launches
                .values()
                .map(|state| SavedLaunch {
                    launch: state.launch.clone(),
                    credentials: state.ceiling.values().cloned().collect(),
                    desired: state.desired.clone(),
                    effective_digest: state.effective_digest.clone(),
                })
                .collect(),
            retired: controller.retired.clone(),
        };
        let mut bytes = BoundedBytes(Vec::new());
        serde_json::to_writer(&mut bytes, &state).map_err(|_| invalid())?;
        let path = self.directory.join("custody.json");
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => private_regular(&metadata, self.owner_uid)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let mut temporary = tempfile::NamedTempFile::new_in(&self.directory)?;
        temporary.write_all(&bytes.0)?;
        temporary.as_file().sync_all()?;
        self.check_current()?;
        temporary.persist(&path).map_err(|error| error.error)?;
        self.published
            .set(Some(stamp(&std::fs::symlink_metadata(&path)?)));
        self.directory_file.sync_all()?;
        self.check_current()
    }
}

struct BoundedBytes(Vec<u8>);

impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_STATE_BYTES.saturating_sub(self.0.len()) {
            return Err(invalid());
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl SshController {
    /// Explicit initialization requires a new private directory. Existing or
    /// partially initialized state is never reset as a side effect of opening.
    pub(crate) fn initialize(directory: &Path, owner_uid: u32) -> Result<Self, ControlError> {
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(directory)
            .map_err(|_| ControlError::StateUnavailable)?;
        let store = DesiredStore::acquire(directory, owner_uid)
            .map_err(|_| ControlError::StateUnavailable)?;
        let mut controller = Self::new(fresh_incarnation()?);
        controller.store = Some(store);
        controller.persist_desired()?;
        // Publishing the snapshot syncs the new directory's contents; its
        // own entry in the existing parent must be durable as well.
        let parent = directory
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        File::open(parent)
            .and_then(|file| file.sync_all())
            .map_err(|_| ControlError::StateUnavailable)?;
        Ok(controller)
    }

    /// Recovery retains desired revisions/tombstones only. Every non-retired
    /// launch must be re-registered against current trusted compiled policy and
    /// confirmed by the native runtime before it can regain readiness.
    pub(crate) fn open(directory: &Path, owner_uid: u32) -> Result<Self, ControlError> {
        let store = DesiredStore::acquire(directory, owner_uid)
            .map_err(|_| ControlError::StateUnavailable)?;
        let state = store.read().map_err(|_| ControlError::StateUnavailable)?;
        if state.version != VERSION
            || state.launches.len() > MAX_LAUNCHES
            || state.retired.len() > MAX_LAUNCHES
        {
            return Err(ControlError::StateUnavailable);
        }
        let mut controller = Self::new(fresh_incarnation()?);
        controller.retired = state.retired;
        for saved in state.launches {
            if saved.credentials.len() > MAX_GRANTS
                || saved.desired.revision == 0
                || (saved.desired.destroyed && !saved.desired.credentials.is_empty())
                || controller.retired.contains(&saved.launch)
            {
                return Err(ControlError::StateUnavailable);
            }
            let mut ceiling = BTreeMap::new();
            for record in saved.credentials {
                if record.name.is_empty() || ceiling.insert(record.name.clone(), record).is_some() {
                    return Err(ControlError::StateUnavailable);
                }
            }
            if saved.desired.credentials.iter().any(|name| {
                !ceiling
                    .get(name)
                    .is_some_and(|record| record.binding == CredentialBinding::Broker)
            }) {
                return Err(ControlError::StateUnavailable);
            }
            let launch = saved.launch;
            if controller
                .launches
                .insert(
                    launch.instance.clone(),
                    LaunchState {
                        launch,
                        ceiling,
                        desired: saved.desired,
                        effective_digest: saved.effective_digest,
                        prepared_session: None,
                        observed: None,
                        runtime_live: false,
                        ceiling_verified: false,
                    },
                )
                .is_some()
            {
                return Err(ControlError::StateUnavailable);
            }
        }
        controller.store = Some(store);
        Ok(controller)
    }
}

fn fresh_incarnation() -> Result<OpaqueId, ControlError> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| ControlError::StateUnavailable)?;
    Ok(OpaqueId::from_bytes(bytes))
}

#[cfg(test)]
mod tests;
