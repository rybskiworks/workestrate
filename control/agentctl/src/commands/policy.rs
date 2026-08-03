//! Spec 22 §13 mount-policy diagnostics. Configuration is loaded through the
//! same `ConfigWorkload::new_with_use_overrides` compiler path as runtime.

use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::cli_actions::{MountsDiagnosticsAction, PolicyAction};
use crate::microsandbox::mounts::{resolve_mount_host, MountRoots};
use crate::microsandbox::workload::{ConfigWorkload, Workload};
use crate::mount_policy::{Decision, LexicalPath, MountPolicyProgram, WriteDecision};

#[derive(Serialize)]
struct OriginJson {
    layer: String,
    file: String,
    scope_kind: crate::mount_policy::ScopeKind,
}
#[derive(Serialize)]
struct MatchJson {
    rule_index: usize,
    pattern: String,
    effect: String,
    terminal: bool,
    frozen_out: bool,
    origin: OriginJson,
}
#[derive(Serialize)]
struct ParentJson {
    path: String,
    may_unmask_descendant: bool,
}
#[derive(Serialize)]
struct MountJson {
    guest: String,
    host: String,
}
#[derive(Serialize)]
struct ExplainJson {
    workload: String,
    mount: MountJson,
    path: String,
    decision: String,
    write: String,
    fail_closed_non_utf8: bool,
    frozen_by: Option<OriginJson>,
    matches: Vec<MatchJson>,
    parents: Vec<ParentJson>,
}

#[derive(Serialize)]
struct TreeJson {
    path: String,
    decision: String,
    write: String,
    protected: bool,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pruned: Option<bool>,
}
#[derive(Serialize)]
struct WarningJson {
    masked: String,
    visible: String,
    dev: u64,
    ino: u64,
}
#[derive(Serialize)]
struct PreviewJson {
    workload: String,
    mount: PreviewMountJson,
    tree: Vec<TreeJson>,
    hardlink_warnings: Vec<WarningJson>,
}
#[derive(Serialize)]
struct PreviewMountJson {
    guest: String,
    host: String,
    root: String,
}

fn origin_json(origin: &crate::mount_policy::RuleOrigin) -> OriginJson {
    OriginJson {
        layer: origin.layer.clone(),
        file: origin.file.display().to_string(),
        scope_kind: origin.scope_kind,
    }
}
fn decision_label(decision: Decision, protected: bool) -> &'static str {
    if protected {
        "protected"
    } else {
        match decision {
            Decision::Visible => "visible",
            Decision::Masked => "masked",
            Decision::TraversalOnly => "traversal-only",
        }
    }
}
fn decision_text(decision: Decision, protected: bool) -> &'static str {
    if protected {
        "Protected"
    } else {
        match decision {
            Decision::Visible => "Visible",
            Decision::Masked => "Masked",
            Decision::TraversalOnly => "TraversalOnly",
        }
    }
}
fn parent_paths(path: &LexicalPath) -> Vec<LexicalPath> {
    let components = path.components();
    (0..components.len())
        .filter_map(|n| LexicalPath::new(&components[..n].join("/")).ok())
        .collect()
}
fn policy_for<'a>(wl: &'a ConfigWorkload) -> Result<&'a MountPolicyProgram> {
    wl.mount_policy().ok_or_else(|| anyhow::anyhow!("workload '{}' has no compiled mount policy; declare [policy.mounts] to use this command", wl.name()))
}

pub async fn cmd_policy(action: PolicyAction, json: bool) -> Result<()> {
    match action {
        PolicyAction::Mounts {
            action:
                MountsDiagnosticsAction::Explain {
                    workload,
                    mount,
                    path,
                },
        } => cmd_explain(&workload, &mount, &path, json),
        PolicyAction::Mounts {
            action:
                MountsDiagnosticsAction::Preview {
                    workload,
                    mount,
                    root,
                },
        } => cmd_preview(&workload, &mount, root.as_deref(), json),
    }
}

