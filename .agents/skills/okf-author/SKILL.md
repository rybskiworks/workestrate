---
name: okf-author
description: |
  Guide for authoring OKF-compliant (Open Knowledge Format v0.1) documents —
  frontmatter schema, body structure, cross-linking, citations, index/log
  files, naming conventions, and validation. Load when creating or editing OKF
  knowledge bundles, writing concept documents, generating index.md/log.md
  files, or converting existing documentation into OKF format. Does NOT cover
  OKF parser internals, programmatic bundle generation APIs, or non-OKF
  documentation formats.
---

# OKF Author

Operational guide for authoring OKF-compliant (Open Knowledge Format v0.1)
documents. Load when creating or editing concept documents, building knowledge
bundles, generating `index.md`/`log.md` files, or converting existing
documentation into OKF format.

## Triggers

Load this skill when:

- Authoring or editing OKF concept documents (markdown + YAML frontmatter).
- Creating a new OKF knowledge bundle or adding concepts to an existing one.
- Generating `index.md` or `log.md` files for a bundle directory.
- Converting existing documentation (wiki pages, runbooks, data dictionaries) into OKF format.
- Reviewing a bundle for OKF v0.1 conformance.

Do NOT load for: parsing OKF programmatically (use the reference agent's
`document.py`), generating bundles via the `write_concept_doc` tool API, or
authoring non-OKF markdown.

## OKF Format Overview

- OKF (Open Knowledge Format) v0.1 is an open, human- and agent-friendly format for representing knowledge.
- A knowledge bundle is a directory tree of markdown files with YAML frontmatter. It is the unit of distribution.
- A concept is a single markdown document within a bundle. Its concept ID is the file path relative to the bundle root with the `.md` suffix removed (e.g., `tables/users.md` → concept ID `tables/users`).
- No schema registry, no central authority, no required tooling. If you can `cat` a file, you can read OKF.
- Bundles may be distributed as git repos (recommended), tarballs, or subdirectories within larger repos.
- The format is minimally opinionated: it standardizes only the small set of structural conventions needed to make a corpus self-describing.

## Frontmatter Schema

Every concept document begins with a YAML frontmatter block delimited by
`---` on its own line at the start of the file and a closing `---` on its own
line.

### Required fields

There are two levels of "required":

**Spec-minimum (OKF v0.1 conformance, §9):**

- `type` — A short string identifying the kind of concept. Consumers use this for routing, filtering, and presentation. Type values are NOT registered centrally. Producers should pick descriptive, self-explanatory values. Consumers must tolerate unknown types gracefully. Example values: `BigQuery Table`, `BigQuery Dataset`, `API Endpoint`, `Metric`, `Playbook`, `Reference`.

**Tooling-compatible (reference parser/validator enforces these):**

The reference agent's `document.py` defines
`REQUIRED_FRONTMATTER_KEYS = ("type", "title", "description", "timestamp")`.
The `write_concept_doc` tool refuses to write documents missing any of these
four. Always include all four for tooling compatibility.

### Recommended fields (in priority order per spec §4.1)

- `title` — Human-readable display name. If omitted, consumers may derive a title from the filename. (Required by tooling.)
- `description` — A single sentence summarizing the concept. Used by index.md generators, search snippets, and previews. (Required by tooling.)
- `resource` — A URI that uniquely identifies the underlying asset the concept describes. Absent for concepts describing abstract ideas rather than physical resources.
- `tags` — A YAML list of short strings for cross-cutting categorization.
- `timestamp` — ISO 8601 datetime of last meaningful change. (Required by tooling.)

### Preferred key order

The reference tooling (`bundle_tools.py`) reorders frontmatter to this
canonical order before writing:

```
type, resource, title, description, tags, timestamp
```

Always author in this order. Additional producer-defined keys go after
`timestamp`.

### Extensions

Producers may include any additional keys. Consumers should preserve unknown
keys when round-tripping and should not reject documents with unrecognized
fields.

### Example frontmatter (resource-bound concept)

