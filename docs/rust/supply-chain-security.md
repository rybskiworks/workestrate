# Supply Chain Security: cargo-audit, cargo-deny, RustSec, cargo-vet

## Purpose

Provide practical, opinionated, repo-independent guidance on Rust supply-chain security for future AI coding agents. This document focuses on vulnerability auditing (`cargo-audit`), comprehensive supply-chain linting (`cargo-deny`), upstream dependency review (`cargo-vet`), build-time provenance (`cargo-auditable`), SBOM generation, source restrictions, and the RustSec advisory database.

For general Cargo mechanics — manifest structure, version requirements, features, resolver versions, workspaces, vendoring, `[patch]`, and `[source]` replacement — see [`cargo-dependencies.md`](cargo-dependencies.md). This document builds on that one and cross-links it rather than duplicating it.

## Sources used

- <https://rustsec.org/>
- <https://rustsec.org/advisories/>
- <https://github.com/rustsec/rustsec>
- <https://github.com/rustsec/rustsec/tree/main/cargo-audit>
- <https://github.com/EmbarkStudios/cargo-deny>
- <https://embarkstudios.github.io/cargo-deny/>
- <https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html>
- <https://doc.rust-lang.org/cargo/reference/features.html>
- <https://doc.rust-lang.org/cargo/reference/resolver.html>
- <https://github.com/rustsec/advisory-db>
- <https://github.com/rustsec/advisory-db/blob/main/CONTRIBUTING.md>
- <https://rustsec.org/contributing.html>
- <https://embarkstudios.github.io/cargo-deny/checks/advisories/cfg.html>
- <https://embarkstudios.github.io/cargo-deny/checks/licenses/cfg.html>
- <https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html>
- <https://embarkstudios.github.io/cargo-deny/checks/sources/cfg.html>
- <https://embarkstudios.github.io/cargo-deny/cli/check.html>
- <https://github.com/EmbarkStudios/cargo-deny/blob/main/deny.template.toml>
- <https://github.com/EmbarkStudios/cargo-deny-action>
- <https://github.com/mozilla/cargo-vet>
- <https://mozilla.github.io/cargo-vet/>
- <https://raw.githubusercontent.com/mozilla/cargo-vet/main/registry.toml>
- <https://github.com/rust-secure-code/cargo-auditable>
- <https://github.com/CycloneDX/cyclonedx-rust-cargo>
- <https://github.com/rust-secure-code/cargo-supply-chain>
- <https://github.com/rust-secure-code/wg>
- <https://github.com/taiki-e/cargo-hack>
- <https://doc.rust-lang.org/cargo/reference/source-replacement.html>
- <https://doc.rust-lang.org/cargo/reference/build-scripts.html>
- <https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-tree.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-metadata.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-vendor.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-yank.html>
- <https://doc.rust-lang.org/cargo/reference/rust-version.html>
- <https://spdx.org/licenses/>
- <https://blog.rust-lang.org/inside-rust/2023/09/01/crates-io-malware-postmortem/>
- <https://blog.rust-lang.org/2022/05/10/malicious-crate-rustdecimal/>
- <https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html>
- <https://github.com/rust-lang/crates.io/issues/10247>
- <https://github.com/rust-lang/rfcs/pull/3691>

### Crawl ledger

SEED:

- <https://rustsec.org/>
- <https://rustsec.org/advisories/>
- <https://github.com/rustsec/rustsec>
- <https://github.com/rustsec/rustsec/tree/main/cargo-audit>
- <https://github.com/EmbarkStudios/cargo-deny>
- <https://embarkstudios.github.io/cargo-deny/>
- <https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html>
- <https://doc.rust-lang.org/cargo/reference/features.html>
- <https://doc.rust-lang.org/cargo/reference/resolver.html>

VISITED:

- <https://github.com/rustsec/advisory-db>
- <https://github.com/rustsec/advisory-db/blob/main/CONTRIBUTING.md>
- <https://rustsec.org/contributing.html>
- <https://embarkstudios.github.io/cargo-deny/checks/advisories/cfg.html>
- <https://embarkstudios.github.io/cargo-deny/checks/licenses/cfg.html>
- <https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html>
- <https://embarkstudios.github.io/cargo-deny/checks/sources/cfg.html>
- <https://embarkstudios.github.io/cargo-deny/cli/check.html>
- <https://github.com/EmbarkStudios/cargo-deny/blob/main/deny.template.toml>
- <https://github.com/EmbarkStudios/cargo-deny-action>
- <https://github.com/mozilla/cargo-vet>
- <https://mozilla.github.io/cargo-vet/>
- <https://raw.githubusercontent.com/mozilla/cargo-vet/main/registry.toml>
- <https://github.com/rust-secure-code/cargo-auditable>
- <https://github.com/CycloneDX/cyclonedx-rust-cargo>
- <https://github.com/rust-secure-code/cargo-supply-chain>
- <https://github.com/rust-secure-code/wg>
- <https://github.com/taiki-e/cargo-hack>
- <https://doc.rust-lang.org/cargo/reference/source-replacement.html>
- <https://doc.rust-lang.org/cargo/reference/build-scripts.html>
- <https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-tree.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-metadata.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-vendor.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-yank.html>
- <https://doc.rust-lang.org/cargo/reference/rust-version.html>
- <https://spdx.org/licenses/>
- <https://blog.rust-lang.org/inside-rust/2023/09/01/crates-io-malware-postmortem/>
- <https://blog.rust-lang.org/2022/05/10/malicious-crate-rustdecimal/>
- <https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html>
- <https://github.com/rust-lang/crates.io/issues/10247>
- <https://github.com/rust-lang/rfcs/pull/3691>

SKIPPED (404 / out of scope / duplicated):

- <https://rustsec.org/policies/> — 404; RustSec policy content lives at <https://rustsec.org/contributing.html>
- <https://github.com/CycloneDX/cargo-cyclonedx> — 404; canonical repo is <https://github.com/CycloneDX/cyclonedx-rust-cargo>
- <https://github.com/rust-secure-code/cargo-vet> — 404; canonical repo is <https://github.com/mozilla/cargo-vet>

## Core guidance

### Threat model in one sentence

Every crate in your dependency tree — especially transitive build-dependencies and proc-macros — can run arbitrary code on developer and CI machines. Cargo executes `build.rs` and proc-macros at compile time, so "safe Rust" in your own source is not enough; you must also trust the code that builds your code.

### cargo-audit

