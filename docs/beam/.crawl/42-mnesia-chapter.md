# Crawl: mnesia/mnesia_chapter.html
- seed_url: https://www.erlang.org/doc/apps/mnesia/mnesia_chapter.html
- canonical_url: https://www.erlang.org/doc/apps/mnesia/mnesia_overview.html
- family: Erlang/OTP mnesia docs
- fetch: 404 (seed URL not found; no redirect). Nearest live chapter = mnesia_overview.html (HTTP 200). Content below extracted from the overview chapter.
- otp_version: OTP 29.0.2 (mnesia 4.26.1)
- feeds_docs: ets-data.md

## Purpose
Mnesia is a multiuser, distributed, fault-tolerant DBMS written in and tightly
coupled to Erlang. It was built because telecommunications (nonstop) systems need
a DBMS that runs in the same address space as the application, with high fault
tolerance and dynamic reconfiguration — requirements traditional DBMSs do not
meet. It targets industrial-grade telecom applications written in Erlang, and
"almost turns Erlang into a database programming language," eliminating the
impedance mismatch between DBMS data format and programming-language data format
(any Erlang term is a valid record/value).

It is designed to meet, in one system, a mix of features usually spread across
DBMSs and telecom data-management systems:
- Fast real-time key/value lookup
- Complex non-real-time queries (mainly O&M)
- Distributed data (because the applications are distributed)
- High fault tolerance
- Dynamic reconfiguration
- Complex objects (arbitrary Erlang records as rows)

## What Mnesia is (transactional, distributed, ETS/DETS-backed)
Mnesia is a transactional, optionally-distributed DBMS layered on top of Erlang's
ETS (in-memory) and DETS (on-disk) table primitives (DETS itself backs onto
`disk_log`). It is not a SQL database and not an external server process; it
lives in the Erlang runtime, in the same address space as the application.

Key properties stated on the overview page:
- The database schema can be dynamically reconfigured at runtime.
- Tables can be declared with properties for location, replication, and persistence.
- Tables can be moved or replicated to several nodes for fault tolerance; other
  nodes can still read/write/delete records while this happens.
- Table locations are transparent to the programmer — programs address table names
  and the system tracks where tables live.
- Transactions can be distributed; multiple operations run in a single transaction.
- Multiple transactions run concurrently and are fully synchronized by Mnesia so no
  two processes manipulate the same data simultaneously.
- Transactions can be set to execute on all nodes or on none.
- Transactions can be bypassed via "dirty operations" that reduce overhead and run fast.

Because it stores Erlang terms directly, Mnesia rows can be arbitrarily complex
records (no schema-mapping layer), which is the "complex objects" feature.

## create_table + storage types (ram_copies/disc_copies/disc_only_copies)
The overview page does not enumerate `mnesia:create_table/1,2` signatures or the
storage-type names; that detail lives in the Getting Started / chapter1 docs
(discovered, not crawled — see links). Decision-level guidance from the overview:

- Tables are created with `mnesia:create_table(Name, Opts)` where `Opts` declare
  per-table properties including storage type, replica nodes, attributes, and
  index. (Stable OTP API; confirm exact options in mnesia_chap1.html.)
- Storage type is a per-replica property controlling persistence vs speed:
  - `ram_copies` — in-memory only (ETS-backed); fastest; lost on node restart.
  - `disc_copies` — in-memory + on-disc replica (ETS + DETS/disk_log); fast reads,
    durable writes, survives restart.
  - `disc_only_copies` — on-disc only (DETS-backed); slower, lower RAM footprint,
    no in-memory copy.
- A single table can have different storage types on different nodes (e.g. ram on
  hot node, disc on backup node), which is how Mnesia trades speed vs durability
  vs fault tolerance per replica.
- The schema itself is a Mnesia table; schema is dynamically reconfigurable at
  runtime (add/move/drop replicas without stopping the system).

NOTE: The storage-type identifiers above are stable, well-established Mnesia facts
but are NOT quoted from this overview page; verify exact option names/signatures
in mnesia_chap1.html before relying on them in code.

## Transactions vs dirty ops
From the overview page (decision guidance):
- Transactions are the consistency model: distributed, multi-operation, fully
  synchronized, mutually exclusive on overlapping data. Use them when several
  records must be updated atomically/safely together.
- Dirty operations bypass transactions: lower overhead, faster, but no
  isolation/atomicity guarantees across multiple ops. Use for hot-path
  single-record reads/writes where you accept the trade-off (e.g. `dirty_read`,
  `dirty_write`, `dirty_delete`).
- Rule of thumb: prefer transactions for correctness when multiple records/nodes
  are involved; use dirty ops only for performance-critical single-record access
  where stale/inconsistent reads are tolerable.

## Replication/distribution + fragmentation
From the overview page:
- Replication is declarative: a table is declared to have replicas on a set of
  nodes; Mnesia keeps them in sync. Replicas improve fault tolerance (a node can
  fail while others keep serving reads/writes/deletes).
- Distribution is transparent: programs address table names, not nodes; Mnesia
  routes to the right replica.
- Transactions can be configured to run on all nodes or on none, giving control
  over consistency vs availability trade-offs.
- Dynamic reconfiguration: tables can be moved or replicated to additional nodes
  at runtime without service disruption.

