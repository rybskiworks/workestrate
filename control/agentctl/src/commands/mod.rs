//! Command handlers (`workestrate <cmd>`).

pub mod config_cmd;
#[cfg(unix)]
pub mod control;
pub mod deps;
pub mod diagnostics;
pub mod doctor;
pub mod home;
pub mod init;
pub mod lifecycle;
pub mod migrate;
pub mod policy;
pub mod schemas;

pub mod secrets_target;
#[cfg(unix)]
pub mod signing_keys;
pub mod source;
pub mod versions;
