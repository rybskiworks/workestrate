---
name: constraint-beam-process-isolation
description: |
  Enforces BEAM process-isolation invariants during code execution — no shared
  mutable state, message passing (not shared memory), ETS ownership/heir
  discipline, process-dictionary avoidance, and mailbox hygiene. Load when
  writing or reviewing concurrent code in Erlang, Elixir, or Gleam-on-BEAM. Does
  NOT cover supervision (see constraint-beam-supervision) or exit-signal
  propagation (see constraint-beam-failure).
metadata:
  org.kind: constraint
---

# Constraint: BEAM Process Isolation

This constraint enforces the share-nothing process model: each process has its
own heap and mailbox, communicates by copying messages, and owns its ETS tables.
Violations leak state across processes, lose tables on owner death, or grow
mailboxes unbounded.

## Triggers

Load this skill when:

- Writing or reviewing `spawn`/`spawn_opt`/`send`/`receive` code.
- Creating or transferring ETS tables (`heir`, `give_away`).
- Using the process dictionary (`put`/`get`/`erase`).
- Designing a process that holds shared state.

## Rules

1. Processes share NO mutable state; all communication is via message passing.
   Messages are COPIED from the sender heap into the receiver mailbox (shared
   off-heap binaries pass by reference).
2. Do NOT use the process dictionary for application state — it is invisible to
   `sys` and bypasses behaviours; reserve it for debugging/short-lived scratch.
3. An ETS table dies with its owner unless `heir` is set or ownership is
   transferred via `give_away/3`. Set an `heir` for tables that must survive
   owner restarts.
4. `heir` must be local; `{heir, Pid}` (no data) sends NO transfer message;
   `{heir, Pid, HeirData}` sends `{'ETS-TRANSFER', tid(), FromPid, HeirData}`.
5. `setopts/2` only allows changing `heir`, and only the owner may call it;
   access rights (`public`/`protected`/`private`) are fixed at creation.
6. `ordered_set` uses compare-equal (`==`) keys (so `1` and `1.0` are the same
   key); `set`/`bag`/`duplicate_bag` use match (`=:=`). `'$end_of_table'` must
   never be used as a key.
7. Use `safe_fixtable` for multi-call traversals on `set`/`bag`/`duplicate_bag`
   (not needed for `ordered_set` or single-call traversals); always release it
   (pair `true`/`false` in `try/after`).
8. Signal ordering is per sender→destination pair only; there is NO global
   ordering across senders — never assume cross-sender message order.
9. Do NOT pass both `link` and `monitor` to `spawn_opt` (raises `badarg`); use
   `spawn_link` or `spawn_monitor` for atomic spawn+link/monitor.
10. Keep `receive` selective (matching specific messages) to avoid scanning
    unbounded queues; a growing mailbox indicates a slow consumer or hot loop.
11. Do NOT use raw `spawn` for supervised work — use behaviours or `proc_lib`.

## References

- Operational skill: `beam-processes`.
- Docs: `docs/beam/processes-and-messages.md`, `docs/beam/ets-data.md`.

## Out of scope

- Supervision tree configuration — see `constraint-beam-supervision`.
- Exit-signal propagation and `trap_exit` — see `constraint-beam-failure`.
- NIF scheduler isolation — see `constraint-beam-nif-safety`.

## Violation examples

### Process dictionary for application state

```erlang
%% FORBIDDEN: invisible to sys, bypasses behaviours
put(counter, get(counter) + 1)
```

Correct: hold the state in a `gen_server`/`gen_statem` state argument, or pass
it explicitly.

### ETS table lost when the owner dies

```erlang
%% FORBIDDEN: no heir — table is destroyed when the owner terminates
ets:new(cache, [set, public])
```

Correct: `ets:new(cache, [set, public, {heir, HeirPid, cache_data}])` or transfer
ownership via `give_away/3`.

### Assuming cross-sender message ordering

```erlang
%% FORBIDDEN assumption: messages from different senders have no global order
%% A ! msg1, B ! msg2  -- msg2 may arrive before msg1 at the receiver
```

Correct: rely only on per-sender ordering; sequence/correlate with a reference if
order matters across senders.

### `link` and `monitor` together in `spawn_opt`

```erlang
%% FORBIDDEN: raises badarg
spawn_opt(Fun, [link, monitor])
```

Correct: use `spawn_link` (link) or `spawn_monitor` (monitor); they cannot be
combined.

## How to check

```bash
# Runtime: process_info(Pid, [message_queue_len, heap_size, dictionary, links, monitors])
#          ets:info(Table, [owner, heir, size, protection])
#          erlang:system_info(ets_count)
```

Manual review:

- No process dictionary for application state.
- Long-lived ETS tables have an `heir` or are transferred.
- `safe_fixtable` paired with release in `try/after`.
- `receive` is selective; no unbounded mailbox growth.
- No raw `spawn` for supervised work; no `link`+`monitor` together.
