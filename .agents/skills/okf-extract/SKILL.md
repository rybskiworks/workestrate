---
name: okf-extract
description: |
  Guide for extracting content from external sources (websites, manuals, source
  repos) into OKF-compliant crawl files and topic documents. Covers the full
  extraction pipeline: identifying source URLs, fetching content (clone repo or
  HTTP fetch), parsing into structured form, creating crawl files with OKF
  frontmatter + metadata headers, mapping crawl files to topic docs via
  provenance tracking, and creating source-map.md. Load when extracting
  documentation from websites, cloning source repos for content, creating crawl
  files under docs/<topic>/.crawl/, or building source-map.md provenance
  indexes. Does NOT cover writing topic docs themselves (that is a separate
  authoring step), skill creation, or general web scraping unrelated to the docs
  corpus.
---

# OKF Content Extraction

## Triggers

Load this skill when:

- Extracting documentation content from external websites (e.g. hexdocs.pm,
  erlang.org, gleam.run) into the `docs/<topic>/.crawl/` corpus.
- Cloning a source repository (e.g. nix.dev, Erlang/OTP source) to extract
  Markdown or documentation sources directly.
- Creating numbered crawl files (`01-source-name.md`, `02-...`) under a
  `docs/<topic>/.crawl/` directory.
- Creating or updating a `docs/<topic>/source-map.md` provenance index.
- Converting fetched HTML pages into structured crawl-file Markdown.
- Using `sitemap.xml` to discover all pages on a documentation site before
  crawling.
- Mapping crawled sources to the topic docs they feed (`feeds_docs` field).

Do NOT load this skill for:

- Writing the topic docs themselves (that is a separate authoring step that
  consumes crawl files as input).
- General web scraping unrelated to the docs corpus.
- Skill creation or agent configuration.

## References

- OKF spec: `.okf-analysis-repo/okf/SPEC.md` — defines frontmatter, concept
  documents, citations, and conformance rules.
- Reference agent web ingestion prompt:
  `.okf-analysis-repo/okf/src/reference_agent/prompts/web_ingestion_instruction.md`
  — extraction patterns, augmentation rules, citation discipline.
- Reference agent fetcher:
  `.okf-analysis-repo/okf/src/reference_agent/web/fetcher.py` — HTML→Markdown
  via `markdownify`, link extraction, 40 KB truncation, 10 s timeout.
- Existing crawl corpora (format examples):
  - `docs/gleam/.crawl/01-documentation.md` through `37-factory-supervisor.md`
  - `docs/elixir/.crawl/E01-genserver.md` through `E09-agent.md`
  - `docs/beam/.crawl/01-design-principles.md` through `47-secure-coding.md`
- Source map examples:
  - `docs/gleam/source-map.md`
  - `docs/elixir/source-map.md`

## Extraction process

The extraction pipeline has six steps. Execute them in order.

### Step 1 — Identify source URL(s) and scope

Before fetching anything, define the crawl scope:

1. **Seed URLs.** List the entry-point URLs (depth-0 seeds). These are the
   pages you start from. Typically 1–3 seeds per documentation family.
2. **Allowed hosts.** Derive from the seed URL hosts. Only fetch pages on
   these hosts unless explicitly expanding scope.
3. **Max-pages budget.** Set a hard cap on total pages to fetch. The
   reference fetcher enforces this; typical budgets are 20–50 pages for a
   full corpus.
4. **Max depth.** Set the hop-depth cap from seeds. Depth 2–3 is typical;
   deeper crawls risk drift into tangential material.