fn cmd_explain(workload: &str, mount: &str, path: &str, json: bool) -> Result<()> {
    let wl = ConfigWorkload::new_with_use_overrides(workload, &[])?;
    let program = policy_for(&wl)?;
    let plan = wl.plan();
    let m = plan
        .mounts
        .iter()
        .find(|m| m.guest == mount)
        .ok_or_else(|| {
            anyhow::anyhow!("workload '{workload}' has no mount with guest path '{mount}'")
        })?;
    let lexical = LexicalPath::new(path)?;
    let read = program.decide(&lexical);
    let write = program.decide_write(&lexical);
    let parents = parent_paths(&lexical);
    if json {
        let out = ExplainJson {
            workload: workload.to_string(),
            mount: MountJson {
                guest: m.guest.clone(),
                host: m.host.clone(),
            },
            path: path.to_string(),
            decision: decision_label(read.decision, program.is_protected(&lexical))
                .replace('-', "_"),
            write: if write.decision == WriteDecision::Allow {
                "allow"
            } else {
                "deny"
            }
            .to_string(),
            fail_closed_non_utf8: read.fail_closed_non_utf8,
            frozen_by: read.frozen_by.as_ref().map(origin_json),
            matches: read
                .matches
                .iter()
                .map(|x| MatchJson {
                    rule_index: x.rule_index,
                    pattern: x.pattern.clone(),
                    effect: x.effect.to_string(),
                    terminal: x.terminal,
                    frozen_out: x.frozen_out,
                    origin: origin_json(&x.origin),
                })
                .collect(),
            parents: parents
                .iter()
                .map(|p| ParentJson {
                    path: p.as_str().unwrap_or("").to_string(),
                    may_unmask_descendant: program.may_unmask_descendant(p),
                })
                .collect(),
        };
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!(
            "workload: {workload}\nmount: {} -> {}\npath: {path}",
            m.guest, m.host
        );
        println!(
            "decision: {}",
            decision_text(read.decision, program.is_protected(&lexical))
        );
        println!(
            "write: {}",
            if write.decision == WriteDecision::Allow {
                "Allow"
            } else {
                "Deny"
            }
        );
        if read.fail_closed_non_utf8 {
            println!("fail_closed_non_utf8: true");
        }
        println!(
            "frozen_by: {}",
            read.frozen_by
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "(none)".to_string())
        );
        println!("matches:");
        for x in &read.matches {
            println!(
                "  rule #{}: {} terminal={} frozen_out={} origin={} pattern={}",
                x.rule_index, x.effect, x.terminal, x.frozen_out, x.origin, x.pattern
            );
        }
        println!("parents:");
        for p in parents {
            println!(
                "  {}: may_unmask_descendant: {}",
                p.as_str().unwrap_or(""),
                program.may_unmask_descendant(&p)
            );
        }
    }
    Ok(())
}

fn resolve_root(
    wl: &ConfigWorkload,
    mount: &crate::microsandbox::plan::MountPlan,
    override_root: Option<&str>,
) -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let content = override_root
        .map(PathBuf::from)
        .or_else(|| wl.mount_content_root())
        .or_else(crate::config::project_root_optional)
        .unwrap_or(cwd);
    let build = wl.build_path();
    resolve_mount_host(
        &MountRoots {
            content_root: &content,
            project_root: None,
            build_path: &build,
        },
        &mount.host,
    )
}

