//! Permissioned Workestrate management, separate from backend data transport.
//!
//! Native exec/lifecycle and direct SSH custody share one permission and operation
//! boundary. Authenticated caller identity is supplied by an adapter, never
//! decoded from a request. Support does not imply runtime or service readiness;
//! SSH readiness requires a matching current broker observation.

pub mod authorization;
pub mod custody;
pub(crate) mod dispatcher;
#[cfg(unix)]
pub mod local;
pub(crate) mod native;
pub mod types;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod dispatcher_tests;