5. **Family classification.** Group seeds into documentation families
   (e.g. "Elixir official guides", "hexdocs gleam_stdlib", "Erlang/OTP
   system docs"). Families drive the source-map structure.

Record the scope decisions; they will appear in the source-map.

### Step 2 — Fetch content

Two primary strategies, depending on source type:

**Strategy A: Clone a repo and extract Markdown directly.**

Use when the documentation source is a Git repository containing Markdown
files (e.g. nix.dev, Erlang/OTP `system/doc/`). This is the highest-fidelity
strategy because the source is already structured Markdown — no HTML
conversion loss.

```
git clone --depth 1 <repo-url> /tmp/<repo-name>
```

Then locate the documentation Markdown files within the repo and read them
directly. Each Markdown file becomes a crawl file. Preserve the source file
path in the crawl file's metadata header.

**Strategy B: Fetch HTML pages and convert to Markdown.**

Use when the documentation is served as HTML (e.g. hexdocs.pm, erlang.org,
gleam.run). The reference fetcher (`fetcher.py`) does:

1. HTTP GET with `User-Agent: okf-reference-agent/0.1` and a 10 s timeout.
2. Reject non-HTML content types.
3. Extract `<title>`.
4. Extract all `href` links (defragmented, absolute, deduplicated).
5. Convert HTML body to Markdown via `markdownify` (ATX headings).
6. Truncate to 40 KB (`_MAX_MARKDOWN_BYTES`).

When fetching manually (curl, wget, or a script), replicate this pipeline:
fetch HTML → convert to Markdown → extract links. Use `markdownify` or
`pandoc -f html -t markdown` for conversion. Record the HTTP status, any
redirects, and the final (canonical) URL.

**Strategy C: Use sitemap.xml to discover all pages.**

Fetch `https://<host>/sitemap.xml` to enumerate all pages on a documentation
site. This is useful for comprehensive coverage when the site has no obvious
index page. Parse the XML, extract `<loc>` URLs, filter to documentation
pages (skip blog posts, marketing, etc.), then fetch each via Strategy B.

### Step 3 — Parse content into structured form

For each fetched page (or cloned Markdown file), extract structured content
into these sections. These sections become the body of the crawl file:

- **Purpose** — One paragraph: what this page is, what it covers, its role in
  the documentation family. State whether it is an index/hub, a reference
  page, a guide, or a module API doc.
- **Key concepts** — Bullet list of the core concepts, terms, and entities
  the page defines. Use the source's own terminology.
- **Strict rules / invariants** — Bullet list of non-negotiable rules stated
  on the page (e.g. "init/1 is the only required callback", "atoms are
  never garbage-collected"). These are the load-bearing constraints.
- **Examples** — Code examples from the page, preserved verbatim in fenced
  blocks. Include the lesson each example teaches.
- **Verbatim quotes** — Numbered list of exact quotes from the page that
  capture authoritative definitions or rules. Each quote must be
  word-for-word from the source. Prefix with the section/heading where the
  quote appears.
- **Version notes** — Version of the tool/language/library the page
  documents, ExDoc version, copyright year, deprecation callouts, and any
  version-specific behavior. If no version is stated, say so explicitly.
- **Discovered links** — Two sub-sections:
  - **Relevant (crawl later)** — Links worth following, with a one-line
    reason and which topic doc they will feed.
  - **Skipped** — Links intentionally not followed, with a one-line reason
    (nav, footer, marketing, duplicate, out of scope, etc.).

### Step 4 — Create crawl files

Each fetched source becomes one crawl file under
`docs/<topic>/.crawl/`. Crawl files ARE OKF concept documents — they must
have OKF-compliant YAML frontmatter.

#### Crawl file naming

Numbered, zero-padded, matching the existing corpus convention:

- `docs/gleam/.crawl/` — `01-documentation.md`, `02-tour-everything.md`, ...
- `docs/elixir/.crawl/` — `E01-genserver.md`, `E02-supervisor.md`, ...
  (Elixir uses an `E` prefix to distinguish from the beam corpus.)
- `docs/beam/.crawl/` — `01-design-principles.md`, `02-gen-server-concepts.md`, ...

Use a short, descriptive slug derived from the page title or URL path. The
number reflects crawl order (seeds first, then discovered pages in
link-following order).

#### Crawl file format

A crawl file has three parts: OKF frontmatter, a metadata header, and a
structured body.

```markdown
---
type: Crawl Source
title: <Short display name>
description: <One-sentence summary of what this source covers>
resource: <Canonical source URL>
tags: [<topic>, crawl, <family-slug>]
timestamp: <ISO 8601 datetime of fetch>
---

# Crawl: <short-name>
- seed_url: <URL used to initiate the fetch>
- canonical_url: <Final URL after redirects, or same as seed_url>
- family: <Documentation family name>
- fetch: <HTTP status + redirect note, e.g. "200" or "HTTP 200 (no redirect)">
- <topic>_version: <Version string from the page, or "not stated on page">
- feeds_docs: <comma-separated list of topic docs this source feeds>

## Purpose
<One paragraph>

## Key concepts
- <bullet>
- <bullet>

## Strict rules / invariants
- <bullet>

## Examples
```<lang>
<verbatim code>
```

## Verbatim quotes
1. "<exact quote>" — <section/heading where it appears>
2. "<exact quote>" — <section/heading>

## Version notes
- <version details>

## Discovered links

### Relevant (crawl later)
- <URL> — <one-line reason> (feeds <topic-doc>.md)

### Skipped
- <URL> — <one-line reason> (SKIP)
```

#### Frontmatter field rules

- `type` — Always `Crawl Source`. This identifies the document as a crawl
  artifact in the OKF bundle.
- `title` — Short display name (e.g. "Gleam documentation hub", "GenServer
  module reference").
- `description` — One sentence summarizing what the source covers.
- `resource` — The **canonical** source URL (after redirects). This is the
  OKF `resource` field — it uniquely identifies the underlying asset.
- `tags` — Always include the topic name (e.g. `gleam`, `elixir`, `beam`),
  the literal `crawl`, and a family slug (e.g. `hexdocs`, `official-docs`).
- `timestamp` — ISO 8601 datetime of when the fetch occurred.

#### Metadata header field rules

The metadata header is a bullet list immediately after the `# Crawl:` H1
heading. It is NOT YAML — it is a human-readable ledger. Required fields:

- `seed_url` — The URL you used to initiate the fetch. May differ from
  canonical_url if redirects occurred.
- `canonical_url` — The final URL after all redirects. If no redirects,
  same as seed_url. Record this even when it equals seed_url.
- `family` — The documentation family this source belongs to (e.g.
  "Elixir core module (hexdocs)", "Erlang/OTP system docs", "Gleam
  official docs").
- `fetch` — HTTP status code and redirect note. Examples: `200`,
  `HTTP 200 (no redirect)`, `301 → <canonical>`.
- `<topic>_version` — The version of the tool/language/library stated on
  the page. Use the topic name as the key prefix (e.g. `elixir_version`,
  `otp_version`, `gleam_version`). If not stated, write
  `not stated on page`.
- `feeds_docs` — Comma-separated list of topic doc filenames (without
  path) that this crawl source feeds. This is the provenance link from
  crawl file → topic doc. Example: `otp-supervision.md, beam-otp-internals.md`.

### Step 5 — Map crawl files to topic docs (provenance tracking)

The `feeds_docs` field in each crawl file's metadata header is the primary
provenance link. It records which topic docs draw from this source.

When writing or updating topic docs:

1. Each topic doc must have a `## Sources used` section listing the crawl
   files and URLs it draws from.
2. Each crawl file's `feeds_docs` must list every topic doc that uses it.
3. These two links must be bidirectional and kept in sync.

### Step 6 — Create/update source-map.md

After all crawl files are created, create (or update)
`docs/<topic>/source-map.md`. This is the authoritative provenance index for
the entire corpus.

#### Source-map structure

```markdown
# <Topic> Source Map

## Purpose

<One paragraph: what this document is, its audience, and that it is the
provenance layer between generated docs and official sources.>

## Source coverage summary

<One paragraph: total count, number of families, all official primary.>

| Family | Count | Authority |
|---|---|---|
| <family 1> | <n> | Official primary |
| <family 2> | <n> | Official primary |
| **Total** | **<N>** | |

## Link expansion coverage

<Describe the crawl depth structure: depth-0 seeds, depth-1 discovery,
depth-2 discovery, etc. Name which families had the richest
cross-referencing.>

## Sources by topic

Legend:

- **Origin:** seed = depth-0 entry point; discovered = reached by following
  links at depth 1-N.
- **Authority:** official primary = the tool's own documentation site.
- **Coverage status:** fully explored = crawled to completion and
  substantially used.

### 1. <Family name> (<n> pages)

| # | URL | Canonical URL if redirected | Origin | Family | Topics extracted | Generated docs that use it | Skills referencing | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 01 | <url> | <canonical or -> | seed | <family> | <topics> | <docs> | <skills> | <workflows> | Official primary | Fully explored |

### 2. <Family name> (<n> pages)
...

## Redirects

<List any seed URLs that redirected or 404'd, with the canonical
replacement and which crawl file recorded it.>

## Discovered but not crawled (revisit later)

<List pages discovered as cross-references but not crawled, grouped by
family, with the reason each was deferred.>

## Skipped

<List categories of source intentionally excluded: playgrounds, blogs,
GitHub mirrors, vendor homepages, etc.>

## Verification method

<One paragraph: how all pages were fetched (curl, HTTP 200), where the
extracted content is persisted (docs/<topic>/.crawl/01-N), and that no
claim in the topic docs relies on memory or non-crawled sources.>
```

#### Source-map field rules

- The per-family tables have columns: `#`, `URL`, `Canonical URL if
  redirected`, `Origin`, `Family`, `Topics extracted`, `Generated docs that
  use it`, `Skills referencing`, `Workflows influenced`, `Authority`,
  `Coverage status`.
- `#` matches the crawl file number (e.g. `01` → `01-documentation.md`).
- `Origin` is `seed` for depth-0 entry points, `discovered` for pages
  reached by following links.
- `Coverage status` is `Fully explored` (crawled to completion and
  substantially used) or `Partially explored` (specific sections used).
- The `Redirects` section records every seed URL that redirected or 404'd,
  with the canonical replacement.
- The `Discovered but not crawled` section is the backlog for future
  expansion passes.

## OKF compliance for crawl files

Crawl files ARE OKF concept documents. Per OKF v0.1 (SPEC.md §9), a
conformant bundle requires:

1. Every non-reserved `.md` file contains a parseable YAML frontmatter
   block.
2. Every frontmatter block contains a non-empty `type` field.

Therefore every crawl file MUST have:

- YAML frontmatter delimited by `---` at the top of the file.
- A `type` field set to `Crawl Source`.
- The `resource` field set to the canonical source URL.

The metadata header (bullet list after `# Crawl:`) is an extension beyond
the OKF minimum — it carries crawl-specific provenance that OKF's generic
frontmatter does not cover. OKF permits producer-defined additional
frontmatter keys (SPEC.md §4.1), but the metadata header is kept as a
human-readable block in the body rather than frontmatter to preserve
compatibility with the existing corpus convention.

## Extraction strategies (detailed)

### Clone a repo (e.g. nix.dev, Erlang/OTP source)

When the documentation source is a Git repo with Markdown files:

1. `git clone --depth 1 <repo-url> /tmp/<repo-name>`
2. Locate documentation directories (e.g. `system/doc/`, `src/`, `content/`).
3. For each Markdown file, read it directly — no HTML conversion needed.
4. Create a crawl file per source Markdown file, preserving the content
   verbatim.
5. Set `seed_url` to the repo URL + relative path (e.g.
   `https://github.com/erlang/otp/blob/OTP-29.0.2/system/doc/design_principles/design_principles.md`).
6. Set `canonical_url` to the same.
7. Set `fetch` to `cloned from <repo-url>@<tag-or-sha>`.

This strategy preserves maximum fidelity because the source is already
structured Markdown. Prefer it whenever the documentation is available as
Markdown in a repo.

### Fetch HTML pages and convert to Markdown

When the documentation is served as HTML:

1. Fetch the page via HTTP (curl, wget, or the reference fetcher).
2. Record HTTP status, redirects, and final URL.
3. Convert HTML → Markdown using `markdownify` (Python), `pandoc -f html -t
   markdown`, or equivalent.
4. Extract outbound links from `href` attributes (defragment, absolutize,
   deduplicate).
5. Truncate to 40 KB if the page is very large (matching the reference
   fetcher's `_MAX_MARKDOWN_BYTES`).
6. Parse the Markdown into the structured sections (Purpose, Key concepts,
   etc.).

### Use sitemap.xml to discover all pages

When you need comprehensive coverage of a documentation site:

1. Fetch `https://<host>/sitemap.xml`.
2. Parse the XML and extract all `<loc>` URLs.
3. Filter to documentation pages (skip `/blog/`, `/news/`, `/feed.xml`,
  `/search`, etc.).
4. Sort by URL path for deterministic crawl order.
5. Fetch each URL via the HTML strategy above.

This is useful when the site has no single index page that links to
everything, or when you want to guarantee no documentation page is missed.

## Quality checks

After creating crawl files, verify:

1. **Content fidelity** — Every verbatim quote is word-for-word from the
   source. Spot-check 3–5 quotes against the original page. If a quote
   does not match, fix it.
2. **Missing pages** — Cross-reference the discovered links in each crawl
   file against the actual crawl files created. Any "Relevant (crawl
   later)" link that was never crawled should either be crawled or moved
   to the source-map's "Discovered but not crawled" section.
3. **Citation preservation** — All source URLs cited in crawl files must
   be URLs that were actually fetched. Do not invent URLs. Do not cite a
   URL you did not fetch (per the reference agent's style rules).
4. **Metadata completeness** — Every crawl file has all required
   frontmatter fields (`type`, `title`, `description`, `resource`,
   `tags`, `timestamp`) and all required metadata header fields
   (`seed_url`, `canonical_url`, `family`, `fetch`, `<topic>_version`,
   `feeds_docs`).
5. **feeds_docs sync** — Every topic doc listed in a crawl file's
   `feeds_docs` must have a corresponding `## Sources used` entry pointing
   back. Bidirectional provenance must be consistent.
6. **Redirect recording** — Every redirect or 404 encountered during the
   crawl is recorded in the source-map's `## Redirects` section with the
   canonical replacement.
7. **OKF conformance** — Every crawl file has parseable YAML frontmatter
   with a non-empty `type` field (OKF v0.1 §9).

## Anti-patterns

- **Do NOT paraphrase source content in crawl files.** Crawl files preserve
  verbatim content — exact quotes, exact code, exact terminology. The
  topic docs (a separate authoring step) synthesize and paraphrase; the
  crawl files are the raw evidence layer. Paraphrasing in crawl files
  destroys provenance.
- **Do NOT skip the metadata header.** The bullet-list metadata header
  (seed_url, canonical_url, family, fetch, version, feeds_docs) is
  required on every crawl file. It is the provenance ledger. Omitting it
  breaks the source-map and makes provenance untraceable.
- **Do NOT break links.** Preserve all outbound links from the source page
  in the "Discovered links" section, classified as Relevant or Skipped.
  Do not silently drop links — even skipped links must be recorded with a
  reason.
- **Do NOT invent URLs.** Only cite URLs you actually fetched. The
  reference agent's style rules are explicit: "Cite only URLs you
  actually fetched. Do not invent URLs."
- **Do NOT skip the OKF frontmatter.** Crawl files without frontmatter are
  non-conformant OKF bundles. The `type: Crawl Source` field is required.
- **Do NOT merge multiple sources into one crawl file.** One source page =
  one crawl file. Merging destroys the per-source provenance granularity.
- **Do NOT fetch beyond the allowed hosts or max-depth.** The reference
  fetcher enforces host allow-lists and hop-depth caps. Respect these
  boundaries to avoid crawl drift.
- **Do NOT replace the body wholesale when augmenting.** If updating an
  existing crawl file with new content from a re-fetch, preserve every
  existing `#` heading in order. Extend, do not replace (per the reference
  agent's augmentation rules).
