---
name: rust-supply-chain
description: |
  Operational reference for Rust supply-chain security: auditing dependencies
  with cargo-audit / cargo-deny / cargo-vet / cargo-auditable, checking for
  vulnerabilities (RustSec advisory database), enforcing license policy,
  detecting yanked crates and duplicate versions, and wiring CI gates. Load
  when auditing Cargo.lock, configuring audit.toml / deny.toml, triaging
  RUSTSEC advisories, or hardening a Rust workspace's dependency supply chain.
  Does NOT cover general Cargo manifest mechanics (see cargo-dependencies.md)
  or MSRV enforcement (cargo-deny does not check MSRV).
---

# Rust Supply-Chain Security

Distilled from [`docs/rust/supply-chain-security.md`](../../../docs/rust/supply-chain-security.md)
and [`docs/rust/cargo-dependencies.md`](../../../docs/rust/cargo-dependencies.md) —
cite those for full rationale and source URLs; do not link upstream.

**Threat model:** every crate in the tree — especially transitive `build.rs` and
proc-macros — runs arbitrary code on dev/CI machines at compile time. "Safe
Rust" in your own source is not sufficient; trust the code that builds your code.

## Triggers

Load when: auditing `Cargo.lock` for vulnerabilities; setting up or editing
`audit.toml` / `deny.toml` / `supply-chain/` (cargo-vet); triaging a
`RUSTSEC-YYYY-NNNN` advisory, yanked crate, or unmaintained/unsound finding;
enforcing license policy, source allowlists, duplicate-version or wildcard bans;
adding supply-chain CI gates or release SBOM steps; reviewing a PR that adds,
removes, or updates dependencies.

## Tool Selection

| Need | Tool |
|------|------|
| Vuln-scan `Cargo.lock` / auto-remediate / scan binaries | cargo-audit (`audit`, `audit fix`, `audit bin`) |
| Single CI gate (advisories+licenses+bans+sources) | cargo-deny (`deny check`) |
| Upstream prevention (audit every new dep) | cargo-vet (`vet`) |
| Embed dep tree in binary / CycloneDX SBOM / publishers | cargo-auditable / cargo-cyclonedx / cargo-supply-chain |

Both cargo-audit and cargo-deny consult the same RustSec advisory-db. Run
cargo-deny in CI; run cargo-audit for binary scanning / `audit fix`.

## cargo-audit

```bash
cargo install cargo-audit --locked      # MSRV 1.88
cargo audit                             # scan Cargo.lock (auto-fetches advisory-db; no `audit fetch` subcommand)
cargo audit fix                         # auto-remediate via cargo update (--dry-run; edits Cargo.lock, not Cargo.toml)
cargo audit bin target/release/<bin>    # scan compiled binary (best with cargo-auditable)
cargo audit --no-fetch                  # offline / -n ; --deny=warnings for non-zero exit
```

- `audit fix` cannot fix advisories with empty `patched` lists or `=`/`<=` reqs.
  `audit.toml` search: `./.cargo/audit.toml` then `~/.cargo/audit.toml`; no
  `[rustc]`/`[packages]`/`[sources]`/`[ignore]` table; field is `informational`
  (not `information`); no `unreviewed` type. Also supports `[database]`
  (`path`/`fetch`) and `[yanked]` (`enabled`/`update_index`).

### audit.toml essentials

```toml
[advisories]
ignore = ["RUSTSEC-2020-0071"]          # prefer object form: { id = "...", reason = "..." }
informational_warnings = ["unmaintained", "unsound", "notice"]
severity_threshold = "medium"           # only filters advisories with a cvss field
```

## cargo-deny (v2 schema)

```bash
cargo install --locked cargo-deny       # MSRV 1.88
cargo deny init && cargo deny check     # all four checks (advisories|licenses|bans|sources)
```

Exit codes are a bitset: advisories=1, bans=2, licenses=4, sources=8.

### deny.toml (v2 required)

```toml
[advisories]
version = 2
yanked = "warn"                        # allow | warn | deny
unmaintained = "all"                   # all | workspace | transitive | none
unsound = "workspace"
ignore = [{ id = "RUSTSEC-2020-0071", reason = "transitive via chrono, tracked" }]
[licenses]                             # v2: ALL licenses denied unless in `allow`
version = 2
confidence-threshold = 0.8
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib"]
# [[licenses.exceptions]] per-crate override (key is `crate`, not `name`): allow=["MPL-2.0"]; crate="webpki-roots"
# [[licenses.clarify]] overrides auto-detected license: crate, expression, license-files=[{path,hash}]
[bans]
multiple-versions = "warn"             # escalate to "deny" once clean
wildcards = "deny"; highlight = "all"
# [[bans.deny]] / [[bans.skip]] / [[bans.skip-tree]]: use `crate = "name@version"`, NOT {name=...}; skip = ignore one crate's duplicate, skip-tree = +N subtree levels
[sources]
unknown-registry = "deny"              # allow | warn | deny
unknown-git = "deny"
required-git-spec = "rev"              # force commit-pinned git deps
allow-registry = ["sparse+https://index.crates.io/"]
```

### Removed v1 keys (now error)

