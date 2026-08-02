---
name: rust-async-tokio
description: |
  Operational reference for writing, reviewing, and debugging async/await Rust
  code on the Tokio runtime. Load when implementing async functions, spawning
  tasks, choosing channels, handling cancellation, diagnosing `Send`/`'static`
  errors, blocking-in-async issues, or graceful shutdown. Distilled from
  docs/rust/async-tokio.md — cite that doc, not upstream URLs.
---

# Async and Tokio in Rust

Compact operational guide. Full theory, examples, and citations live in
`docs/rust/async-tokio.md` (relative to repo root) — read it before non-trivial
async work.

## Triggers

Load this skill when:

- Writing/reviewing `async fn`, `.await`, `tokio::spawn`, `tokio::select!`.
- Choosing a Tokio runtime flavor or building a runtime by hand.
- Picking a channel (`oneshot`/`mpsc`/`broadcast`/`watch`).
- Implementing cancellation, shutdown, or timeouts.
- Debugging `Send`/`'static` errors from `tokio::spawn`.
- Diagnosing scheduler starvation, deadlocks, lost wakeups, `JoinError`.

## Mental Model (3 axioms)

1. **Futures are lazy.** Calling `async fn` builds a future; nothing runs
   until `.await` or the runtime polls it. An un-awaited future does nothing.
2. **`.await` is the only suspension point.** Code between `.await`s runs
   synchronously and can starve the worker.
3. **`Send + 'static` for `tokio::spawn`.** Non-`Send` values held across
   `.await` in a spawned task are a compile error — scope them in a block
   that ends before the `.await`.

## Runtime Flavor

| Need | Flavor | Feature |
|---|---|---|
| Networked/I/O server, multi-core | `multi_thread` (work-stealing) | `rt-multi-thread` |
| Tests, sims, lightweight CLI, `!Send` tasks | `current_thread` | `rt` |
| `!Send` tasks via `spawn_local` | `local` (`Builder::build_local`) | `rt` |

- `#[tokio::main]` defaults to `multi_thread`; override with
  `#[tokio::main(flavor = "current_thread")]` or `worker_threads = N`.
- Hand-built runtime: call `.enable_all()` (drivers off by default); shut
  down with `rt.shutdown_timeout(...)`.
- `async fn main` body is a non-worker future; only `tokio::spawn` tasks
  get work-stealing.

## Spawning & JoinHandle

- `tokio::spawn(fut)` → `JoinHandle<T>`; future must be `Send + 'static`.
- Dropping `JoinHandle` = detach (keeps running, result lost).
- `abort()` cancels at next `.await`; cannot abort a running `spawn_blocking`.
- `JoinHandle.await` → `Result<T, JoinError>`: `is_cancelled()` / `is_panic()`;
  `try_into_panic()` + `resume_unwind` to re-raise. Inspect in production,
  don't blind `.unwrap()`.
- `spawn` / `Handle::block_on` outside a runtime context → panic.

## Concurrency Macros

| Macro | Behavior | Short-circuits? |
|---|---|---|
| `tokio::join!` | All branches, concurrent (same task, not parallel) | No |
| `tokio::try_join!` | All branches, returns `Result<(..), E>` | On first `Err` |
| `tokio::select!` | First ready branch; others dropped (cancelled) | First ready |

- `biased;` → fixed top-to-bottom order (use for shutdown-first fairness).
- Anti-pattern: serial `.await` when you meant concurrent — create futures
  first, then `join!`.
- `select!` in a `loop` requires **cancellation-safe** branch futures.

## Blocking Rules (strict)

Never on an async worker: `std::thread::sleep`, blocking I/O (sync files,
blocking DNS, sync DB drivers), long CPU loops.

| Tool | Use for | Notes |
|---|---|---|
| `spawn_blocking` | CPU work, blocking I/O | Dedicated pool (max 512); cannot abort mid-run; shutdown waits for it |
| `block_in_place` | Blocking on `multi_thread` only | Panics on `current_thread`; prefer `spawn_blocking` |
| `yield_now().await` | Break hot loops | No hard real-time guarantee |
| `tokio::time::sleep` | Delays | Never `std::thread::sleep` |

`Handle::block_on` inside an async context panics. Long-running background
workers → dedicated `std::thread`.

## Channel Choice

| Channel | Producers | Consumers | Buffer | Sees all? | Best for |
|---|---|---|---|---|---|
| `oneshot` | 1 | 1 | 1 | yes | request/response, single result |
| `mpsc` bounded | many | 1 | N | rx only | worker queues, backpressure |
| `mpsc` unbounded | many | 1 | ∞ | rx only | non-async callers only |
| `broadcast` | many | many | ring N | yes (if not lagged) | pub/sub fan-out |
| `watch` | 1 | many | 1 latest | latest only | config, shutdown flags |

- Bounded `mpsc`: `send().await` is NOT cancel-safe (drops value on cancel);
  use `reserve().await` → `Permit::send(v)` in `select!` loops.
- `watch`: never hold `Ref` across `.await` (deadlocks the writer).
- Clean `mpsc` shutdown: `Receiver::close()` then drain.

## Cancellation

