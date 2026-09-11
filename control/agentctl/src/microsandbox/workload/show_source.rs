use super::Workload;
use super::config::ConfigWorkload;
use super::secrets::secret_line_source;

impl ConfigWorkload {
    pub(super) fn show_source_render(&self) -> String {
        let plan = self.plan();
        let mut out = String::new();
        let default_source = "core";
        let secret_prov = crate::merge::get_secret_provenance();

        let source_of = |key: &str| -> &str {
            self.provenance
                .as_ref()
                .and_then(|p| p.get(key))
                .map(|s| s.as_str())
                .unwrap_or(default_source)
        };

        let write_line = |out: &mut String, prefix: &str, content: &str, source: &str| {
            let full = format!("{}{}", prefix, content);
            let pad = if full.len() < 48 {
                " ".repeat(48 - full.len())
            } else {
                "  ".to_string()
            };
            out.push_str(&full);
            out.push_str(&pad);
            out.push('[');
            out.push_str(source);
            out.push(']');
            out.push('\n');
        };

        write_line(
            &mut out,
            "",
            &format!("name: {}", plan.name),
            default_source,
        );
        if let Some(img) = &plan.image {
            write_line(
                &mut out,
                "",
                &format!("image: {}", img),
                source_of(&format!("workloads.{}.image", self.name)),
            );
        }
        if let Some(wd) = &plan.workdir {
            write_line(
                &mut out,
                "",
                &format!("workdir: {}", wd),
                source_of(&format!("workloads.{}.workdir", self.name)),
            );
        }
        if let Some(init) = &plan.init {
            write_line(
                &mut out,
                "",
                &format!("init: {init}"),
                source_of(&format!("workloads.{}.init", self.name)),
            );
        }
        if !plan.command.is_empty() {
            write_line(
                &mut out,
                "",
                &format!("command: {}", plan.command.join(" ")),
                source_of(&format!("workloads.{}.command", self.name)),
            );
        }
        if let Some(cpus) = plan.cpus {
            write_line(
                &mut out,
                "",
                &format!("cpus: {}", cpus),
                source_of(&format!("workloads.{}.cpus", self.name)),
            );
        }
        if let Some(mem) = plan.memory_mib {
            write_line(
                &mut out,
                "",
                &format!("memory: {} MiB", mem),
                source_of(&format!("workloads.{}.memory_mib", self.name)),
            );
        }
        if let Some(size) = plan.root_disk_mib {
            write_line(
                &mut out,
                "",
                &format!("root disk: {size} MiB (new managed OCI disk)"),
                source_of(&format!("workloads.{}.root_disk_mib", self.name)),
            );
        }
        if self.workload.kind == "service" {
            let seconds = self
                .workload
                .readiness_timeout_secs
                .map(u64::from)
                .unwrap_or(crate::microsandbox::runtime::DEFAULT_WAIT.as_secs());
            write_line(
                &mut out,
                "",
                &format!("readiness timeout: {seconds}s (dependency/batch TCP)"),
                source_of(&format!("workloads.{}.readiness_timeout_secs", self.name)),
            );
        }
        let env_source = source_of(&format!("workloads.{}.env", self.name));
        for e in &plan.env {
            let source = if let Some(dep) = &e.injected_by {
                // ADR 0026(d): an injected var attributes to the layer that
                // declared the depends_on entry, not the env layer.
                source_of(&format!("workloads.{}.depends_on.{}", self.name, dep))
            } else if e.is_secret {
                // P1 Wave 1: resolve via the rendered name — secret-load
                // provenance first, then the binding-site merge provenance
                // `workloads.{wl}.env.{NAME}`.
                secret_line_source(
                    &e.name,
                    &self.name,
                    self.provenance.as_ref(),
                    secret_prov.as_ref(),
                    default_source,
                )
            } else {
                env_source
            };
            write_line(&mut out, "", &format!("env: {}", e), source);
        }
        for se in &plan.secret_env {
            let source = secret_line_source(
                &se.name,
                &self.name,
                self.provenance.as_ref(),
                secret_prov.as_ref(),
                default_source,
            );
            write_line(
                &mut out,
                "",
                &format!(
                    "secret_env: {} (redacted, allowed: {})",
                    se.name,
                    se.allowed_hosts.join(", ")
                ),
                source,
            );
        }
        let ports_source = source_of(&format!("workloads.{}.ports", self.name));
        for p in &plan.ports {
            let name_prefix = p
                .name
                .as_deref()
                .map(|n| format!("{n}:"))
                .unwrap_or_default();
            let auto_suffix = if p.host == 0 { " (auto)" } else { "" };
            write_line(
                &mut out,
                "",
                &format!("port: {}{}:{}{}", name_prefix, p.host, p.guest, auto_suffix),
                ports_source,
            );
        }
        let mounts_source = source_of(&format!("workloads.{}.mounts", self.name));
        for m in &plan.mounts {
            let ro = if m.is_read_only() { " (ro)" } else { "" };
            write_line(
                &mut out,
                "",
                &format!("mount: {}:{}{}", m.host, m.guest, ro),
                mounts_source,
            );
        }
        let egress_source = source_of(&format!("workloads.{}.network.defaults.egress", self.name));
        let ingress_source =
            source_of(&format!("workloads.{}.network.defaults.ingress", self.name));
        // One rendered line carries two independently-sourced tokens; when
        // the layers differ, attribute each direction explicitly.
        let network_source = if egress_source == ingress_source {
            egress_source.to_string()
        } else {
            format!("{egress_source} / ingress: {ingress_source}")
        };
        write_line(
            &mut out,
            "",
            &format!(
                "network: egress_default={} ingress_default={}",
                if plan.network.egress_default_deny {
                    "deny"
                } else {
                    "allow"
                },
                if plan.network.ingress_default_deny {
                    "deny"
                } else {
                    "allow"
                }
            ),
            &network_source,
        );
        let ingress_source = source_of(&format!("workloads.{}.network.ingress", self.name));
        for rule in &plan.network.ingress_rules {
            write_line(
                &mut out,
                "  ",
                &format!("ingress: {}:{} {}", rule.protocol, rule.port, rule.scope),
                ingress_source,
            );
        }
        for rule in &plan.network.egress_rules {
            // ADR 0026(d): a rule derived from a depends_on resolution
            // renders with its marker (matching the plan Display) and
            // attributes to the layer that declared the dependency.
            let (content, source) = match &rule.derived_from {
                Some(dep) => (
                    format!(
                        "egress: {}:{} -> {} (derived: depends_on '{}')",
                        rule.protocol, rule.port, rule.target, dep
                    ),
                    source_of(&format!("workloads.{}.depends_on.{}", self.name, dep)),
                ),
                None => (
                    format!("egress: {}:{} -> {}", rule.protocol, rule.port, rule.target),
                    "core",
                ),
            };
            write_line(&mut out, "  ", &content, source);
        }
        for rule in &plan.network.deny_rules {
            write_line(
                &mut out,
                "  ",
                &format!("egress: deny domain suffix {}", rule.domain_suffix),
                source_of(&format!(
                    "workloads.{}.network.deny.{}",
                    self.name, rule.domain_suffix
                )),
            );
        }
        // ADR 0030 Phase 1: the instance policy block renders with the whole-
        // block provenance (merge is whole-block-last-layer-wins).
        if let Some(policy) = &plan.instance_policy {
            let instance_source = source_of(&format!("workloads.{}.instance", self.name));
            write_line(
                &mut out,
                "",
                &format!("instance: strategy={}", policy.strategy),
                instance_source,
            );
            if let Some(chain) = &policy.on_conflict {
                let steps: Vec<String> = chain.0.iter().map(ToString::to_string).collect();
                write_line(
                    &mut out,
                    "",
                    &format!("instance: on_conflict=[{}]", steps.join(", ")),
                    instance_source,
                );
            }
            if let Some(port) = &policy.port {
                write_line(
                    &mut out,
                    "",
                    &format!(
                        "instance: port={}",
                        crate::microsandbox::plan::render_instance_port(port)
                    ),
                    instance_source,
                );
            }
            if let Some(label) = &policy.label {
                write_line(
                    &mut out,
                    "",
                    &format!("instance: label={label}"),
                    instance_source,
                );
            }
        }

        out
    }
}
