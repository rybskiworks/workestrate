mod litellm;
mod odysseus;
mod opencode;
mod pi;

use crate::microsandbox::workload::Workload;

pub use litellm::Litellm;
pub use odysseus::Odysseus;
pub use opencode::Opencode;
pub use pi::Pi;

/// Look up a workload by name.
pub fn get(name: &str) -> Option<Box<dyn Workload>> {
    match name {
        "litellm" => Some(Box::new(Litellm)),
        "pi" => Some(Box::new(Pi)),
        "odysseus" => Some(Box::new(Odysseus)),
        "opencode" => Some(Box::new(Opencode)),
        _ => None,
    }
}
