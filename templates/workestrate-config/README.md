# workestrate-config: {{ config_name }}

This is a workestrate configuration repo, generated from the
`workestrate-config` copier template.

## Structure

```
workestrate.toml     # workload definitions + secrets schema
.env.enc             # SOPS-encrypted secrets (NOT committed until encrypted)
.sops.yaml           # SOPS config (age recipients + creation rules)
.env.example         # auto-generated schema (run `workestrate generate-env-example`)
infra/litellm/       # LiteLLM config values (config.yaml, models.yaml)
agents/*/config/     # agent config files (models.json, settings.json, etc.)
```

## Secrets & env (the model in three lines)

Env bindings default to the **placeholder** — workloads never see a real
credential unless you opt in. The real value in-sandbox is an explicit
`bound = "guest"` on the binding, reserved for workloads that **verify** the
credential (e.g. litellm itself). `secret` appears at a binding only when
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
   setup-secrets --config {{ config_name }} init
   ```

4. Register this config repo with workestrate:
   ```bash
   workestrate config add <path-or-url> {{ config_name }}
   ```

5. Verify:
   ```bash
   workestrate validate-config
   workestrate pi plan
   ```

## Updating from the template

To pull in template improvements:
```bash
copier update
```

## `.env.example` regeneration

If `workestrate` is on your PATH when you run `copier copy` or `copier update`,
the `.env.example` file is automatically regenerated from the live secrets
schema via `workestrate generate-env-example`. If `workestrate` is not on PATH,
the committed static `.env.example` is used as a fallback.

## Multi-recipient SOPS (team secrets)

If a team age key was provided during template generation, `.sops.yaml`
includes per-path creation rules:
- `secrets/shared/*.enc` → encrypted to both personal + team keys
- `secrets/{{ config_name }}/*.enc` → encrypted to personal key only
- `.env.enc` → encrypted to both (or personal only if no team key)