Fragmentation (splitting a large table across nodes by key range) is a Mnesia
feature but is NOT described on this overview page; it is covered in the deeper
Mnesia chapters (discovered, not crawled). Decision note: fragmentation is the
answer when a single table's size or write load exceeds what one node can hold,
complementing replication (which is about fault tolerance, not scale-out).

## When to use Mnesia vs ETS vs DETS vs external store (decision guidance)
Verbatim decision guidance from the overview ("When to Use Mnesia" section):

Mnesia is a great fit for applications that:
- Need to replicate data.
- Perform complex data queries.
- Need to use atomic transactions to safely update several records simultaneously.
- Require soft real-time characteristics.

Mnesia is not as appropriate for applications that:
- Process plain text or binary data files.
- Merely need a lookup dictionary that can be stored on disc → use `dets`
  (disc-based version of `ets`).
- Need disc logging facilities → use `disk_log`.
- Require hard real-time characteristics.

Decision matrix (synthesized from the above):
| Need | Choose |
|---|---|
| In-memory key/value, single process, no persistence, fastest | ETS |
| On-disc key/value dictionary, single node, no transactions | DETS |
| Append-only disc log / WAL semantics | disk_log |
| Replicated, transactional, distributed, soft-real-time, complex Erlang records | Mnesia |
| Hard real-time deadlines | neither Mnesia nor ETS guarantees; redesign |
| SQL, ad-hoc reporting, huge datasets beyond a few nodes | external SQL/NoSQL store |

## Limitations
- No SQL interface. Querying is via list comprehensions (QLC) or the Mnesia
  match/select API, not SQL. QLC can optimize queries for Mnesia and acts as a
  database programming language, but it is still Erlang-centric.
- Soft real-time only — not suitable for hard real-time.
- Not a bulk file/blob store — use the filesystem or `disk_log` for plain
  text/binary data files.
- Distribution scales to a limited number of nodes; Mnesia's full-mesh
  transaction coordination is not designed for very large clusters (the overview
  does not state a number; this is a known OTP constraint — verify in chap docs).
- Tight Erlang coupling is a feature for Erlang apps but a limitation for
  polyglot systems (no wire protocol for non-Erlang clients beyond external
  wrappers).

## Verbatim quotes
- "Mnesia is a multiuser distributed DBMS specifically designed for
  industrial-grade telecommunications applications written in Erlang, which is
  also the intended target language."
- "It almost turns Erlang into a database programming language, which yields many
  benefits. The foremost is that the impedance mismatch between the data format
  used by the DBMS and the data format used by the programming language ...
  completely disappears."
- "The database schema can be dynamically reconfigured at runtime."
- "Tables can be moved or replicated to several nodes to improve fault tolerance.
  Other nodes in the system can still access the tables to read, write, and delete
  records."
- "Table locations are transparent to the programmer. Programs address table names
  and the system itself keeps track of table locations."
- "Multiple transactions can run concurrently and their execution is fully
  synchronized by Mnesia, ensuring that no two processes manipulate the same data
  simultaneously."
- "Transactions can be bypassed using dirty operations, which reduce overheads
  and run fast."
- "Mnesia is a great fit for applications that: Need to replicate data. Perform
  complex data queries. Need to use atomic transactions to safely update several
  records simultaneously. Require soft real-time characteristics."
- "Mnesia is not as appropriate for applications that: Process plain text or binary
  data files. Merely need a lookup dictionary that can be stored on disc. ...
  Need disc logging facilities. ... Require hard real-time characteristics."

## Version notes
- Page title: "Overview — OTP 29.0.2 (mnesia 4.26.1)".
- Built with ExDoc v0.40.3.
- Copyright © 1996-2026 Ericsson AB.
- Seed URL `mnesia_chapter.html` returned HTTP 404 (no redirect). The live
  overview chapter is `mnesia_overview.html`. The deeper user-guide content
  (Getting Started, transactions, table storage, fragmentation) is split across
  `mnesia_chap1.html` and `mnesia_chap2.html`, which were NOT crawled per the
  one-link constraint.

## Discovered links
### Relevant (crawl later)
- https://www.erlang.org/doc/apps/mnesia/mnesia_chap1.html — "Getting Started":
  create_table, storage types (ram_copies/disc_copies/disc_only_copies), schema,
  transactions. HIGH priority; fills the gaps left by this overview.
- https://www.erlang.org/doc/apps/mnesia/mnesia_chap2.html — deeper Mnesia chapter
  (transactions, dirty ops, fragmentation, replication details).
- https://www.erlang.org/doc/apps/mnesia/mnesia_overview.md — markdown source of
  this overview (mirror).
- https://www.erlang.org/doc/apps/stdlib/dets.html — DETS module ref (comparison
  target for ETS-vs-DETS-vs-Mnesia decision).
- https://www.erlang.org/doc/apps/kernel/disk_log.html — disk_log module ref
  (comparison target for disc-logging use case).
- https://www.erlang.org/doc/apps/stdlib/qlc.html — QLC (Query List Comprehension)
  used as Mnesia's query language.

### Skipped
- https://www.erlang.org/doc/apps/mnesia/llms.txt — site llms index (not a chapter).
- https://www.erlang.org/doc/apps/mnesia/mnesia.epub — epub bundle (binary).
- https://www.erlang.org/doc/man/mnesia.html — full module reference (out of scope:
  task requested the user-guide chapter, not the module ref).