`cargo-audit` scans `Cargo.lock` against the [RustSec advisory database](https://github.com/rustsec/advisory-db) and reports vulnerabilities, unmaintained crates, unsound APIs, yanked releases, and notices. Sources: [cargo-audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit), [RustSec](https://github.com/rustsec/rustsec).

Install (MSRV `rust-version = "1.88"`):

```bash
cargo install cargo-audit --locked
```

Core subcommands:

```bash
cargo audit                            # audit Cargo.lock
cargo audit fix                        # auto-remediate via cargo update; requires --features=fix
cargo audit bin <PATH>...              # scan compiled binaries
```

There is **no** `cargo audit fetch` subcommand. The advisory database is fetched automatically on every `cargo audit`; control it with `--no-fetch`/`-n` or `[database] fetch = false` in `audit.toml`.

`cargo audit fix` shells out to `cargo update` to upgrade vulnerable deps to patched, SemVer-compatible versions. It modifies `Cargo.lock` indirectly; it does **not** edit `Cargo.toml`. It supports `--dry-run`. It cannot fix advisories with empty `patched` lists (e.g. many `unmaintained` advisories) or where the `Cargo.toml` requirement is too restrictive (`=` / `<=`).

`cargo audit bin` scans compiled binaries (ELF/PE/Mach-O). Accuracy is full when the binary was built with [`cargo auditable`](https://github.com/rust-secure-code/cargo-auditable); otherwise it recovers partial info from panic messages. Flags: `--max-binary-size` (default 100 MiB), `--audit-data-size-limit` (default 8 MiB).

CLI flags (exact, from source): `-c/--color`, `-D/--deny {warnings|unmaintained|unsound|yanked}` (repeatable), `-f/--file` (`Cargo.lock` path; `-` for STDIN), `--ignore` (repeatable), `--format {terminal|json|sarif}`, `--json`, `-n/--no-fetch`, `--no-yanked`, `--stale`, `--target-arch`, `--target-os`, `-u/--url`, `-q/--quiet`, `-v/--verbose`.

#### audit.toml configuration

Search order: `./.cargo/audit.toml` then `~/.cargo/audit.toml`. All tables use `#[serde(deny_unknown_fields)]`.

```toml
[advisories]
ignore = ["RUSTSEC-2020-0071"]
informational_warnings = ["unmaintained", "unsound", "notice"]
severity_threshold = "medium"          # none | low | medium | high | critical

[database]
path = "~/.cargo/advisory-db"
url = "https://github.com/rustsec/advisory-db"
fetch = true
stale = false

[output]
deny = ["warnings", "unmaintained", "unsound", "yanked"]
format = "terminal"                    # terminal | json | sarif
quiet = false
show_tree = true

[target]
arch = []
os = []

[yanked]
enabled = true
update_index = true
```

`informational_warnings` defaults to all three when unset. `severity_threshold` only filters advisories with a `cvss` field; advisories without `cvss` are not filtered.

Do not use nonexistent tables/keys: no `[rustc]`, `[rpm-ostree]`, `[packages]`, `[sources]`, or `[ignore]` table; no `unreviewed` informational type; no `unthreshold` key; no advisory fields `solution`, `information` (the field is `informational`), or `invalidated_by`.

### RustSec advisory database

The [advisory-db](https://github.com/rustsec/advisory-db) is the canonical data source for `cargo-audit` and `cargo-deny`. The [RustSec website](https://rustsec.org/) and [advisories page](https://rustsec.org/advisories/) provide a browsable index.

Repo layout (from source): `crates/` (one subdir per crate, e.g. `crates/lz4-sys/RUSTSEC-2022-0051.md`), `rust/` (Rust toolchain advisories), `LICENSES/`, plus `CONTRIBUTING.md`, `README.md`, `EXAMPLE_ADVISORY.md`, `HOWTO_UNMAINTAINED.md`, `MAINTAINERS_GUIDE.md`. There is no `misc/` directory. A separate `osv` branch holds auto-generated OSV-format export (queryable at <https://osv.dev>; also mirrored to GitHub Advisory Database).

Advisory files are Markdown with a TOML front-matter block (fenced ```toml … ```), then `# Title` and body. They are not pure TOML.

#### Advisory TOML schema

`[advisory]` table: `id` (mandatory; use `RUSTSEC-0000-0000` placeholder in PRs), `package` (mandatory), `date` (RFC 3339, mandatory), `title`/`description` (optional; often taken from Markdown), `aliases`, `related`, `categories`, `keywords`, `cvss`, `informational` (`"unsound"|"unmaintained"|"notice"`), `references`, `url`, `withdrawn`, `license` (`CC0-1.0` default or `CC-BY-4.0`), `expect-deleted`.

`[affected]` table: `arch`, `os`, `functions` (table mapping `"path::func"` to VersionReq array).

`[versions]` table: `patched` (array; can be `[]`), `unaffected`.

There is no top-level `patched`, no `solution`, no singular `information`, and no `invalidated_by`.

Example from the database:

```toml
[advisory]
id = "RUSTSEC-2022-0051"
package = "lz4-sys"
date = "2022-08-25"
url = "https://github.com/lz4/lz4/pull/972"
categories = ["memory-corruption"]
cvss = "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"
keywords = ["integer-overflow", "out-of-bounds"]
related = ["CVE-2021-3520"]
aliases = ["GHSA-9q5j-jm53-v7vr"]

[versions]
patched = [">= 1.9.4"]
```

Source: <https://github.com/rustsec/advisory-db/blob/main/CONTRIBUTING.md>.

#### Filing advisories

Report upstream first, wait for confirmation, use coordinated disclosure, then create `RUSTSEC-0000-0000.md` in `crates/<crate>/` and open a PR. RustSec does **not** handle embargoed vulnerabilities. Rust toolchain vulns go to the Rust Security Policy; malicious crates go to <https://crates.io/policies/security>. RustSec historically accepts advisories if there is no response for roughly two weeks after public disclosure.

Per [RustSec contributing guidance](https://rustsec.org/contributing.html): "Do I need to be owner of a crate to file an advisory? No, anyone can file an advisory against any crate."

#### Informational types

Exact taxonomy (from `rustsec/src/advisory/informational.rs`, `#[non_exhaustive]`):

| Type | Meaning |
|------|---------|
| `Notice` | Security notice, not a vulnerability |
| `Unmaintained` | Crate abandoned |
| `Unsound` | Safe API can cause Undefined Behavior |
| `Other(String)` | Extension escape hatch |

An advisory is informational when `informational = "..."` is set; otherwise it is a vulnerability.

#### Categories

Exact taxonomy (from `rustsec/src/advisory/category.rs`): `code-execution`, `crypto-failure`, `denial-of-service`, `file-disclosure`, `format-injection`, `malicious`, `memory-corruption`, `memory-exposure`, `privilege-escalation`, `thread-safety`, `other`.

`info-leak`, `dos`, and `authentication-bypass` are **not** valid RustSec category strings.

#### Severity

Severity follows the CVSS:3.1 qualitative scale: `None`, `Low`, `Medium`, `High`, `Critical`. `severity_threshold` uses these values. Advisories without a `cvss` field are not filtered by `severity_threshold`.

### Yanked crates

`cargo-audit` checks yanked status via `[yanked]` (`enabled = true`, `update_index = true` by default). CLI: `--no-yanked` disables; `--deny=yanked` forces non-zero exit.

Cargo resolver behavior (source: [resolver.html](https://doc.rust-lang.org/cargo/reference/resolver.html)): yanked releases are **ignored** when building the graph unless they already exist in `Cargo.lock`, or are explicitly requested by `cargo update --precise` ("while not recommended"). Existing lockfiles keep resolving the yanked version; fresh resolves skip it. `cargo yank` does not affect existing lockfiles/downloads.

`cargo build` emitting a "package was yanked" warning is widely observed but **not** documented in the Cargo reference; treat it as de-facto behavior.

### cargo-deny

[cargo-deny](https://github.com/EmbarkStudios/cargo-deny) lints advisories, licenses, banned crates, duplicate versions, and sources. As of 0.19.1 it removed the `rustsec` crate dependency (parses advisories itself) and removed `gix` (shells out to `git`). Latest stable at research time: ~0.19.9. Tool MSRV: `rust-version = "1.88.0"`. Book: <https://embarkstudios.github.io/cargo-deny/>.

```bash
cargo install --locked cargo-deny
cargo deny init [PATH]
```

Commands: `check [advisories|bans|licenses|sources|all]` (singular aliases `ban`, `license` accepted), `fetch`, `init`, `list`.

#### Exit codes

`cargo deny check` returns a bitset (since 0.14.1): advisories = 0x1 (1), bans = 0x2 (2), licenses = 0x4 (4), sources = 0x8 (8). A licenses-only failure exits 4; bans + sources exits 10.

#### GitHub Action

```yaml
- uses: EmbarkStudios/cargo-deny-action@v2
  with:
    command: check
    arguments: --all-features
```

Inputs: `command` (default `check`), `arguments` (default `--all-features`), `manifest-path` (default `./Cargo.toml`), `log-level`, `rust-version`. Source: <https://github.com/EmbarkStudios/cargo-deny-action>.

#### cargo-deny configuration (current v2 schema)

`[graph]`: `targets`, `exclude`, `features`, `all-features`, `no-default-features`, `exclude-dev`, `exclude-unpublished`. No `all-targets` or `tests` key.

`[output]`: only `feature-depth` (Option<u32>, default 1). `format` is a CLI flag, not config.

#### `[advisories]` (v2)

```toml
[advisories]
version = 2
db-urls = ["https://github.com/rustsec/advisory-db"]
db-path = "$CARGO_HOME/advisory-dbs"
yanked = "warn"                        # allow | warn | deny
disable-yank-checking = false
ignore = [
    "RUSTSEC-2020-0071",
    "yanked-crate@0.1.0",
    { id = "RUSTSEC-2021-0139", reason = "transitive via clap 2.x, upgrade tracked" },
    { crate = "ansi_term", reason = "unmaintained transitive, upgrade in progress" },
]
unmaintained = "all"                   # all | workspace | transitive | none
unsound = "workspace"                  # all | workspace | transitive | none (added 0.19.0)
git-fetch-with-cli = false
maximum-db-staleness = "P90D"
unused-ignored-advisory = "warn"       # allow | warn | deny
```

Removed keys that now error: `severity-threshold`, `notice`, `vulnerability`. cargo-deny no longer has `informational-warnings` or `severity-threshold`. Use `cargo audit` for severity filtering.

#### `[licenses]` (v2)

Under v2 **all licenses are denied unless explicitly allowed**. The old `copyleft = "deny"` pattern no longer works.

```toml
[licenses]
version = 2
confidence-threshold = 0.8             # 0.0–1.0
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib"]

[[licenses.exceptions]]
allow = ["MPL-2.0"]
crate = "webpki-roots"                 # key is crate, not name

[[licenses.clarify]]
crate = "ring"
expression = "ISC AND OpenSSL"
license-files = [{ path = "LICENSE", hash = 0x0123abcd }]

[licenses.private]
ignore = false
```

Removed keys that now error: `deny`, `copyleft`, `allow-osi-fsf-free`, `default`, `unlicensed`, `deprecated`.

License detection precedence: (1) `[[licenses.clarify]]`, (2) Cargo.toml `license`, (3) `license-file` + LICENSE files joined with `AND`. Detection uses SPDX expressions + `askalono` confidence scoring. As of 0.18.4 GNU licenses are pedantic (`GPL-2.0` ≠ `GPL-2.0-only`).

#### `[bans]` (v2)

Entries use `crate = "name@version"` PackageSpec syntax, not the old `{ name = "...", version = "..." }` object.

```toml
[bans]
multiple-versions = "warn"             # allow | warn | deny
multiple-versions-include-dev = false
wildcards = "allow"                    # allow | warn | deny
allow-wildcard-paths = true
highlight = "all"                      # all | lowest-version | simplest-path
workspace-default-features = "allow"   # allow | warn | deny
external-default-features = "allow"
allow = ["openssl@0.10", { crate = "internal-crate", reason = "internal mirror" }]
allow-workspace = false                # added 0.19.0

[[bans.deny]]
crate = "openssl"
reason = "Use rustls instead — avoid C dependency and CVE surface"

[[bans.deny]]
crate = "time@0.1"
reason = "RUSTSEC-2020-0071: segfault via localtime_r; use time 0.3.x"
wrappers = ["chrono"]                  # wrappers lives INSIDE the deny entry

[[bans.skip]]
crate = "winapi@0.2"
reason = "Transitive dep via old windows-sys; tracked in ISSUE-5678"

[[bans.skip-tree]]
crate = "windows-sys@0.42"
depth = 3
reason = "Old windows-sys subtree; pinned for MSRV compatibility"
```

`skip` ignores just that crate's duplicate; `skip-tree` ignores the crate and up to `N` levels of its subtree (`depth = 0` = skip only the named crate; default = infinite). Sub-tables: `[bans.workspace-dependencies]`, `[bans.build]`, `[[bans.features]]`, `[[bans.build.bypass]]`. Full schema: <https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html>.

#### `[sources]` (v2)

There is no `replace-source` key in `deny.toml`. Source replacement is Cargo's `.cargo/config.toml` feature.

```toml
[sources]
unknown-registry = "warn"              # allow | warn | deny
unknown-git = "warn"
required-git-spec = "any"              # any | branch | tag | rev — set "rev" to force commit-pinned git deps
allow-registry = [
    "https://github.com/rust-lang/crates.io-index",
    "sparse+https://index.crates.io/",
]
allow-git = [
    "https://github.com/your-org/internal-crate",
    { url = "https://github.com/your-org/other", reason = "internal mirror" },
]
private = []
unused-allowed-source = "warn"
unused-allowed-org = "warn"            # added 0.19.9

[sources.allow-org]
github = ["your-org"]
gitlab = []
bitbucket = []
```

Registries are identified by index URL. crates.io git index: `https://github.com/rust-lang/crates.io-index`; sparse index (Cargo ≥1.68 default): `sparse+https://index.crates.io/`.

#### cargo-deny does NOT check MSRV

cargo-deny has no dependency MSRV lint. "msrv" issues in its tracker refer to cargo-deny's own build MSRV.

For MSRV enforcement use: `rust-version`, `resolver.incompatible-rust-versions = "fallback"` (resolver v3, edition 2024 default, Rust 1.84+), and `cargo +<msrv> check` in CI. Optionally use [`cargo-hack`](https://github.com/taiki-e/cargo-hack) or `cargo-msrv`. See [`cargo-dependencies.md`](cargo-dependencies.md) and [`editions-tooling.md`](editions-tooling.md).

### cargo-vet

[cargo-vet](https://github.com/mozilla/cargo-vet) is upstream-prevention: every new dependency must be audited or exempted. Canonical repo is `mozilla/cargo-vet`; `rust-secure-code/cargo-vet` does not exist. Book: <https://mozilla.github.io/cargo-vet/>.

```bash
cargo install --locked cargo-vet
```

Commands: `cargo vet` (= `cargo vet check`, default), `cargo vet init`, `cargo vet certify`, `cargo vet suggest`, `cargo vet import <name>`, `cargo vet trust <pkg> <publisher>`, `cargo vet aggregate <SOURCES>`, plus `inspect`, `diff`, `add-exemption`, `record-violation`, `regenerate`, `prune`, `renew`, `explain-audit`, `dump-graph`, `gc`, `fmt`.

#### File layout

`cargo vet init` creates `supply-chain/` next to `Cargo.lock`:

- `audits.toml` — criteria, audits (`crate → version`), trusted (`publisher → trust`). **Importable** by other projects.
- `config.toml` — `default-criteria`, `imports`, `policy`, `exemptions`. **Not** importable.
- `imports.lock` — auto-generated snapshot of imported audits; treat as implementation detail; `--locked` skips the fetch.

Onboarding is instant: `init` auto-populates `[exemptions]` with current deps so the first `cargo vet check` passes.

#### Trust model

Every audit needs `--who`, `--criteria` (`safe-to-run`, `safe-to-deploy`, or custom), and `--start` date. Trust/wildcard entries need `--end` date (max one year in the future, must be renewed). Wildcards bind to a crates.io username or trusted publisher signature like `github:org/repo`.

#### Shared-audit registry

Registry at <https://raw.githubusercontent.com/mozilla/cargo-vet/main/registry.toml>. Current participants include mozilla, google, embark-studios (EmbarkStudios/rust-ecosystem), bytecode-alliance (wasmtime), isrg (divviup/libprio-rs), zcash, fermyon (spin), actix, ariel-os. Imports are **non-transitive**: add each trusted org explicitly.

#### How cargo-vet fits in

| Tool | Mode | When it helps |
|------|------|---------------|
| cargo-audit / cargo-deny | Downstream detection | Compare lockfile against known-vuln DB |
| cargo-vet | Upstream prevention | Every new dep must be reviewed/exempted |
| cargo-auditable | Build-time instrumentation | Embed dependency data into release binaries |

The [cargo-audit README](https://github.com/rustsec/rustsec/tree/main/cargo-audit) recommends cargo-vet for supply-chain attack prevention.

### cargo-auditable

[cargo-auditable](https://github.com/rust-secure-code/cargo-auditable) embeds the dependency tree as JSON (crate names + versions; URLs/paths redacted), Zlib-compressed, into linker section `.dep-v0`. Size overhead is <4 KiB even for 400+ deps. No timestamps → reproducible builds preserved.

```bash
cargo install cargo-auditable cargo-audit
cargo auditable build --release
cargo audit bin target/release/<binary>
```

`cargo auditable` passes all args to `cargo` as-is.

Supported formats: Linux ELF, Windows PE, macOS Mach-O, WebAssembly (since v0.6.3; readable via `wasm-tools metadata show` v1.227.0+).

cargo-audit reads embedded data via `cargo audit bin` (since cargo-audit v0.17.3). Other readers: `rust-audit-info`, `auditable2cdx` (→CycloneDX), `syft` v1.15.0+, `trivy` v0.31.0+, `grype` v0.83.0+, `osv-scanner` v2.0.1+, `blint` v2.1.3+.

IMPORTANT: cargo-auditable does **not** protect against supply-chain attacks. Quote: "Does this protect against supply chain attacks? No. Use cargo-vet or cargo-crev for that." It is defense-in-depth for post-build auditing. Used by Microsoft internally; also adopted by Alpine, NixOS, openSUSE, Void, Chimera, Wolfi, Ubuntu 26.04, and Chainguard's `rust` image.

### SBOM tooling

#### cargo-cyclonedx

Canonical repo: <https://github.com/CycloneDX/cyclonedx-rust-cargo> (not `CycloneDX/cargo-cyclonedx`, which 404s).

```bash
cargo install cargo-cyclonedx
cargo cyclonedx
```

Emits OWASP CycloneDX SBOM. WARNING: "cargo-cyclonedx calls into Cargo internally... Cargo may run arbitrary code when invoked on an untrusted project, so cargo-cyclonedx should not be called on untrusted projects either."

#### cargo metadata

`cargo metadata --format-version 1` produces a JSON dump of the whole graph. Security-relevant fields: `packages[].license`, `license_file`, `repository` (claimed), `source` (actual origin), `targets[].kind` (`custom-build`, `proc-macro`), `rust_version`, `links`, `publish`, plus `dependencies[].req`/`source`/`registry`.

#### Cargo native SBOM

`-Z sbom` is unstable (tracking <https://github.com/rust-lang/cargo/issues/13709>, RFC 3553). Do not rely on it until stabilized. `cargo build --build-plan` was removed in 1.93.0-nightly; do not recommend it.

#### Decision matrix

| Need | Tool | Notes |
|------|------|-------|
| Vuln-scan release binaries without `Cargo.lock` | cargo-auditable + cargo audit bin | Minimal size overhead |
| Standards-compliant SBOM | cargo-cyclonedx | CycloneDX; container attestations, vendor compliance |
| Future native format | `-Z sbom` | Wait for stabilization |
| SPDX | No first-class Rust generator | Convert CycloneDX → SPDX via syft |

### cargo-supply-chain

[cargo-supply-chain](https://github.com/rust-secure-code/cargo-supply-chain) surfaces the human side of trust: who publishes each crate on crates.io.

```bash
cargo install cargo-supply-chain
cargo supply-chain publishers          # list crates.io publishers in dep graph
cargo supply-chain crates              # per-crate view
cargo supply-chain json                # machine-readable output
cargo supply-chain update              # fetch latest daily dump from crates.io
```

Use it to answer "how many distinct publishers are in my transitive tree?" and "is this critical crate published by an individual or a known org?". It complements content-side checks from cargo-audit and cargo-auditable.

### Rust Secure Code Working Group

The [Rust Secure Code WG](https://github.com/rust-secure-code/wg) is the official Rust WG with mission "make it easy to write secure code in Rust." Coordination in `#wg-secure-code` Zulip; curated ecosystem at `rust-secure-code/projects`. WG-stewarded tools: cargo-audit (via RustSec org), cargo-geiger, cargo-supply-chain, safety-dance. Related in other orgs: cargo-deny (Embark), cargo-vet (Mozilla), cargo-crev, Miri, cargo-fuzz, Rudra, Prusti, MIRAI.

### Supply-chain attack surface and incidents

#### Code-execution vectors

Three ordinary Cargo mechanisms execute arbitrary code during a build:

1. **Build scripts (`build.rs`).** Per [Cargo build-scripts.html](https://doc.rust-lang.org/cargo/reference/build-scripts.html): "Placing a file named build.rs in the root of a package will cause Cargo to compile that script and execute it just before building the package."
2. **Proc-macro crates.** `crate-type = ["proc-macro"]` runs arbitrary code inside `rustc` at compile time.
3. **Git dependency submodules.** Git deps are fetched with submodules recursively; a compromised submodule becomes build input.

Enumerate code-execution deps with `cargo metadata --format-version 1` and inspect `packages[].targets[].kind` for `custom-build` and `proc-macro`.

#### Known incidents

- **2023-08-16 typosquatting attack.** Nine crates typosquatting popular crates with malicious `build.rs` exfiltrating OS/IP/geolocation to Telegram; yanked immediately; removed from crates.io file store 2023-08-18. Postmortem: <https://blog.rust-lang.org/inside-rust/2023/09/01/crates-io-malware-postmortem/>.
- **2022-05-10 malicious crate `rustdecimal`.** Typosquatting `rust_decimal` containing malware. Report: <https://blog.rust-lang.org/2022/05/10/malicious-crate-rustdecimal/>.
- **2018-10-15 crates.io incident.** Malicious crate uploaded and removed.
- **2025-04-11 session-cookie leakage.** crates.io session cookies inadvertently sent to Sentry; sessions invalidated. Report: <https://blog.rust-lang.org/2025/04/11/crates-io-security-session-cookies/>.
- Recurring 2025–2026 "crates.io: Malicious crates ..." blog series.

Do **not** assert a "September 2024 crates.io incident" as fact. The only September-2024 artifact is the opening of RFC 3691 (trusted publishing), which is policy design, not an attack report.

#### Trusted publishing

crates.io adopted trusted publishing via [RFC 3691](https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html), opened 2024-09-10. Tracking issue: <https://github.com/rust-lang/crates.io/issues/10247> (opened 2024-12-19, closed 2025-08-07). Uses GitHub Actions OIDC to mint short-lived crates.io tokens (<1 hour), replacing long-lived API tokens. API tokens default to 90-day expiry since 2024-09-12. First provider: GitHub Actions; GitLab/CircleCI future work. Mitigates token-theft-driven malicious publishes.

#### Typosquatting defense

`package.repository` is a manifest claim; Cargo does not verify it. `cargo metadata` `source` is the actual origin (`registry+URL`, `git+URL?rev=...#<sha>`, `sparse+URL`). Compare them manually or with a known-good maintainer mapping. `cargo deny check sources` enforces a registry/git-remote allowlist, catching accidental untrusted registries/unannounced git deps, but is **not** a per-crate identity or typosquat validator.

### cargo-deny vs cargo-audit

| Capability | cargo-audit | cargo-deny |
|------------|-------------|------------|
| RustSec advisories | Yes | Yes (`check advisories`) |
| License policy | No | Yes |
| Duplicate-version detection | No | Yes |
| Crate banning / feature gating | No | Yes |
| Source allowlisting | No | Yes |
| Binary scanning | Yes (`cargo audit bin`) | No |
| Auto-remediation | Yes (`cargo audit fix`) | No |

Both consult the same [RustSec advisory-db](https://github.com/rustsec/advisory-db). They are complementary: run `cargo deny check` in CI, and `cargo audit bin` against release artifacts. The Rust Secure Code WG ecosystem recommends both.

### Resolver v1 vs v2 vs v3 (supply-chain angle)

For full mechanics see [`cargo-dependencies.md`](cargo-dependencies.md) and [resolver.html](https://doc.rust-lang.org/cargo/reference/resolver.html).

- **Resolver v1** unifies features globally across all targets/kinds. Build-dep/dev-dep features leak into normal builds, expanding the compiled attack surface with unaudited code.
- **Resolver v2** (edition 2021 default; Rust 1.51+) isolates features per kind: target-specific only when built; build-deps/proc-macros isolated from normal deps; dev-dep features only for tests/examples/benches.
- **Resolver v3** (edition 2024 default; Rust 1.84+) same feature semantics as v2, but changes `resolver.incompatible-rust-versions` default from `"allow"` to `"fallback"` (MSRV-aware selection).

MSRV-fallback can select older dependency versions that may carry unfixed CVEs. Recommendation: use resolver v2 or v3; it is both a correctness and supply-chain improvement.

### cargo tree / cargo metadata for supply-chain review

Useful `cargo tree` flags: `-d` (duplicates), `-i <pkg>` (reverse tree), `-e features` (what enables what), `-f "{p} {l} {r}"` (package/license/repository), `--target all` (platform-only deps), `--no-dedupe` (full graph).

`cargo metadata --format-version 1` security-relevant fields:

| Field | Why it matters |
|-------|----------------|
| `packages[].license` / `license_file` | License compliance |
| `packages[].repository` | Claimed origin (unverified) |
| `packages[].source` | Actual origin (registry+/git+/sparse+) |
| `packages[].targets[].kind` | `custom-build` / `proc-macro` = code execution |
| `packages[].rust_version` | MSRV |
| `packages[].links` | Native library binding |
| `packages[].publish` | Whether publishable |
| `dependencies[].req` / `source` / `registry` | Version requirement and source |

Use `--filter-platform`, `--no-deps`, `--all-features` as needed.

### Dependency pinning (supply-chain angle)

For the full SemVer/pinning table see [`cargo-dependencies.md`](cargo-dependencies.md) and [specifying-dependencies.html](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html).

- Caret (`^`) is default and recommended for most deps.
- `0.x.y` and `0.0.x` have special caret semantics (rightmost non-zero component governs compatibility).
- Pin exact `=x.y.z` for reproducible/release artifacts, security/crypto crates (`ring`, `rustls`, `aws-lc-rs`), forked/patched deps, and crates with SemVer-violation history.
- For git deps, `rev = "<sha>"` is the strongest guarantee (immutable). `branch = "main"` moves on `cargo update`; `tag` durability depends on upstream policy. Enforce commit-pinned git deps via `[sources].required-git-spec = "rev"`.
- Targeted rollback: `cargo update -p <crate> --precise <version>` (can re-select a yanked version, "while not recommended").
- Vendoring: `cargo vendor` vendors crates.io + git deps into a directory and prints source-replacement TOML. Each vendored crate has `.cargo-checksum.json`.

IMPORTANT caveat from [source-replacement.html](https://doc.rust-lang.org/cargo/reference/source-replacement.html): "It is not a security mechanism and does not protect against malicious changes." Pair vendoring with provenance tracking (advisory-db commit, `Cargo.lock` hash).

## Practical rules

1. **Run `cargo deny check` in CI on every PR.** It covers advisories, licenses, bans, and sources. Also run `cargo audit` if you need binary scanning or `cargo audit fix`.
2. **Run a scheduled audit at least weekly.** Fresh advisories land continuously; PR-only checks miss new advisories on unchanged lockfiles.
3. **Commit `Cargo.lock` for all binaries.** See [`cargo-dependencies.md`](cargo-dependencies.md).
4. **Never ignore an advisory without a documented reason and tracking issue.** Every entry in `audit.toml` or `deny.toml` `[advisories].ignore` must have a comment and remediation plan.
5. **Treat `unmaintained` and `unsound` advisories as blockers for new code.** Existing code may retain an exception; new code must not introduce one.
6. **Use cargo-deny v2 license allow-list semantics.** Under v2 all licenses are denied unless listed in `[licenses].allow`. Do not use `copyleft = "deny"` or `[licenses].deny`.
7. **Deny wildcard version requirements.** Set `[bans].wildcards = "deny"`.
8. **Restrict sources to crates.io by default.** Set `[sources].unknown-registry = "deny"` and `[sources].unknown-git = "deny"`. Allow specific git sources only with documented reasons and `required-git-spec = "rev"`.
9. **Pin git dependencies to a commit hash.** Never use bare `branch = "main"` in production.
10. **Re-audit after every `cargo update`.** New transitive deps or version bumps can introduce vulnerabilities.
11. **Set `[bans].multiple-versions` to `"warn"` at minimum.** Escalate to `"deny"` once the tree is clean.
12. **Never vendor dependencies without provenance tracking.** Record the advisory-db commit hash and `Cargo.lock` hash.
13. **Pin exact versions for cryptographic/security-sensitive crates.** Use `=x.y.z` for `ring`, `rustls`, `aws-lc-rs`, etc.
14. **Review every advisory before ignoring it.** Read the full advisory at <https://rustsec.org/advisories/>.
15. **Generate an SBOM for every release artifact.** Prefer `cargo-cyclonedx`; add `cargo auditable` for post-build binary scanning.
16. **Adopt cargo-vet for high-assurance codebases.** Require audits or exemptions for every new third-party dependency.
17. **Use resolver v2 or v3.** It isolates build-dep/proc-macro/dev-dep features and reduces compiled attack surface.
18. **Inspect `cargo metadata` for code-execution deps.** Identify `custom-build` and `proc-macro` targets in your transitive tree.

## Review checklist

1. Does the PR add, remove, or update any dependency in `Cargo.toml` or `Cargo.lock`?
2. Does `cargo deny check` pass across all four check categories?
3. Does `cargo audit` pass with no new advisories?
4. Are any new advisories being ignored? If so, is there a documented reason and tracking issue?
5. Are any new git dependencies introduced? If so, are they pinned to a specific commit?
6. Do new dependencies have acceptable licenses per `[licenses].allow`?
7. Does the PR introduce duplicate crate versions? Check `cargo deny check bans`.
8. Are any wildcard version requirements (`version = "*"`) present?
9. If `Cargo.lock` changed, was `cargo deny check` / `cargo audit` re-run?
10. Are any new `build.rs` or proc-macro dependencies introduced without justification?
11. If vendoring changed, are the `Cargo.lock` hash and advisory-db commit recorded?
12. Does the PR modify `deny.toml`? If so, is the change justified and documented?
13. Are any new features being enabled on existing dependencies that could expand the attack surface?
14. Is `[sources].required-git-spec = "rev"` set if git deps are allowed?
15. Are ignored advisories using object-form with a `reason` in `deny.toml`?

## Implementation checklist

1. Install tooling:
   ```bash
   cargo install --locked cargo-audit cargo-deny cargo-vet cargo-auditable cargo-supply-chain
   ```
2. Initialize cargo-deny:
   ```bash
   cargo deny init
   ```
3. Configure `deny.toml`:
   - `[advisories]` with `version = 2`, `ignore = []`, `unmaintained`, `unsound`.
   - `[licenses]` with `version = 2`, explicit `allow` list, `confidence-threshold`.
   - `[bans]` with `multiple-versions`, `wildcards = "deny"`, crate-specific denies.
   - `[sources]` with `unknown-registry = "deny"`, `unknown-git = "deny"`, `allow-registry`, and `required-git-spec = "rev"` if git deps are permitted.
4. Initialize cargo-vet (high-assurance projects):
   ```bash
   cargo vet init
   ```
5. Add `cargo deny check` to PR CI.
6. Add `cargo audit` to PR CI if binary scanning or `cargo audit fix` is desired.
7. Add a scheduled CI job (nightly or weekly) for `cargo deny check` and `cargo audit`.
8. Add pre-release steps: `cargo deny check`, `cargo audit`, `cargo auditable build --release`, `cargo audit bin target/release/<binary>`, `cargo cyclonedx`.
9. Document the advisory-ignore procedure in `CONTRIBUTING.md` or `SECURITY.md`.
10. Pin exact versions for security-sensitive crates in `Cargo.toml`.
11. Record the advisory-db commit hash and `Cargo.lock` hash for provenance.
12. Verify MSRV with `cargo +<msrv> check` (cargo-deny does not check MSRV).
13. Run `cargo tree -d` after dependency changes.
14. Run `cargo supply-chain publishers` periodically to review publisher diversity.

## Validation hooks

```bash
# Audit / SBOM / lint
cargo audit
cargo audit --format json
cargo auditable build --release
cargo audit bin target/release/<binary>
cargo deny check
cargo deny check advisories
cargo deny check licenses
cargo deny check bans
cargo deny check sources

# Rollback (can re-select yanked versions; use carefully)
cargo update -p <crate> --precise <version>

# SBOM / metadata / tree
cargo install cargo-cyclonedx
cargo cyclonedx
cargo metadata --format-version 1
cargo tree -d
cargo tree -i <crate>
cargo tree -e features

# Vendoring / publishers / vet
cargo vendor ./vendor > .cargo/config.toml
cargo supply-chain publishers
cargo vet
```

## Examples

### 1. `deny.toml` using current cargo-deny v2 schema

```toml
# deny.toml — supply-chain policy for this workspace

[graph]
all-features = true
targets = [
    { triple = "x86_64-unknown-linux-gnu" },
    { triple = "aarch64-unknown-linux-gnu" },
    { triple = "x86_64-pc-windows-msvc" },
]

[output]
feature-depth = 1

[advisories]
version = 2
yanked = "warn"
unmaintained = "all"
unsound = "workspace"
ignore = [
    { id = "RUSTSEC-2020-0071", reason = "transitive via chrono, upgrade tracked" },
]

[licenses]
version = 2
confidence-threshold = 0.8
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib"]

[[licenses.exceptions]]
allow = ["MPL-2.0"]
crate = "webpki-roots"
reason = "MPL-2.0 is file-level copyleft; acceptable for non-modified use"

[[licenses.clarify]]
crate = "ring"
expression = "ISC AND OpenSSL"
license-files = [{ path = "LICENSE", hash = 0x0123abcd }]

[bans]
multiple-versions = "warn"
wildcards = "deny"
allow-wildcard-paths = true
highlight = "all"

[[bans.deny]]
crate = "openssl"
reason = "Use rustls instead — avoid C dependency and CVE surface"

[[bans.deny]]
crate = "time@0.1"
reason = "RUSTSEC-2020-0071: segfault via localtime_r; use time 0.3.x"
wrappers = ["chrono"]

[[bans.skip]]
crate = "winapi@0.2"
reason = "Transitive dep via old windows-sys; tracked in ISSUE-5678"

[[bans.skip-tree]]
crate = "windows-sys@0.42"
depth = 3
reason = "Old windows-sys subtree; pinned for MSRV compatibility"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
required-git-spec = "rev"
allow-registry = [
    "https://github.com/rust-lang/crates.io-index",
    "sparse+https://index.crates.io/",
]

[sources.allow-org]
github = ["your-org"]
```

### 2. CI workflow combining cargo-deny and cargo-audit

```yaml
name: supply-chain
on:
  pull_request:
  push:
    branches: [main]
  schedule:
    - cron: '0 4 * * 1'

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install --locked cargo-audit cargo-deny
      - run: cargo deny check
      - run: cargo audit --deny=warnings
```

Also usable via `EmbarkStudios/cargo-deny-action@v2` with `command: check` and `arguments: --all-features`.

### 3. cargo-vet workflow

```bash
cargo vet init
cargo vet certify anyhow --version 1.0.86 \
  --who "Jane Doe <jane@example.com>" \
  --criteria safe-to-deploy \
  --start 2026-01-01
cargo vet suggest
cargo vet import mozilla
```

### 4. cargo-auditable binary scanning

```bash
cargo auditable build --release
cargo audit bin target/release/my-app
```

### 5. cargo-supply-chain publisher review

```bash
cargo supply-chain update
cargo supply-chain publishers
cargo supply-chain json > publishers.json
```

### 6. Enumerating code-execution deps with cargo metadata

```bash
cargo metadata --format-version 1 | \
  jq '.packages[] | select(.targets[].kind | contains(["custom-build"])) | .name' | \
  sort -u

cargo metadata --format-version 1 | \
  jq '.packages[] | select(.targets[].kind | contains(["proc-macro"])) | .name' | \
  sort -u
```

## Common mistakes

### 1. Using removed cargo-deny v1 keys in a v2 config

Removed keys that now error:

- `[advisories]`: `severity-threshold`, `notice`, `vulnerability`.
- `[licenses]`: `deny`, `copyleft`, `allow-osi-fsf-free`, `default`, `unlicensed`, `deprecated`.
- `[bans]`: `{ name = "...", version = "..." }` object form. Use `crate = "name@version"`.
- `[sources]`: `replace-source`. Source replacement belongs in `.cargo/config.toml`.

Do not write:

```toml
# WRONG — removed in v2
[licenses]
copyleft = "deny"
deny = ["GPL-3.0"]

[[bans.deny]]
name = "openssl"
version = "*"
```

Write instead:

```toml
# CORRECT — v2
[licenses]
version = 2
allow = ["MIT", "Apache-2.0"]

[[bans.deny]]
crate = "openssl"
reason = "Use rustls instead"
```

### 2. Running `cargo audit fetch`

There is no `cargo audit fetch` subcommand. Use `cargo audit` (fetch is automatic) or suppress fetching with `--no-fetch`/`-n` or `[database] fetch = false`.

### 3. Silencing low-severity advisories globally

Setting `severity_threshold = "medium"` in `audit.toml` hides low-severity findings. Prefer per-advisory `ignore` entries with documented reasons. cargo-deny v2 has no `severity-threshold` key.

### 4. Using bare branches for git dependencies

```toml
# BAD
[dependencies]
my-crate = { git = "https://github.com/org/crate", branch = "main" }

# GOOD
[dependencies]
my-crate = { git = "https://github.com/org/crate", rev = "a1b2c3d4" }
```

Also set `[sources].required-git-spec = "rev"` in `deny.toml`.

### 5. Treating vendoring as a security boundary

Vendoring makes builds hermetic and offline-capable, but does not prove the vendored code is trustworthy. Record the advisory-db commit and `Cargo.lock` hash, and still run cargo-deny/cargo-audit.

### 6. Assuming cargo-deny enforces MSRV

cargo-deny has no dependency MSRV lint. Use `rust-version`, resolver v3 fallback, and `cargo +<msrv> check` in CI. See [`editions-tooling.md`](editions-tooling.md).

### 7. Ignoring unmaintained/unsound advisories as "just informational"

`unmaintained` means the crate is abandoned and may accumulate unpatched vulnerabilities. `unsound` means safe API can cause UB. Both are real risks and should be blockers for new dependencies.

### 8. Not re-running audit after `cargo update`

A `cargo update` can pull in new transitive versions that introduce vulnerabilities. Always re-run `cargo deny check` and `cargo audit` after updating `Cargo.lock`.

### 9. Pinning exact versions on rapidly-evolving UI crates

Exact pins (`=x.y.z`) on UI/web frameworks block patch fixes and create maintenance burden. Reserve exact pins for security-sensitive and cryptographic crates; use caret elsewhere.

### 10. Confusing `package.repository` with actual source

`package.repository` is a manifest claim. The actual source is `cargo metadata` `source`. A typosquat can set `repository` to a legitimate-looking URL while the crate is published from a different account. Use `cargo supply-chain` to inspect publishers and `cargo deny check sources` to enforce source allowlists.

## Strict vs contextual guidance

### Strict guidance

- Run `cargo deny check` in CI on every PR. No exceptions.
- Run `cargo audit` on every PR if you need binary scanning or `cargo audit fix`.
- Commit `Cargo.lock` for all binary crates.
- Never vendor dependencies without provenance tracking (advisory-db commit, `Cargo.lock` hash).
- Review every advisory before adding it to an ignore list. Read the full advisory at <https://rustsec.org/advisories/>.
- Treat `unmaintained` and `unsound` advisories as blockers for new code.
- Deny wildcard version requirements (`[bans].wildcards = "deny"`).
- Pin git dependencies to a commit hash, never a bare branch.
- Use cargo-deny v2 schema; do not use removed v1 keys.
- Use resolver v2 or v3 to isolate build-dep/proc-macro/dev-dep features.

### Common convention

- Run `cargo deny check` as a single CI gate covering all four check categories.
- Use `audit.toml` or `deny.toml` `[advisories].ignore` with a reason comment and tracking issue per entry.
- Use an allow-list of permissive licenses (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib).
- Set `[bans].multiple-versions = "warn"` at minimum; escalate to `"deny"` once the tree is clean.
- Run a scheduled audit in addition to PR-level checks.
- Use `cargo auditable build --release` and `cargo audit bin` for release artifacts.
- Use `cargo-cyclonedx` for release SBOMs.

### Contextual tradeoffs

- **cargo-deny vs cargo-audit.** cargo-deny is the better single CI gate (advisories + licenses + bans + sources). cargo-audit is required for `cargo audit bin` and `cargo audit fix`. Use both.
- **Copyleft policy.** A library distributed as source may accept MPL-2.0; an embedded binary shipped as firmware should deny all copyleft.
- **Vendor for hermetic CI vs vendor only for release artifacts.** Vendoring in CI ensures reproducibility but increases repository size. Some teams vendor only for release builds.
- **Exact-pinning for cryptographic/security deps vs flexible-pinning for UI deps.** Pin `=x.y.z` for `ring`, `rustls`, `aws-lc-rs`; use `^x.y.z` for `serde`, `tokio`, `clap`.
- **`[bans].multiple-versions = "deny"` vs `"warn"`.** Strict enforcement prevents bloat but can block PRs when a transitive dep has not updated yet. Start with `"warn"`.
- **`audit.toml` informational scope.** Including `unmaintained` and `unsound` catches real risk but may be noisy. Exclude `notice` only if too noisy.
- **MSRV vs security.** Resolver v3 MSRV-fallback can select older dependency versions that may carry unfixed CVEs. Verify security posture explicitly with `cargo deny check` even when MSRV fallback is enabled.

## Policy decisions for individual repos

Document the following in each repository's `CONTRIBUTING.md` or `SECURITY.md`:

| Decision | Options | Default recommendation |
|----------|---------|------------------------|
| License allow-list | Permissive-only vs copyleft-allowed | Permissive-only (`MIT`, `Apache-2.0`, `BSD-*`, `ISC`, `Zlib`) |
| Advisory ignore approval | Individual vs team review | Team review with tracking issue |
| Source allow-list | crates.io only vs +git | crates.io only; git requires `rev` pin |
| Vendoring | Never / CI-only / always | CI-only with provenance tracking |
| MSRV enforcement | None / CI matrix / cargo-hack | `rust-version` + resolver v3 fallback + `cargo +<msrv> check`; cargo-deny does **not** check MSRV |
| Git dependency policy | Forbidden / pinned-commit only / allowed | Pinned-commit only with `[sources].allow-git` allowlist |
| Duplicate versions | Warn / deny | Warn, escalate to deny |
| SBOM generation | None / CycloneDX / SPDX | CycloneDX via `cargo-cyclonedx` for releases |
| cargo-vet adoption | Off / advisory / required | Required for high-assurance codebases |
| Trusted publishing | Off / GitHub Actions OIDC / other | Adopt GitHub Actions OIDC when available |

## Related docs

- [`cargo-dependencies.md`](cargo-dependencies.md) — Version pinning, dependency sources, features, resolver versions, workspaces, `cargo vendor`, `[patch]`, and `[source]` replacement.
- [`unsafe-security.md`](unsafe-security.md) — Most memory-corruption advisories involve `unsafe`; unsafe usage policy and review.
- [`lints-clippy.md`](lints-clippy.md) — The `[lints]` table in `Cargo.toml` is part of broader quality and supply-chain policy.
- [`editions-tooling.md`](editions-tooling.md) — MSRV, edition selection, and resolver configuration. MSRV enforcement belongs there and in CI, **not** in cargo-deny.

## Related skills

No repo-specific skills for this topic.
