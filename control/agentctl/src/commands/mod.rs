//! Command handlers (`workestrate <cmd>`).
//!
//! NOTE (WP4-B): every item in these leaf modules is a verbatim copy of the
//! private item still live in `main.rs` (only `pub(crate)` visibility was
//! added, plus header `use` lines for cross-module wiring). Commit C cuts
//! `main.rs` over to these via full `crate::commands::<leaf>::<item>` paths
//! and deletes its own originals.

#[allow(dead_code)]
mod config_cmd;
#[allow(dead_code)]
mod diagnostics;
#[allow(dead_code)]
mod doctor;
#[allow(dead_code)]
mod init;
#[allow(dead_code)]
mod lifecycle;
#[allow(dead_code)]
mod migrate;
#[allow(dead_code)]
mod secrets_target;
#[allow(dead_code)]
mod source;
