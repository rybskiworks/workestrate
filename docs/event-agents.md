# Event agents on workestrate hosts

Part of issue #56. Dogfood pattern: GitHub events fan out to bounded
on-host role workloads. No NixOS install, no k0s, no new repository.

## Roles

| Workload | Strategy | Job |
| :--- | :--- | :--- |
| `event-implementer` | `parallel` | One instance per branch or PR (`workload up event-implementer:feat-x`, or `--instance pr-12`). Does the code change. |
| `event-reviewer` | singleton | One queue owner. Reads `workspaces/event-review-queue`, writes one verdict per instance id. No split-brain approvals. |
| `event-qa` | `parallel` | One instance per check run (`--new` or `--instance pr-12-check`). Reads the queue read-only, runs checks, reports. |

All three are `kind = "agent"`, registry `python:3.12-slim` images,
deny-by-default network (DNS plus `github.com`/`api.github.com:443), the
`example-litellm` discovery edge (`required = false`, golden-compatible),
`LITELLM_MASTER_KEY` guest-bound plus optional `GITHUB_TOKEN`, a private
state mount, and the shared queue mount (`/queue`, read-write except qa
which is read-only). Each role seeds a private inbox copy from
`config.reference/agents/<role>/config/queue.json` on first start.

## Branch and instance mapping

- Branch work: `workload up event-implementer:feat-x` plans instance
  `event-implementer@feat-x` beside `event-implementer@main`.
- PR work: `workload up event-implementer --instance pr-12`.
- QA runs: `workload up event-qa --new`, or `--instance pr-12-check`.
- Review stays singleton: one `event-reviewer` owns the verdict queue.
- State mounts are instance-scoped for `per-dir` strategy only; parallel
  instances share the declared state root and coordinate through the
  queue file, not through private disk.

## Event wiring (v1, signal-only)

`.github/workflows/event-agents.yml` runs on hosted runners with no
secrets and no KVM. It authorizes the event (allow-listed actor, label,
or verb), derives the instance id, and posts a Check or comment pointing
at the host-side run. It never launches a microVM itself. Host-side
launch (`workload up ...`) is an explicit operator step until the broker
service lands.

## What this does not do

No self-hosted runner, no broker service, no k0s or Go operator, no
fleet rollout, no auto-run on every event, no new secrets, no image
builds, no live VM proof. KVM acceptance stays a separate recorded run
with host and pins, never claimed from hosted CI.