- `CancellationToken` is in **`tokio_util::sync`** (needs `tokio-util` crate),
  NOT `tokio::sync`. `use tokio::sync::CancellationToken;` won't compile.
- `cancel()` cancels token + all children; `cancelled()` is cancel-safe.
- `child_token()` hierarchical; `clone()` symmetric; `drop_guard()`
  cancel-on-drop; `run_until_cancelled(fut)` helper.
- Prefer `CancellationToken` over ad-hoc `oneshot`/`broadcast` for shutdown.
- No async `Drop` in stable Rust — don't rely on `Drop` for async cleanup.

### Cancellation safety in `loop { select! }`

Safe: `mpsc::Receiver::recv`, `broadcast::Receiver::recv`,
`watch::Receiver::changed`, `TcpListener::accept`, `AsyncReadExt::read`,
`AsyncWriteExt::write`, `StreamExt::next`, `Interval::tick`, `JoinHandle`.

NOT safe (lose progress/data on cancel): `read_exact`, `read_to_end`,
`read_to_string`, `write_all`, `Mutex::lock`, `RwLock::read`/`write`,
`Semaphore::acquire`, `Notify::notified`, `Barrier::wait`.

## Synchronization & Time

- `std::sync::Mutex`/`parking_lot::Mutex` is fine and faster for short,
  non-`.await` sections. **Never** hold its guard across `.await`.
- `tokio::sync::Mutex` only when the guard must cross `.await`. Tokio sync
  primitives do not poison on panic. `*_owned` variants need `Arc<Self>`.
- Prefer message passing over shared mutable state.
- Always `tokio::time`, never `std::thread::sleep` (needs `time` feature +
  active driver; all time futures cancel-safe).
- `timeout` drops the inner future on elapsed; a non-yielding future can
  overrun without returning `Elapsed`.
- `Interval` first tick completes immediately. `MissedTickBehavior`:
  `Burst` (catch up) / `Delay` (min spacing) / `Skip` (anchored).
- `test-util` feature → `time::pause()`/`resume()`/`advance()` for
  deterministic tests; affects `tokio::time::Instant` only.

## Process & Signals

- `tokio::process::Command` works on **both** runtime flavors (needs
  `process` feature + I/O driver). The "requires multi_thread" claim is wrong.
- **Set `kill_on_drop(true)`** when orphaned children are unacceptable; drop
  does not cancel by default.
- `wait().await` is cancel-safe and closes stdin; `.take()` stdin first.
  `kill().await` waits; `start_kill()` does not.
- Containers: handle **both SIGINT and SIGTERM** (Docker/K8s/systemd send
  SIGTERM, then SIGKILL after grace; K8s default 30s).

### Graceful shutdown pattern

1. `select!` on `ctrl_c()` + `signal(unix::SignalKind::terminate())`.
2. `CancellationToken::cancel()` (fan-out to workers).
3. Workers `select!` on work + `token.cancelled()` (use `biased;`).
4. Hard `tokio::time::timeout` fallback.
5. `child.kill().await` for subprocesses.

## Async Traits & Features

- Native `async fn` in traits stable since Rust 1.75; not object-safe
  (`no dyn Trait`); adding `+ Send` later is a breaking change.
- `trait_variant::make` for public `Send` APIs; `#[async_trait]` (boxes
  futures) for `dyn Trait` or pre-1.75 MSRV; `#[async_trait(?Send)]` opts out.
- Apps: `full` is fine. Libraries: enable only what's needed — `rt`,
  `rt-multi-thread`, `macros`, `net`, `time`, `sync`, `process`, `signal`,
  `fs`, `io-util`, `io-std`. `AsyncRead`/`AsyncWrite` need no feature.

## Review Checklist

- [ ] No blocking calls / `std::thread::sleep` in async tasks.
- [ ] `Send + 'static` respected across `.await` in spawned tasks.
- [ ] No `std::sync` guard held across `.await`.
- [ ] `loop { select! }` branches are cancellation-safe.
- [ ] Channel type matches pattern (table above).
- [ ] CPU/blocking work on `spawn_blocking` or dedicated thread.
- [ ] External I/O has `tokio::time::timeout`.
- [ ] Runtime flavor matches workload; drivers enabled on hand-built runtime.
- [ ] Subprocesses set `kill_on_drop(true)` when needed.
- [ ] Server/container handles SIGINT + SIGTERM.
- [ ] `JoinError` inspected (panic vs cancel); library feature flags minimal (`full` only in apps).

## Verification Commands

```bash
cargo check                       # type/Send/pinning errors
cargo clippy -- -D warnings       # suspicious async patterns, unused awaits
cargo test                        # #[tokio::test] (current_thread default)
# Multi-thread: #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
# Deterministic time: tokio::time::pause();  (needs tokio "test-util" feature)
RUSTFLAGS="--cfg tokio_unstable" cargo run   # then attach tokio-console
```

## Related

- `docs/rust/async-tokio.md` — full reference, examples, upstream citations.
- `docs/rust/{error-handling,ownership-lifetimes,testing}.md` — adjacent topics.
- `nix-usage` skill — Rust toolchain / `nix develop` for this repo.
