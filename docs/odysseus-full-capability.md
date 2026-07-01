# Odysseus full-capability gap analysis

## 1. Summary / current state

Odysseus builds from source and runs as a single microVM today. The `nix develop` shell vendors its Python dependencies into `agents/odysseus/build/.deps`; the `python:3.12-slim` microVM imports them via `PYTHONPATH=/app/.deps` and runs `python -m uvicorn app:app --host 0.0.0.0 --port 7000`. Uvicorn boots on port 7000 and reaches the shared LiteLLM proxy microVM via `OPENAI_BASE_URL=http://host.microsandbox.internal:4000/v1` (env wired in `control/agentctl/src/workloads/odysseus.rs`). HuggingFace egress is allowed (huggingface.co, cdn-lfs.huggingface.co, cdn-lfs-us-1.huggingface.co) so fastembed can fetch its ONNX embedding model. Admin auth is being wired up (`AUTH_ENABLED=true`, `LOCALHOST_BYPASS=false`, `LITELLM_AUTH` host-bound secret).

What is missing is the rest of Odysseus's stack. It runs as a **single microVM with no companions provisioned**.

"Full capability" here means Odysseus running with its full 4-service stack intact: vector memory (chromadb), web search (searxng), and push notifications (ntfy) all reachable. Today it boots, but those three subsystems are degraded or absent.

## 2. What full Odysseus capability requires

Odysseus ships upstream (see `agents/odysseus/repo/docker-compose.yml`) as a 4-service stack. The per-companion roles, verified against that compose file and the Odysseus source:

| Service | Image | Port (container) | Role |
|---|---|---|---|
| `odysseus` | built | 7000 | FastAPI/uvicorn app — already provisioned |
| `chromadb` | `docker.io/chromadb/chroma:latest` | 8000 (compose maps host 8100→8000) | Vector store; Odysseus uses `chromadb-client` |
| `searxng` | `docker.io/searxng/searxng:<pinned>` | 8080 | Web search |
| `ntfy` | `docker.io/binwiederhier/ntfy` (`serve`) | 80 (compose maps host 8091→80) | Push notifications |

Odysseus reads its companions from env, not compose service names. `CHROMADB_HOST` (default `localhost`) and `CHROMADB_PORT` (default `8100`) are consumed in `src/chroma_client.py`; without chromadb the client raises a clear error pointing at `docker compose up chromadb`, so vector memory / RAG is absent, not silently broken. `SEARXNG_INSTANCE` (default `http://localhost:8080`) is read in `src/constants.py`; without searxng, web search is absent. ntfy is integrated as a generic integration in `src/integrations.py` (`auth_type: none`) and used as a reminder channel (`reminder_channel: "ntfy"` in `src/settings.py`) — its base URL is configured **per-integration in the Odysseus UI, not via a single env var**. Without ntfy, push notifications are absent, but browser/email/webhook reminder channels still work.

Stated plainly: without the companions, Odysseus boots and serves chat, but vector memory, web search, and push notifications are degraded or absent.

## 3. The gap

The microsandbox model is one microVM per agent, with no compose or multi-service orchestration, default-deny egress, and the project is docker-free by design (msb→KVM direct). The current Odysseus workload (`control/agentctl/src/workloads/odysseus.rs`) provisions only the `odysseus` app microVM. The three companions are not provisioned, not networked, and not wired into Odysseus's env. So Odysseus runs but is missing 3 of its 4 stack services.

## 4. Recommended approach: internalize companions as shared microsandbox service-microVMs

The recommended approach is the **same pattern already proven for the LiteLLM proxy**: one service microVM reachable at `host.microsandbox.internal:<port>`, shared across agents. The template is `control/agentctl/src/workloads/litellm.rs`, which sets `PORT=4000`, `PortMapping::same(4000)`, `IngressRule::local_tcp(4000)`, and egress to upstream providers. Odysseus already reaches it at `host.microsandbox.internal:4000` — the companions should follow the same shape.