```yaml
---
type: BigQuery Table
resource: https://console.cloud.google.com/bigquery?p=acme&d=sales&t=orders
title: Customer Orders
description: One row per completed customer order across all channels.
tags: [sales, orders, revenue]
timestamp: 2026-05-28T14:30:00Z
---
```

### Example frontmatter (abstract concept, no resource)

```yaml
---
type: Playbook
title: Incident response — data freshness alert
description: Steps to triage a freshness alert on the orders pipeline.
tags: [oncall, incident]
timestamp: 2026-04-12T09:00:00Z
---
```

Note: when `resource` is absent, it is simply omitted — do not set it to null
or empty string.

## Body Structure

The body is standard markdown after the frontmatter. Producers should favor
structural markdown (headings, lists, tables, fenced code blocks) over
freeform prose, since structure aids both human reading and agent retrieval.

There are NO required body sections. The following headings have conventional
meaning and should be used when applicable:

| Heading | Purpose |
|---------|---------|
| `# Schema` | Structured description of an asset's columns/fields. Use a markdown table with columns like Column, Type, Description. |
| `# Examples` | Concrete usage examples, often as fenced code blocks. |
| `# Citations` | External sources backing claims in the body. See Citations section below. |

Other common headings seen in real bundles: `# Overview`, `# Joins`,
`# Metrics`, `# Steps`, `# Trigger`, `# Limitations`, `# Pre-requisites`.
These are free-form; use whatever sections make sense for the concept.

The body may be empty for minimal stub concepts, but a description in
frontmatter is still required by tooling.

## Document Types

OKF does not define a fixed taxonomy. Common `type` values and guidance:

| Type | When to use |
|------|-------------|
| `Reference` | Abstract definitions — metrics, join definitions, enumerated types, data structures. Concepts not bound to a single physical resource. |
| `BigQuery Table` | A concrete BigQuery table. Use `# Schema` for columns. Include `resource` URI. |
| `BigQuery Dataset` | A BigQuery dataset grouping tables. Include `resource` URI. |
| `API Endpoint` | A REST/RPC endpoint. Use `# Schema` for request/response fields. Include `resource` URI. |
| `Metric` | A business or technical metric definition. Often abstract (no `resource`). |
| `Playbook` | Operational runbook / incident response steps. Use `# Steps` or numbered lists. |
| `Tutorial` | Step-by-step learning guide. Use `# Examples` heavily. |
| `Guide` | How-to or conceptual guide. |

Choose descriptive, self-explanatory type names. Consumers tolerate unknown
types by treating them as generic concepts.

## Cross-linking Rules

Concepts may link to other concepts using standard markdown links. Two forms:

### Absolute (bundle-relative) links — RECOMMENDED

Begin with `/`, interpreted relative to the bundle root.

```markdown
See the [customers table](/tables/customers.md) for the join key.
```

This is the recommended form because it is stable when documents are moved
within their subdirectory.

### Relative links

Standard markdown relative paths.

```markdown
See the [neighboring concept](./other.md).
```

Or without `./`:

```markdown
See [Event Count](../references/metrics/event_count.md).
```

Relative links are common in real bundles (e.g., the ga4 bundle uses
`../references/metrics/event_count.md`).

### Link semantics

A link from concept A to concept B asserts a relationship. The specific kind
(parent/child, references, joins-with, depends-on) is conveyed by surrounding
prose, not the link itself. Consumers treat all links as directed edges of an
untyped relationship.

Consumers MUST tolerate broken links — a link whose target does not exist is
not malformed; it may represent not-yet-written knowledge.

## Citation Format

When a concept's body makes claims sourced from external material, list those
sources under a `# Citations` heading at the bottom of the document.

Spec format (numbered):

```markdown
# Citations

[1] [BigQuery public dataset announcement](https://cloud.google.com/blog/products/data-analytics/...)
[2] [Internal data quality runbook](https://wiki.acme.internal/data/quality)
```

Citation links may be:

- Absolute URLs
- Bundle-relative paths
- Paths into a `references/` subdirectory that mirrors external material as first-class OKF concepts