`[advisories]`: `severity-threshold`, `notice`, `vulnerability` · `[licenses]`:
`deny`, `copyleft`, `allow-osi-fsf-free`, `default`, `unlicensed` · `[bans]`:
`{ name, version }` object → use `crate = "name@version"` · `[sources]`:
`replace-source` (source replacement lives in `.cargo/config.toml`).
cargo-deny does **NOT** check MSRV — use `rust-version`, resolver v3 fallback,
and `cargo +<msrv> check` in CI.

## Advisory Workflow

1. `cargo audit` / `cargo deny check advisories` reports `RUSTSEC-YYYY-NNNN`.
2. Read the full advisory via `docs/rust/supply-chain-security.md`.
3. Remediate: `cargo audit fix` or `cargo update -p <crate> --precise <ver>`.
4. If unfixable now, add an **object-form** ignore with `reason` + tracking
   issue (never ignore without a documented reason). Treat
   `unmaintained`/`unsound` as blockers for **new** code.

**Yanked crates:** cargo-audit checks via `[yanked]`; resolver ignores yanked
versions on fresh resolves but keeps them if already in `Cargo.lock`.
**Duplicate versions:** `cargo deny check bans` + `cargo tree -d`; start
`multiple-versions = "warn"`, escalate to `"deny"` once clean. **License
policy (default):** permissive allow-list (`MIT`, `Apache-2.0`, `BSD-2-Clause`,
`BSD-3-Clause`, `ISC`, `Zlib`); add per-crate `[[licenses.exceptions]]` for
copyleft (e.g. MPL-2.0) with a reason. Embedded/firmware builds should deny
all copyleft.

## cargo-vet (high-assurance) & cargo-auditable (SBOM)

```bash
cargo install --locked cargo-vet cargo-auditable cargo-audit
cargo vet init                          # creates supply-chain/{audits,config}.toml; auto-populates exemptions
cargo vet                               # = cargo vet check (default); also: certify/suggest/import
cargo auditable build --release         # embeds dep tree in .dep-v0 section (<4 KiB, reproducible)
cargo audit bin target/release/<bin>    # reads embedded data
```

- **cargo-vet:** every new dep must be audited or exempted. Audits need
  `--who`/`--criteria`/`--start`; trust entries need `--end` (max one year,
  renewable); imports are non-transitive. **cargo-auditable** embeds crate
  names+versions (URLs redacted); does **NOT** protect against supply-chain
  attacks — use cargo-vet for prevention; this is defense-in-depth for
  post-build auditing.

## CI Integration
```yaml
# .github/workflows/supply-chain.yml — PR + weekly (fresh advisories land continuously)
on: { pull_request: , schedule: [{ cron: '0 4 * * 1' }] }
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install --locked cargo-audit cargo-deny
      - run: cargo deny check && cargo audit --deny=warnings
```
Or `EmbarkStudios/cargo-deny-action@v2` with `command: check`.

## Review Checklist

- PR changes `Cargo.toml`/`Cargo.lock`? → re-run `cargo deny check` + `cargo audit`
  (must pass all four categories; no new `cargo audit` findings).
- Any ignored advisory uses object form with `reason` + tracking issue.
- New git deps pinned to `rev` (not bare `branch`); `required-git-spec = "rev"` set.
- New deps' licenses in `[licenses].allow` (or a documented `[[licenses.exceptions]]`);
  `cargo deny check bans` clean of new duplicates; no `version = "*"` wildcards.
- New `build.rs`/proc-macro deps justified; `deny.toml` changes documented.

## Anti-patterns

- Using removed v1 deny.toml keys (`copyleft`, `deny`, `{name,version}`).
- Running `cargo audit fetch` (does not exist — fetch is automatic); silencing
  low-severity advisories globally via `severity_threshold`.
- Bare `branch = "main"` git deps in production.
- Treating `cargo vendor` as a security boundary (it is not — record advisory-db
  commit + `Cargo.lock` hash for provenance).
- Assuming cargo-deny enforces MSRV (it does not — use `cargo +<msrv> check`);
  ignoring `unmaintained`/`unsound` as "just informational" (blockers for new code).
- Not re-running audit after `cargo update`; exact-pinning (`=x.y.z`) on
  rapidly-evolving UI crates (reserve for crypto/security: `ring`, `rustls`, `aws-lc-rs`).

## Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| `unknown field 'copyleft'`/`'name'` | v1 key in v2 config | Remove; use `[licenses].allow`; bans use `crate = "name@version"` |
| `cargo audit fetch` fails | Subcommand does not exist | Use `cargo audit` (auto-fetch) or `--no-fetch` |
| New advisory after unchanged lockfile | PR-only checks miss new advisories | Add scheduled (weekly) audit job |
| Yanked version still resolves / `multiple-versions` blocks PRs | Already in `Cargo.lock` / transitive dep not updated | `cargo update -p <crate>`; start with `"warn"` + `[[bans.skip]]` |
| `cargo deny check` passes but MSRV broken | cargo-deny ignores MSRV | Add `cargo +<msrv> check` to CI |

## References

- [`docs/rust/supply-chain-security.md`](../../../docs/rust/supply-chain-security.md) — full guidance, advisory schema, incidents, policy decisions.
- [`docs/rust/cargo-dependencies.md`](../../../docs/rust/cargo-dependencies.md) — manifest mechanics, version pinning, resolver, vendoring, `[patch]`.