### Bundle vs split

- **Bundle option**: one "services" microVM running chromadb + searxng + ntfy under a process supervisor (s6 / overmind / supervisord). Tradeoff: lower total RAM (one kernel, one VM overhead), simpler agentctl surface, but weaker isolation and replaceability — one companion crashing or being upgraded restarts the bundle.
- **Split option**: one microVM per companion (chromadb-vm, searxng-vm, ntfy-vm). Tradeoff: stronger isolation, independent upgrade/restart, but each microVM adds kernel + baseline RAM overhead.

RAM is the real constraint. The README targets ≥4 GB host RAM, and the LiteLLM VM already takes 2048 MiB and Odysseus 2048 MiB (`memory_mib` in their workload plans). Split-per-companion adds up fast on a 4 GB host; the bundle option mitigates this. **Recommend bundle for M1/M2, split later if isolation demands it.**

### How agents reach them

Same as LiteLLM: each companion service-microVM exposes an `IngressRule::local_tcp(<port>)` + `PortMapping::same(<port>)`; agent microVMs get an egress allow rule to `host.microsandbox.internal:<port>`. Companion ports (from compose, to verify against each image's docs before building the workload plan): chromadb 8000 (compose maps host 8100→container 8000; in microsandbox just expose 8000), searxng 8080, ntfy 80 (or pick 8091 to match the compose host mapping).

### agentctl surface

New "services" workloads mirroring the Litellm workload shape (`control/agentctl/src/workloads/litellm.rs`): e.g. `chromadb`, `searxng`, `ntfy` (or a single bundled `odysseus-services`), each implementing the `Workload` trait with `plan()`, `exec()`, `detach_args()`. CLI surface: `agentctl chromadb plan|up|down`, etc. — same shape as `agentctl litellm plan|up|down`.

### Behavior parity = env-wiring

Point Odysseus at the companions via env, not compose service names. Concrete env to add to `control/agentctl/src/workloads/odysseus.rs`:

```
CHROMADB_HOST=host.microsandbox.internal
CHROMADB_PORT=8000
SEARXNG_INSTANCE=http://host.microsandbox.internal:8080
```

ntfy has no single env var — it is configured per-integration in the Odysseus UI (base URL `http://host.microsandbox.internal:80` + `auth_type: none`). This asymmetry is intentional in upstream Odysseus and must be carried through the wiring plan.

### Persistence per companion

Each companion needs a persistent volume (`chromadb-data`, `searxng-data`, `ntfy-cache` in compose). In microsandbox this maps to `MountPlan::readwrite("${MSB_HOME}/sandboxes/<service>/data", "<container-path>")` — the same pattern as the LiteLLM logs mount.

### Companion auth

- **chromadb**: authless internal (compose sets only `ANONYMIZED_TELEMETRY=FALSE`); protect via network default-deny + the fact it is only reachable at `host.microsandbox.internal`.
- **searxng**: authless internal; `SEARXNG_SECRET` is for cookie signing, not auth.
- **ntfy**: supports per-topic tokens; for a shared instance, per-agent tokens are a design option (see §5 and §8).

## 5. Sharing across agents

searxng and ntfy are trivially shared — they are stateless-ish (searxng has a config volume; ntfy has a cache) and multi-tenant by design.

chromadb is the real design decision. Two models:

- **Collections-per-agent** (privacy): one shared chromadb VM, each agent gets its own collection namespace. Memory is isolated. Recommended default for untrusted or multi-tenant agents.
- **Shared collections** (collaborative): agents read/write the same collections — cross-agent memory. Only viable if agents are trusted and you want shared context.

The sharing model is a **design decision the user must make**: do agents share memory? That answer drives chromadb topology.

## 6. Why NOT docker-compose / k3s

**docker-compose** breaks the docker-free + microVM-isolation model. The project is docker-free by design (msb→KVM direct); introducing a docker daemon regresses the isolation guarantee (containers ≠ microVMs) and re-introduces a docker dependency the flake deliberately avoids.

**k3s** is overkill for single-host, and containers ≠ microVMs so it regresses isolation the same way. Reserve k3s for if/when the project goes multi-host or explicitly drops the microVM model. The shared-service-microVM approach (§4) gets the same multi-service outcome without leaving the microsandbox model.

## 7. "Is microVM really better?" nuance

For untrusted agents: yes — microVM isolation is the point. For trusted companions (chromadb / searxng / ntfy are well-known upstream images), it is a consistency-vs-RAM tradeoff. Running them as microVMs keeps one deployment model (everything is a microVM, everything reaches via `host.microsandbox.internal`), at the cost of RAM. The bundle option (§4) mitigates the RAM cost. This is a deliberate consistency choice, not a purity test.

## 8. Open decisions for the user

- **chromadb sharing model** — collections-per-agent (privacy) vs shared collections (collaborative). Drives chromadb topology and whether agents can read each other's memory.
- **bundle vs split** — one services microVM vs one per companion. Drives RAM footprint and isolation granularity.
- **companion auth** — authless-internal (rely on network default-deny) vs per-agent ntfy tokens vs chromadb auth. Drives secret surface.
- **agentctl services surface** — separate `chromadb` / `searxng` / `ntfy` workloads vs a single bundled `odysseus-services` workload. Drives CLI ergonomics.

## 9. Verification unknowns

- **microsandbox inter-microVM reachability beyond :4000** — proven for :4000 via the LiteLLM proxy; need to confirm generic inter-VM reach at arbitrary ports via `host.microsandbox.internal:<port>`. *(to verify)*
- **companion image exact ports/env** — taken from upstream `docker-compose.yml`; confirm against each image's docs before building the workload plan. *(to verify)*
- **Odysseus's exact config keys** — `CHROMADB_HOST` / `CHROMADB_PORT` (verified in `src/chroma_client.py`), `SEARXNG_INSTANCE` (verified in `src/constants.py`); ntfy has no single env var (configured per-integration in UI — verified in `src/integrations.py` + `src/settings.py`). *(mostly verified; ntfy wiring path to verify at runtime)*
- **graceful degradation without companions** — chromadb client raises a clear error (verified in `src/chroma_client.py`); searxng/ntfy degradation behavior at runtime to verify. This matters because it determines whether a phased rollout (§10) is safe as an interim state.
- **companion image availability without docker** — these are docker.io images; microsandbox consumes OCI images, but the project is docker-free. Need to confirm whether the companion images can be pulled/converted into the microsandbox image store, or whether Nix-built equivalents are needed (mirrors the open question in `integration-plan.md` §"Microsandbox compatibility"). *(to verify)*

## 10. Phasing suggestion

Suggestion only (not a commitment):

- **Phase 1 — chromadb as a shared service-microVM + wire Odysseus.** Biggest functional win: vector memory / RAG. Stand up a `chromadb` workload (mirror of `litellm.rs`), expose :8000, add `CHROMADB_HOST` / `CHROMADB_PORT` env to the Odysseus workload, add an egress allow rule to `host.microsandbox.internal:8000`. Decide the sharing model (§5).
- **Phase 2 — searxng + ntfy.** Smaller functional win (search + notifications). Same pattern. ntfy requires per-integration UI config in Odysseus (note the asymmetry in §4).
- **Phase 3 — sharing-model hardening.** If Phase 1 chose collections-per-agent, harden the namespace boundary; if shared, add access controls. Revisit bundle-vs-split if RAM pressure appears.

Hook points: `control/agentctl/src/workloads/litellm.rs` (template), `control/agentctl/src/workloads/odysseus.rs` (env to extend), `infra/litellm/config.yaml` (existing shared-service precedent).