Note: real bundles (e.g., ga4) sometimes use bare URLs without the
`[n] [Title](url)` wrapper:

```markdown
# Citations
- https://developers.google.com/analytics/bigquery/web-ecommerce-demo-dataset
```

Both forms are acceptable. The numbered `[n] [Title](url)` form is preferred
for readability and traceability.

## Index File Generation

An `index.md` file may appear in any directory, including the bundle root. It
enumerates the directory's contents to support progressive disclosure —
letting a human or agent see what is available before opening individual
documents.

### Rules

- Index files contain NO frontmatter (exception: the bundle-root `index.md` may include `okf_version: "0.1"` to declare the target OKF version).
- The body uses one or more sections, each grouping concepts under a heading.
- Entries should include the description from the linked concept's frontmatter.

### Format

```markdown
# <Type or Group Heading>

* [Title 1](relative-url-1) - short description of item 1
* [Title 2](relative-url-2) - short description of item 2

# Another Section

* [Subdirectory](subdir/) - short description of the subdirectory
```

### Index patterns seen in real bundles

Root index groups by subdirectory:

```markdown
# Subdirectories

* [datasets](datasets/index.md) - Description of the datasets directory.
* [tables](tables/index.md) - Description of the tables directory.
* [references](references/index.md) - Specifications for joins and metric definitions.
```

Subdirectory index groups by type:

```markdown
# BigQuery Table

* [Events table](events_.md) - Contains Google Analytics event export data.
```

```markdown
# Reference

* [Event Count](event_count.md) - Total number of events.
* [User Count](user_count.md) - Total number of unique users.
```

Producers may generate `index.md` automatically; consumers may synthesize one
on the fly when none is present.

## Log File Format

A `log.md` file may appear at any level of the hierarchy to record the history
of changes to that scope.

### Rules

- Format is a flat list of date-grouped entries, newest first.
- Date headings MUST use ISO 8601 `YYYY-MM-DD` form.
- Log entries are prose; the leading bold word is a convention, not a requirement.

### Format

```markdown
# Directory Update Log

## 2026-05-22
* **Update**: Added new BigQuery table reference for [Customer Metrics](/tables/customer-metrics.md).
* **Creation**: Established the [Dataplex Playbook](/playbooks/dataplex.md).

## 2026-05-15
* **Initialization**: Created foundational directory structure.
* **Update**: Added progressive-disclosure guidelines to the root [index](/index.md).
```

Common entry types: `**Update**`, `**Creation**`, `**Deprecation**`,
`**Initialization**`.

## Naming Conventions

### Concept ID rules

- Concept ID = file path within the bundle with `.md` suffix removed.
- Each path segment must match the regex: `[A-Za-z0-9_][A-Za-z0-9_.\-]*`
  - Must start with an alphanumeric or underscore.
  - May contain letters, digits, underscores, dots, and hyphens.
  - No spaces, no slashes within a segment.
- Examples of valid concept IDs: `tables/users`, `references/metrics/event_count`, `references/joins/events___ads_clickstats`
- The reference parser (`paths.py`) validates each segment against this regex and raises `ValueError` on invalid segments.

### File naming

- Use lowercase with underscores or hyphens: `event_count.md`, `events_.md`
- Avoid spaces and special characters in filenames.
- Reserved filenames (MUST NOT be used for concept documents): `index.md`, `log.md`

### Directory structure

- Organize concepts into subdirectories however makes sense for the domain.
- Common patterns: `tables/`, `datasets/`, `references/`, `references/metrics/`, `references/joins/`
- Each subdirectory may have its own `index.md` and `log.md`.

### Example bundle layout

```
my_bundle/
├── index.md
├── log.md
├── datasets/
│   ├── index.md
│   └── sales.md
├── tables/
│   ├── index.md
│   ├── orders.md
│   └── customers.md
└── references/
    ├── index.md
    ├── metrics/
    │   ├── index.md
    │   └── event_count.md
    └── joins/
        └── events___ads_clickstats.md
```

## Validation Checklist

