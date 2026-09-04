---
name: validation-litellm-startup
description: |
  Verifies the LiteLLM proxy boots successfully with a given config.yaml. Load
  after the static config check passes, before smoke testing. Does NOT cover
  static config schema validation (see validation-litellm-config-check) or
  live request smoke testing (see validation-litellm-smoke).
metadata:
  org.kind: validation
---

# Validation: LiteLLM Startup

This gate verifies that the LiteLLM proxy actually starts with the config. It
is the runtime boot check that sits between the static config check and the
live smoke test. A config that passes static validation can still fail at
startup due to Pydantic validation errors, missing env vars at runtime, or
provider-adapter import failures.

## Triggers

Load this skill when:

- After `validation-litellm-config-check` passes, before smoke testing.
- When diagnosing whether a proxy failure is a startup/boot error vs. a config
  schema error.
- Before declaring a config change safe to deploy.

## Command

Boot the proxy with the config and capture the startup log:

```bash
litellm --config <path-to-config.yaml> --model <default-model-name> \
  --port 4000 > /tmp/litellm-startup.log 2>&1 &
PROXY_PID=$!
sleep 10
kill $PROXY_PID 2>/dev/null
cat /tmp/litellm-startup.log
```

Or via Docker (when not running inside the devshell (`just shell`)):

```bash
docker run --rm -d --name litellm-test -p 4000:4000 \
  -v <abs-path-to-config.yaml>:/app/config.yaml \
  ghcr.io/berriai/litellm:main-latest \
  --config /app/config.yaml --model <default-model-name> --port 4000
sleep 15
docker logs litellm-test
docker stop litellm-test
```

## Pass criteria

- The proxy process starts (exit 0 / container reaches running state).
- The startup log contains the line:
  `Loaded config YAML (api_key and environment_variables are not shown)`.
- No `Pydantic validation error` or `Traceback` in the startup log.

## Fail criteria

- The proxy process exits non-zero during startup.
- Startup log contains a Pydantic validation error, a traceback, or a
  `config` / `KeyError` / `ImportError` message.
- The "Loaded config YAML" success line is absent.

## Evidence to report

- Exit code of the proxy process (or container status).
- The full startup log (or the relevant excerpt around the error).
- Presence/absence of the "Loaded config YAML" success line.

## Notes

- Requires the `litellm` binary (installed via `pip install litellm` or
  available inside the devshell (`just shell`)) OR the LiteLLM Docker image.
- In this environment there is no KVM, so the Docker-based path is
  `not_fully_checkable` here. Prefer the `litellm` CLI path inside
  the devshell (`just shell`), or run this gate on a KVM-capable host.
- The proxy needs `LITELLM_MASTER_KEY` (and all provider API keys referenced via
  `os.environ/`) injected into the runtime environment; static validation
  cannot confirm they are present — this gate does.
- Use a short `sleep` (10-15s) and then kill the process; this gate only checks
  boot, not serving. Serving is `validation-litellm-smoke`.
- Do not leave the proxy running after this gate unless proceeding directly to
  the smoke test.
