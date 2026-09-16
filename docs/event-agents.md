# Event-driven agent workloads (fleet pattern)

How an event (a labelled pull request, a tag push, an issue assignment) turns
into a bounded agent workload, and which part of that lives in this tool.

## Placement: this is fleet composition

The role capsules, prompts, model configuration and the event runner are
**fleet content**. They belong in a fleet repository (for this project,
`rybskiworks/workestrate-fleet`), next to the other capsules it composes.
This repository's `config.reference/` stays synthetic `example-*` fixtures, so
the golden-plan guards keep testing the tool, not one deployment's roles.

The tool owns the contract below. A fleet owns the roles, the images, the
credentials and the events it reacts to.

## Tool contract

What the CLI already provides for an event-driven role:

- **Instance addressing.** `workload up <name> --instance <id>` targets
  `<slot>@<id>` and refuses when that exact instance is already running;
  `--new` allocates the lowest free integer id; `--replace` tears down an
  existing instance at the slot first. `workload down <name> --instance <id>`
  and `--all-instances` tear down. Instance ids are how branch or pull-request
  work is kept apart.
- **Detachment.** `workload up` is detached by default; `--foreground` blocks.
  A host-side service can therefore launch a role and exit.
- **Instance strategy.** The `[instance] strategy` vocabulary is
  `singleton | parallel | replace | reuse | per-dir`. `per-dir` derives the id
  from the canonical invocation directory. `parallel`, `replace` and `reuse`
  are parsed and validated but do **not** yet select behaviour (ADR 0030
  Phase 2); multi-instance work today comes from `--instance` / `--new`, not
  from declaring `parallel`.
- **State.** `[[mounts]]` with `mode = "rw" | "ro"`; `[[seed_files]]` with
  `only_if_missing` and `template`; both paths are relative to the config-repo
  root.
- **Network.** `[network.defaults] egress = "deny"` plus explicit
  `policy.egress.allow.host` / `.domain` grants. Deny-by-default is the only
  defensible default for a workload reacting to untrusted input.
- **Dependencies.** `[depends_on.<service>]` with `required` and the injected
  address (`env`), so a role reaches the model gateway rather than the public
  internet.
- **Secrets.** A bare `true` reads the host environment variable of the same
  name; `{ secret = "NAME", bound = "host" }` substitutes at the egress proxy
  and `bound = "guest"` injects into the guest. See [secrets](secrets.md).
- **Host control endpoint.** `workestrate control serve --state-dir <dir>
  --instance <name>...` serves one private Unix endpoint for retained exec and
  SSH custody on selected, already-running instances (16 connections, 32 queued
  requests, 32 retained operations). It is not an admission queue: it does not
  start workloads, hold pending launches or place work across hosts. See
  [runtime provisioning](runtime-provisioning.md).

## Host-side pattern

```
event  ->  authorize  ->  derive instance id  ->  workestrate workload up <role> --instance <id>
                                                        |
                                                   agent works in the microVM
                                                        |
                                            report back (check / comment / logs)
```

- **Capture.** The runner runs on the KVM host that owns the workloads. It
  polls the GitHub API or reads a captured event file, keeps a cursor, and
  never accepts an inbound connection.
- **Authorize before launch.** Allow-listed actor, allow-listed verb or label,
  fork pull requests untrusted by default, and an instance id derived from the
  payload (pull-request number, issue number, tag) rather than from free text.
- **Idempotency.** A ledger keyed by event id, so a restart or a repeated poll
  cannot launch the same instance twice.
- **Reporting.** Launch is not a gate: GitHub-hosted runners can only carry
  signals (no secrets, no `/dev/kvm`). KVM work stays on the host and its
  result is reported back as a check or comment.

## What this tool does not do

The command surface stays launch-oriented. There is no event broker, no
webhook receiver, no durable admission queue for pending work, no scheduler and
no multi-host placement, and no Kubernetes or k0s operator. There is also no
self-hosted runner, no auto-merge and no bypass of review. A hosted CI job in
this repository must never launch a VM; KVM acceptance remains a recorded
host-side run.