Before committing an OKF document, verify:

- [ ] File starts with `---` on its own line (frontmatter delimiter).
- [ ] Frontmatter is valid YAML, terminated by a closing `---` on its own line.
- [ ] Frontmatter contains `type` (spec-required) AND `title`, `description`, `timestamp` (tooling-required).
- [ ] `timestamp` is a valid ISO 8601 datetime (e.g., `2026-05-28T14:30:00Z`).
- [ ] Frontmatter keys are in preferred order: `type, resource, title, description, tags, timestamp` (then extensions).
- [ ] `resource` is present for concrete assets (tables, datasets, endpoints); omitted for abstract concepts.
- [ ] `tags` is a YAML list (inline `[a, b]` or block list with `- item` entries).
- [ ] Filename is not `index.md` or `log.md` (reserved).
- [ ] Each path segment of the concept ID matches `[A-Za-z0-9_][A-Za-z0-9_.\-]*`.
- [ ] Body uses structural markdown (headings, tables, lists, code blocks) over freeform prose.
- [ ] `# Schema` section present when describing an asset with columns/fields.
- [ ] `# Citations` section present when body makes claims from external sources.
- [ ] Cross-links use bundle-relative absolute paths (`/tables/customers.md`) where possible.
- [ ] No broken links that could be easily fixed (broken links are tolerated but should be minimized).
- [ ] If an `index.md` exists in the same directory, it lists this concept.

For `index.md` files:

- [ ] No frontmatter (unless bundle root with `okf_version`).
- [ ] Entries use `* [Title](link) - description` format.
- [ ] Descriptions match the linked concept's frontmatter `description`.

For `log.md` files:

- [ ] Date headings use `YYYY-MM-DD` ISO format.
- [ ] Entries are newest-first.
- [ ] Each entry starts with a bold action word (`**Update**`, `**Creation**`, etc.).

## Anti-patterns

- **Missing frontmatter entirely.** Every non-reserved `.md` file must have a parseable YAML frontmatter block.
- **Missing `type` field.** This is the only spec-required field; without it the bundle is non-conformant.
- **Missing `title`, `description`, or `timestamp`.** The reference tooling refuses to write documents missing these.
- **Wrong key order.** While consumers tolerate any order, the tooling reorders to `type, resource, title, description, tags, timestamp`. Author in this order to avoid surprises.
- **Using reserved filenames for concepts.** `index.md` and `log.md` have defined meaning; do not use them for concept documents.
- **Frontmatter on index.md.** Index files have no frontmatter (except the bundle root, which may declare `okf_version`).
- **Spaces or special characters in concept IDs.** Segments must match `[A-Za-z0-9_][A-Za-z0-9_.\-]*`.
- **Freeform prose instead of structured markdown.** Structure aids both human reading and agent retrieval.
- **Omitting `# Schema` for data assets.** Tables and endpoints should document their fields.
- **Omitting `# Citations` when making sourced claims.** External sources should be traceable.
- **Using `null` or empty string for absent `resource`.** Simply omit the key.
- **Hardcoding non-ISO timestamps.** Use ISO 8601 (e.g., `2026-05-28T14:30:00Z`).
- **Treating broken links as errors.** Consumers must tolerate broken links; they represent not-yet-written knowledge.

## Template

A blank OKF-compliant concept document template is available at:
`templates/concept.md` (relative to this skill's directory).

Copy it as a starting point for new concept documents.

## References

- OKF v0.1 Specification: the normative spec defines conformance (§9), frontmatter (§4.1), body (§4.2), cross-linking (§5), index files (§6), log files (§7), and citations (§8).
- Reference parser: `document.py` — defines `REQUIRED_FRONTMATTER_KEYS` and the `OKFDocument.parse`/`validate`/`serialize` cycle.
- Reference writer: `bundle_tools.py` — defines `_PREFERRED_KEY_ORDER` and the `write_concept_doc` augmentation guards.
- Path validation: `paths.py` — defines the concept ID segment regex and `concept_id_to_path`/`parse_concept_id` functions.
