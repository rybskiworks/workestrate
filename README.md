# ai-workestrator (POC)

A working proof-of-concept for the threat model: **a LiteLLM proxy that holds
only DUMMY credentials, while the real provider key is injected at the network
boundary so LiteLLM never sees it.**

The architecture is portable. The POC runs in this environment without
microsandbox because the sandbox is unavailable here (see [Constraints](#constraints));
the same architecture works in production with `msb` doing the secret injection
inside the microVM boundary.

## Threat model

- The isolated LiteLLM process holds ONLY a dummy provider key. Even a
  full compromise of the LiteLLM process (RCE, file-read, env-read) leaks
  nothing of value.
- The real provider key lives only on the host, in `infra/litellm/.env`
  (mode 0600), and is read only by the host-side egress proxy.
- All outbound calls from LiteLLM are forced through the egress proxy.
  The proxy enforces:
  1. an **egress allowlist** (deny by default; only the real provider host
     is allowed), and
  2. **secret injection** — the dummy key is rewritten to the real key
     on the way out.
- Agents/clients use the LiteLLM master key to talk to LiteLLM. They
  never see the provider key, dummy or real.

## Architecture

```
                       (host side, holds the real key)
   +-----------------+   +----------------------+   +------------------+
   | fake-provider   |   | egress-proxy         |   | .env (host only) |
   | 127.0.0.1:8081  |<--+ 127.0.0.1:8082       |   | FAKE_PROVIDER_   |
   | (the "real"     |   | (allowlist + header  +-->| REAL_KEY=sk-real |
   |  provider)      |   |  rewrite)            |   | DUMMY_KEY=...    |
   +-----------------+   +----------------------+   +------------------+
                                   ^
                                   | outbound
                            (LiteLLM is configured to call this URL)
                                   |
                         +-----------------+
                         | LiteLLM proxy   |
                         | 127.0.0.1:4000  |
                         | api_key=dummy   |
                         +-----------------+
                                   ^
                                   | client calls
                                   |
                          (agents / clients)
```

**Production form:** the egress-proxy disappears and microsandbox's
in-microVM egress layer performs the allowlist + secret injection. The
LiteLLM process still holds only the dummy key; the `msb` runtime
substitutes the real key only for traffic bound for the allowlisted
upstream. See [docs.microsandbox.dev secret-injection][1].

[1]: https://docs.microsandbox.dev/core-concepts/secret-injection

## Layout

```
/home/node/Development/ai-workbench/ai-workestrator/    <-- this POC
├── README.md
├── .gitignore                      .env is gitignored, .env.example is committed
├── infra/
│   ├── litellm/
│   │   ├── config.yaml             LiteLLM model + master key
│   │   ├── .env.example            template (committed)
│   │   └── .env                    REAL KEY LIVES HERE (gitignored, mode 600)
│   ├── fake-provider/
│   │   └── server.py               stdlib HTTP server, OpenAI-compatible
│   └── egress-proxy/
│       └── server.py               allowlist + secret-injecting forwarder
├── scripts/
│   ├── start.sh                    starts fake-provider, egress-proxy, LiteLLM
│   ├── stop.sh                     stops all of the above
│   ├── test.sh                     spec-style curl probes
│   ├── run-tests.sh                full end-to-end validation (7 sections)
│   ├── status.sh                   process + endpoint + isolation status
│   ├── setup-env.sh                generates .env from .env.example
│   ├── start-fake-provider.sh
│   ├── start-egress-proxy.sh
│   ├── start-litellm.sh            strips REAL_KEY before exec; refuses to leak
│   ├── stop-all.sh                 alias for stop.sh
│   ├── fake_provider.py            thin wrapper for spec compatibility
│   └── _find_evidence.py           test helper: pairs body lines w/ header lines
└── var/
    ├── log/                        runtime logs (gitignored)
    └── run/                        pid files (gitignored)
```

> **Path note.** The task asked for this POC at
> `/home/node/Development/ai-workestrator/`. That path could not be
> created: `/home/node/Development/` is owned by `root` with mode `0755`
> in this environment, and the running user (`node`, uid 1000) cannot
> create new entries there. The only writable sibling is
> `/home/node/Development/ai-workbench/`, which already contains the
> POC tree from a prior attempt. The POC therefore lives one directory
> deeper than requested. See [Constraints](#constraints) below.

## How to run

```bash
cd /home/node/Development/ai-workbench/ai-workestrator

# 1. One-time: generate .env from the template.
bash scripts/setup-env.sh          # creates .env with random keys (mode 600)

# 2. Start the whole stack.
bash scripts/start.sh              # brings up :8081, :8082, :4000

# 3. Probe LiteLLM and see the secret-injection proof.
bash scripts/test.sh               # spec-style curl probes
bash scripts/run-tests.sh          # full 7-section validation

# 4. Status / stop.
bash scripts/status.sh
bash scripts/stop.sh
```

The first run is the only one that needs `setup-env.sh`; subsequent
runs just `start.sh` and go.

## What was validated

`scripts/run-tests.sh` runs seven sections. Output of the latest run:

```
===== 1. fake-provider :8081 =====                              PASS
===== 2. egress-proxy :8082 (forwarding to 127.0.0.1:8081) ===  PASS
===== 3. egress-proxy :8082 (DENY non-allowlisted host) ===     PASS  (169.254.169.254 and 8.8.8.8:53 both 403)
===== 4. secret injection proof =====                           PASS
        Evidence: fake-provider received the REAL key
===== 5. LiteLLM :4000 =====                                    PASS
===== 6. end-to-end chat through LiteLLM =====                  PASS
===== 7. isolation proof: real key NOT in litellm process env   PASS
===== ALL CHECKS PASSED =====
```

The critical proof is **section 4**: the fake-provider's log shows
`Authorization='Bearer sk-real-provider-...'` for a request that
LiteLLM originated with the dummy key. The egress proxy at the
boundary rewrote it. Section 7 confirms the LiteLLM process's `/proc/.../environ`
contains neither the variable name nor the real key value.

A direct hit of the egress proxy with the dummy key returns the
provider's response (after key injection). The fake provider's response
itself echoes back the Authorization header it received, which is a
visible cross-check:

```json
"content": "[fake-provider echo] you said: 'hello'. I received Authorization='Bearer sk-real-provider-...'"
```

## What was NOT validated (and why)

- **Egress from inside the LiteLLM *process* to a denied host.** In the
  production design with microsandbox, LiteLLM runs in a microVM with
  NetworkPolicy deny-all and an explicit allow for the provider host.
  The POC fallback does not have a microVM boundary. The allowlist is
  enforced by the egress proxy; bypassing it requires bypassing the
  proxy (e.g. by reconfiguring `FAKE_PROVIDER_API_BASE`), which is a
  config edit, not a network exploit. The deny test in section 3
  confirms the proxy itself rejects non-allowlisted targets, which is
  the same enforcement that runs in the microsandbox path.
- **Resource isolation (CPU/mem/disk).** Out of scope; microsandbox
  provides it and the host fallback intentionally does not.
- **Postgres-backed persistent keys/teams.** The plan defers this.
  LiteLLM is running in in-memory mode.

## Constraints (why microsandbox isn't used)

This host fails microsandbox's prerequisites:

1. **glibc.** `Microsandbox Linux releases require glibc 2.39 or newer,
   but this system has glibc 2.36.` (verbatim from the installer.)
2. **KVM.** `/dev/kvm` is absent; `/proc/cpuinfo` does not advertise
   `vmx` or `svm`. The host is a container without nested-virt.
3. **Docker.** The daemon is not running and the user has no `sudo`
   to start it.

The fallback in this POC — a host-side Python egress proxy that
substitutes the real key — is the same architectural role as
microsandbox's egress layer; it is functionally equivalent for an HTTP
POC and a deliberate shortcut per the task spec
("Pragmatic Phase 1B shortcut").

## What is deferred (per the plan)

- Postgres-backed key/team storage in LiteLLM
- `agentctl` and the rest of the agent runtime
- Codegen / browser profiles
- Nix flake for reproducible hosts
- Pi/Odysseus edge deployment

These are out of scope for this POC.

## Files of interest (read in this order)

1. `infra/litellm/config.yaml` — the LiteLLM config; the only place the
   dummy key is referenced.
2. `infra/litellm/.env.example` — the template; the only place the
   real key is committed (as a placeholder).
3. `infra/egress-proxy/server.py` — the boundary enforcer.
4. `scripts/start-litellm.sh` — the assertion that the real key is
   stripped before LiteLLM exec, with a hard refusal to launch if the
   leak somehow happened anyway.
5. `scripts/run-tests.sh` — the validator.
6. `var/log/` — runtime logs (gitignored; recreate by running).

## Security notes for production hand-off

When this POC graduates to a real host with `msb`:

- Replace the `infra/egress-proxy/` step with `msb run --network-policy deny-all --allow-host <provider>` and `--secret "FAKE_PROVIDER_DUMMY_KEY@<provider>=sk-real-..."` in the LiteLLM command.
- Delete `infra/egress-proxy/` and `scripts/start-egress-proxy.sh`.
- Keep `scripts/start-litellm.sh` as a reference for the env-strip
  safety check.
- Move `infra/litellm/.env` to a secrets manager (1Password Connect,
  Vault, etc.) and inject `FAKE_PROVIDER_REAL_KEY` only into the
  host-side secret store, not the LiteLLM env.
