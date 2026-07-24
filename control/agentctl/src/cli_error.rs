//! CLI error classification and emission.
//!
//! Maps an `anyhow::Error` to a classified kind + exit code and emits either
//! a human-readable stderr line or a machine-readable JSON error envelope
//! (under `--json`). Message-substring matching is the current heuristic.

pub fn emit_error(e: &anyhow::Error, json_mode: bool) {
    let classified = classify_error(e);
    if json_mode {
        let body = serde_json::json!({
            "error": {
                "kind": classified.kind,
                "message": classified.message,
            }
        });
        eprintln!(
            "{}",
            serde_json::to_string(&body).unwrap_or_else(|_| {
                r#"{"error":{"kind":"serialize_failed","message":"see logs"}}"#.to_string()
            })
        );
    } else {
        eprintln!("error: {}", classified.message);
    }
}

pub struct Classified {
    kind: &'static str,
    message: String,
    exit_code: i32,
}

pub fn classify_error(err: &anyhow::Error) -> Classified {
    let msg = err.to_string();
    if msg.contains("is already running") {
        Classified {
            kind: "refuse_occupied",
            message: msg,
            exit_code: 3,
        }
    } else if msg.contains("port collision") {
        Classified {
            kind: "port_collision",
            message: msg,
            exit_code: 4,
        }
    } else {
        Classified {
            kind: "error",
            message: msg,
            exit_code: 1,
        }
    }
}

pub fn classify_exit_code(err: &anyhow::Error) -> i32 {
    classify_error(err).exit_code
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;

    #[test]
    fn classify_error_refuse_occupied() {
        let e =
            anyhow::anyhow!("instance 'personal-litellm' is already running. Use --replace ...");
        let c = classify_error(&e);
        assert_eq!(c.kind, "refuse_occupied");
        assert_eq!(c.exit_code, 3);
    }

    #[test]
    fn classify_error_port_collision() {
        let e = anyhow::anyhow!("port collision: port 4000 is already in use by ...");
        let c = classify_error(&e);
        assert_eq!(c.kind, "port_collision");
        assert_eq!(c.exit_code, 4);
    }

    #[test]
    fn classify_error_generic() {
        let e = anyhow::anyhow!("something went wrong");
        let c = classify_error(&e);
        assert_eq!(c.kind, "error");
        assert_eq!(c.exit_code, 1);
    }
}
