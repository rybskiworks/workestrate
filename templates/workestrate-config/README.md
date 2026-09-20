# workestrate-fleet: {{ config_name }}

This is a workestrate fleet, generated from the
`workestrate-config` copier template.

## Structure

```
workestrate.toml     # workload definitions + secrets schema
.env.enc             # SOPS-encrypted secrets (NOT committed until encrypted)
.sops.yaml           # SOPS config (age recipients + creation rules)
.env.example         # schema example (regenerate: `workestrate generate-env-example`)
infra/<service>/     # service config values (e.g. config.yaml, models.yaml)
agents/*/config/     # agent config files (models.json, settings.json, etc.)
```

## Build artifacts: `.workestrate-build/`

`.workestrate-build/` at the repo root is the reserved workload-agnostic
build-artifact location (spec 21 §6): one subdirectory per workload
(`.workestrate-build/<workload-name>/`), shared across recipes — e.g.
`local_build` outputs. It is also the new default output for `local_build`
recipes with no declared output dir. The directory is gitignored, which makes
it artifact-only BY CONSTRUCTION: a `git+file` / path-flake input sees only
committed content, so nothing in it can ever become a flake input source —
the artifact/source separation is structural, not discipline. It is created
on demand by the tool/build; there is nothing to provision manually.

## Secrets & env (the model in three lines)

Env bindings default to the **placeholder** — workloads never see a real
credential unless you opt in. The real value in-sandbox is an explicit
`bound = "guest"` on the binding, reserved for workloads that **verify** the
credential (e.g. a proxy service verifying its callers). `secret` appears at a binding only when
**renaming** (env name ≠ secret ID); same-name bindings are just `KEY = true`.

## Setup

1. Generate your age key (if not already done):
   ```bash
   age-keygen -o ~/.config/sops/age/ai-workbench-secrets.txt
   age-keygen -y ~/.config/sops/age/ai-workbench-secrets.txt  # print public key
   ```

2. Update `.sops.yaml` with your public key (replace `age1PLACEHOLDER`).

3. Initialize secrets:
   ```bash
   workestrate secrets init --fleet {{ config_name }}
   ```

4. Register this fleet with workestrate:
   ```bash
   workestrate fleet add <path-or-url> {{ config_name }}
   ```

5. Verify:
   ```bash
   workestrate validate-config
   workestrate workload plan <name>
   ```

## Updating from the template

To pull in template improvements:
```bash
copier update
```

## `.env.example` regeneration

The committed `.env.example` matches this repo's scaffolded
`workestrate.toml`. Once the repo is registered and your real secrets are
declared, regenerate it from your active config:
```bash
workestrate generate-env-example --output .env.example
```

## Multi-recipient SOPS (team secrets)

If a team age key was provided during template generation, `.sops.yaml`
includes per-path creation rules:
- `secrets/shared/*.enc` → encrypted to both personal + team keys
- `secrets/{{ config_name }}/*.enc` → encrypted to personal key only
- `.env.enc` → encrypted to both (or personal only if no team key)
