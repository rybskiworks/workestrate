//! CLI error classification and emission.
//!
//! Maps an `anyhow::Error` to a classified kind + exit code and emits either
//! a human-readable stderr line or a machine-readable JSON error envelope
//! (under `--json`). Message-substring matching is the current heuristic.
//!
//! NOTE (WP4-A): these are verbatim copies of the private items still live
//! in `main.rs`. Commit C cuts `main.rs` over to these and deletes its own.

pub(crate) fn emit_error(e: &anyhow::Error, json_mode: bool) {
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

pub(crate) struct Classified {
    kind: &'static str,
    message: String,
    exit_code: i32,
}

pub(crate) fn classify_error(err: &anyhow::Error) -> Classified {
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

pub(crate) fn classify_exit_code(err: &anyhow::Error) -> i32 {
    classify_error(err).exit_code
}
