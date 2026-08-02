//! `workestrate` library crate.
//!
//! This library target exists so that `cargo test --doc` can compile and run
//! doctests (docs-as-code, WP8b) and so the `workestrate` binary at
//! `src/main.rs` shares one module tree with the test harnesses. The binary
//! consumes these modules as `workestrate::<module>`; unit tests inside the
//! modules run under this crate's test harness.

pub mod cli_actions;
pub mod cli_error;
pub mod commands;
pub mod config;
pub mod git;
pub mod images;
pub mod json_out;
pub mod merge;
pub mod microsandbox;
pub mod mount_policy;
pub mod policy;
pub mod recipes;
pub mod scaffold;