fn cmd_preview(workload: &str, mount: &str, root_override: Option<&str>, json: bool) -> Result<()> {
    let wl = ConfigWorkload::new_with_use_overrides(workload, &[])?;
    let program = policy_for(&wl)?;
    let plan = wl.plan();
    let m = plan
        .mounts
        .iter()
        .find(|m| m.guest == mount)
        .ok_or_else(|| {
            anyhow::anyhow!("workload '{workload}' has no mount with guest path '{mount}'")
        })?;
    let root = resolve_root(&wl, m, root_override)?;
    if !root.exists() {
        anyhow::bail!("mount host root '{}' does not exist", root.display());
    }
    let mut rows = Vec::new();
    let mut visible_inodes: HashMap<(u64, u64), String> = HashMap::new();
    walk_tree(
        &root,
        Path::new(""),
        program,
        &mut rows,
        &mut visible_inodes,
    )?;
    let mut warnings = Vec::new();
    for (path, (dev, ino)) in masked_inodes(&root, &rows)? {
        if let Some(visible) = visible_inodes.get(&(dev, ino)) {
            warnings.push(WarningJson {
                masked: path,
                visible: visible.clone(),
                dev,
                ino,
            });
        }
    }
    if json {
        let tree = rows.into_iter().map(|(_, row)| row).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&PreviewJson {
                workload: workload.to_string(),
                mount: PreviewMountJson {
                    guest: m.guest.clone(),
                    host: m.host.clone(),
                    root: root.display().to_string()
                },
                tree,
                hardlink_warnings: warnings
            })?
        );
    } else {
        println!("{}/", root.display());
        for (path, row) in rows {
            println!(
                "  {} [{}]{}{}",
                path,
                row.decision,
                if row.write == "deny" {
                    " [write-deny]"
                } else {
                    ""
                },
                if row.pruned.unwrap_or(false) {
                    " (pruned)"
                } else {
                    ""
                }
            );
        }
        for w in warnings {
            println!(
                "hardlink alias: '{}' shares inode with visible '{}'",
                w.masked, w.visible
            );
        }
    }
    Ok(())
}

fn walk_tree(
    root: &Path,
    rel: &Path,
    program: &MountPolicyProgram,
    rows: &mut Vec<(String, TreeJson)>,
    visible: &mut HashMap<(u64, u64), String>,
) -> Result<()> {
    let dir = root.join(rel);
    let mut entries: Vec<_> =
        std::fs::read_dir(&dir)?.collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name();
        let child = rel.join(&name);
        let text = child
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        let lexical = LexicalPath::new(&text)?;
        let meta = std::fs::symlink_metadata(entry.path())?;
        let kind = if meta.file_type().is_symlink() {
            "symlink"
        } else if meta.is_dir() {
            "dir"
        } else {
            "file"
        };
        let read = program.decide(&lexical);
        let protected = program.is_protected(&lexical);
        let decision = decision_label(read.decision, protected).replace('-', "_");
        let write = if program.decide_write(&lexical).decision == WriteDecision::Allow {
            "allow"
        } else {
            "deny"
        };
        let prune = meta.is_dir()
            && (protected
                || (read.decision == Decision::Masked && !program.may_unmask_descendant(&lexical)));
        let row = TreeJson {
            path: text.clone(),
            decision: decision.to_string(),
            write: write.to_string(),
            protected,
            kind: kind.to_string(),
            pruned: if meta.is_dir() { Some(prune) } else { None },
        };
        if decision == "visible" {
            #[cfg(unix)]
            if let Some(inode) = inode(&meta) {
                visible.entry(inode).or_insert_with(|| text.clone());
            }
        }
        rows.push((text, row));
        if meta.is_dir() && !prune {
            walk_tree(root, &child, program, rows, visible)?;
        }
    }
    Ok(())
}

fn masked_inodes(root: &Path, rows: &[(String, TreeJson)]) -> Result<Vec<(String, (u64, u64))>> {
    let mut out = Vec::new();
    for (path, row) in rows.iter().filter(|(_, r)| r.decision == "masked") {
        let meta = std::fs::symlink_metadata(root.join(path))?;
        #[cfg(unix)]
        if let Some(i) = inode(&meta) {
            out.push((path.clone(), i));
        }
    }
    Ok(out)
}
#[cfg(unix)]
fn inode(meta: &std::fs::Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    Some((meta.dev(), meta.ino()))
}
