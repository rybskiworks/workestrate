# Async and Tokio in Rust

## Purpose

This document is a reference for future AI coding agents working on async Rust code with the Tokio runtime. It covers the async/await mental model, the `Future` trait, pinning, wakers, the Tokio runtime, concurrency primitives, cancellation semantics, and the practical rules that keep async code correct and performant. Use it when implementing, reviewing, or refactoring async Rust.

## Sources used

- [The Rust Programming Language, Chapter 17: Futures and Async](https://doc.rust-lang.org/book/ch17-00-async-await.html)
- [The Rust Programming Language, chapter 17: Async and Await](https://doc.rust-lang.org/book/ch17-01-futures-and-syntax.html)
- [The Rust Programming Language: More Futures](https://doc.rust-lang.org/book/ch17-03-more-futures.html)
- [The Rust Programming Language: Traits for Async](https://doc.rust-lang.org/book/ch17-05-traits-for-async.html)
- [Asynchronous Programming in Rust](https://rust-lang.github.io/async-book/)
- [Asynchronous Programming in Rust: Getting Started](https://rust-lang.github.io/async-book/01_getting_started/01_chapter.html)
- [Asynchronous Programming in Rust: Wakeups](https://rust-lang.github.io/async-book/02_execution/03_wakeups.html)
- [Asynchronous Programming in Rust: I/O](https://rust-lang.github.io/async-book/02_execution/05_io.html)
- [Asynchronous Programming in Rust: Async/Await Primer](https://rust-lang.github.io/async-book/03_async_await/01_chapter.html)
- [Asynchronous Programming in Rust: Pinning](https://rust-lang.github.io/async-book/04_pinning/01_chapter.html)
- [Asynchronous Programming in Rust: Executing Multiple Futures at a Time](https://rust-lang.github.io/async-book/06_multiple_futures/01_chapter.html)
- [Asynchronous Programming in Rust: Send Approximation](https://rust-lang.github.io/async-book/07_workarounds/03_send_approximation.html)
- [`std::future::Future`](https://doc.rust-lang.org/std/future/trait.Future.html)
- [`std::pin`](https://doc.rust-lang.org/std/pin/index.html)
- [`std::task::Waker`](https://doc.rust-lang.org/std/task/struct.Waker.html)
- [Tokio crate root](https://docs.rs/tokio/latest/tokio/)
- [Tokio feature flags](https://docs.rs/tokio/latest/tokio/#feature-flags)
- [`#[tokio::main]`](https://docs.rs/tokio/latest/tokio/attr.main.html)
- [`tokio::runtime`](https://docs.rs/tokio/latest/tokio/runtime/)
- [`tokio::runtime::Builder`](https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html)
- [`tokio::task`](https://docs.rs/tokio/latest/tokio/task/)
- [`tokio::task::spawn`](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html) — note that `https://docs.rs/tokio/latest/tokio/macro.spawn.html` is a 404; `spawn` is a function in `tokio::task`, not a macro.
- [`tokio::task::JoinHandle`](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html)
- [`tokio::task::JoinError`](https://docs.rs/tokio/latest/tokio/task/struct.JoinError.html)
- [`tokio::task::spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
- [`tokio::task::block_in_place`](https://docs.rs/tokio/latest/tokio/task/fn.block_in_place.html)
- [`tokio::task::yield_now`](https://docs.rs/tokio/latest/tokio/task/fn.yield_now.html)
- [`tokio::join!`](https://docs.rs/tokio/latest/tokio/macro.join.html)
- [`tokio::try_join!`](https://docs.rs/tokio/latest/tokio/macro.try_join.html)
- [`tokio::select!`](https://docs.rs/tokio/latest/tokio/macro.select.html)
- [`tokio::sync`](https://docs.rs/tokio/latest/tokio/sync/index.html)
- [`tokio::sync::Mutex`](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html)
- [`tokio::sync::RwLock`](https://docs.rs/tokio/latest/tokio/sync/struct.RwLock.html)
- [`tokio::sync::Semaphore`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html)
- [`tokio::sync::Notify`](https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html)
- [`tokio::sync::Barrier`](https://docs.rs/tokio/latest/tokio/sync/struct.Barrier.html)
- [`tokio::sync::OnceCell`](https://docs.rs/tokio/latest/tokio/sync/struct.OnceCell.html)
- [`tokio_util::sync::CancellationToken`](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html)
- [`tokio::time`](https://docs.rs/tokio/latest/tokio/time/)
- [`tokio::process`](https://docs.rs/tokio/latest/tokio/process/)
- [`tokio::signal`](https://docs.rs/tokio/latest/tokio/signal/)
- [`tokio::net`](https://docs.rs/tokio/latest/tokio/net/)
- [Async fn and return-position `impl Trait` in traits](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/)
- [`async-trait`](https://docs.rs/async-trait/latest/async_trait/)

Claims in the sections below are inline-cited to the specific URLs above.

## Core guidance

### The async/await mental model

An `async fn` is syntactic sugar for a function that returns an anonymous `impl Future<Output = T>`. The body is wrapped in an `async` block, so the future is not executed on the call — it is constructed and returned ([The Rust Programming Language: Async and Await](https://doc.rust-lang.org/book/ch17-00-async-await.html), [The Rust Programming Language: Futures and Syntax](https://doc.rust-lang.org/book/ch17-01-futures-and-syntax.html), [Asynchronous Programming in Rust: Async/Await Primer](https://rust-lang.github.io/async-book/03_async_await/01_chapter.html)).

```rust
async fn foo() -> u8 { 5 }

// Is equivalent to:
fn foo() -> impl Future<Output = u8> {
    async { 5 }
}
```

Three axioms govern day-to-day work:

1. **Futures are lazy.** Calling an `async fn` produces a future value; nothing runs until that future is polled, typically by `.await` or by the runtime ([Asynchronous Programming in Rust: Async/Await Primer](https://rust-lang.github.io/async-book/03_async_await/01_chapter.html)).
2. **`.await` is postfix.** `future.await` yields control back to the executor until the future becomes ready.
3. **Each `.await` is a suspension point.** Code between two `.await` expressions runs synchronously, without yielding. Only at `.await` can the runtime suspend this task and run another.

Strict rule: never assume work happens before `.await`. If you construct a future and forget to await it, it does nothing.

### Futures as state machines; the Future trait

The compiler desugars each `async` body into an anonymous `struct` that implements [`std::future::Future`](https://doc.rust-lang.org/std/future/trait.Future.html). The struct has one variant per `.await` point and owns the local variables that are alive across those points ([Asynchronous Programming in Rust](https://rust-lang.github.io/async-book/)).

The trait is:

```rust
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

`Poll<T>` is either `Ready(T)` or `Pending` ([`std::future::Future`](https://doc.rust-lang.org/std/future/trait.Future.html)).

Strict rules when implementing `Future` manually:

- Never poll in a tight spin loop; only re-poll after the waker has been invoked.
- Do not poll a future again after it returns `Ready` — doing so may panic or invoke undefined behavior.
- When you see a future returning `Pending`, you must arrange for `wake()` to be called later.

### Pinning

Compiler-generated futures can be self-referential: a local variable may be borrowed across an `.await`, and the generated state machine holds a pointer to another field of itself. If the future were moved in memory after polling, those internal pointers would dangle. [`Pin<P>`](https://doc.rust-lang.org/std/pin/index.html) is the pointer-wrapper contract that the pointee will not be moved ([Asynchronous Programming in Rust: Pinning](https://rust-lang.github.io/async-book/04_pinning/01_chapter.html), [The Rust Programming Language: Traits for Async](https://doc.rust-lang.org/book/ch17-05-traits-for-async.html)).

- `Unpin` is an auto-trait. Most types are `Unpin`: once pinned, they are still safe to move.
- `!Unpin` means address-sensitive. Examples include async-generated futures and types containing `std::marker::PhantomPinned`.

Everyday pinning helpers:

```rust
// Stack pinning.
tokio::pin!(future);

// Heap pinning.
let future = Box::pin(future);
```

Framing: when you only write `async fn` and `.await`, you almost never write `Pin` yourself. `Pin` surfaces in error messages, when implementing `Future` by hand, or when building async-aware data structures.

Strict rule: never move a `!Unpin` future after it has been polled.

### Wakers and the executor loop

The executor drives the future lifecycle. It creates a [`Waker`](https://doc.rust-lang.org/std/task/struct.Waker.html), wraps it in a `Context`, and passes it to `poll`. If `poll` returns `Pending`, the future stores the waker and invokes `wake()` when it can make progress. The executor then schedules the task for another poll ([Asynchronous Programming in Rust: Wakeups](https://rust-lang.github.io/async-book/02_execution/03_wakeups.html)).

Key details:

- Multiple wakeups may coalesce; a single `wake()` may trigger only one poll.
- The future must update its stored waker on every poll. Storing a stale waker from an old poll is a real bug that causes lost wakeups.
- The reactor sits below the executor and translates OS events (epoll/kqueue/IOCP via mio) into waker notifications ([Asynchronous Programming in Rust: I/O](https://rust-lang.github.io/async-book/02_execution/05_io.html)).

### Runtimes: why async needs one

Rust ships no async runtime in the standard library. `fn main` cannot be `async` because there is no executor available by default. A runtime provides three things: a reactor (I/O/time drivers), a scheduler, and an executor API. Tokio is the dominant runtime in the Rust ecosystem ([The Rust Programming Language: Async and Await](https://doc.rust-lang.org/book/ch17-00-async-await.html), [Asynchronous Programming in Rust: Getting Started](https://rust-lang.github.io/async-book/01_getting_started/01_chapter.html), [The Rust Programming Language: More Futures](https://doc.rust-lang.org/book/ch17-03-more-futures.html)).

Tokio uses cooperative multitasking: each task decides when to yield by `.await`. A task that runs a long blocking stretch between `.await` points starves every other task on the same worker thread. If you must yield in a hot loop, `tokio::task::yield_now().await` is the escape hatch, but it has scheduling overhead — measure before sprinkling it everywhere.

### The `Send` + `'static` requirement and the "Send approximation"

[`tokio::task::spawn`](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html) requires the future and its output to be `Send + 'static`. Holding a non-`Send` value across an `.await` inside a spawned task is a compile error, because the runtime may move the task to another worker thread while it is suspended ([`tokio::task::spawn`](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html), [Asynchronous Programming in Rust: Send Approximation](https://rust-lang.github.io/async-book/07_workarounds/03_send_approximation.html)).

The compiler's `Send` analysis is conservative. You can still use `!Send` values inside an async block as long as they do not cross an `.await`. The standard fix is a narrow block scope:

```rust
tokio::spawn(async move {
    {
        let local = Rc::new(1);
        println!("{}", local);
    } // `local` drops here, before the await.
    tokio::time::sleep(Duration::from_millis(1)).await;
});
```

For shared mutable state across tasks, replace `Rc<RefCell<T>>` with `Arc<Mutex<T>>` or `Arc<tokio::sync::Mutex<T>>` depending on whether the guard crosses `.await`.

## Practical rules

### Choosing a Tokio runtime flavor

Tokio offers three runtime flavors ([`tokio::runtime`](https://docs.rs/tokio/latest/tokio/runtime/), [`tokio::runtime::Builder`](https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html), [`#[tokio::main]`](https://docs.rs/tokio/latest/tokio/attr.main.html)):

- `multi_thread` — work-stealing pool; default for `#[tokio::main]`; requires the `rt-multi-thread` feature.
- `current_thread` — single-threaded scheduler; requires the `rt` feature.
- `local` runtime — built with `Builder::build_local`; supports `spawn_local` for `!Send` tasks.

Default parameters for `multi_thread`:

- `worker_threads` = number of logical CPUs.
- `max_blocking_threads` = 512.
- `thread_stack_size` = 2 MiB.
- `thread_keep_alive` = 10 s.

You can override `worker_threads` via the `#[tokio::main(worker_threads = 4)]` attribute or the `TOKIO_WORKER_THREADS` environment variable.

Heuristics:

- Use `multi_thread` for networked, I/O-bound, or multi-core CPU servers.
- Use `current_thread` for tests, deterministic sims, lightweight CLIs, and when tasks are `!Send`.
- Use a hand-built runtime when you need to disable a driver, control shutdown, or run Tokio inside a non-Tokio thread.

A hand-built runtime has no drivers enabled by default:

```rust
let rt = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .unwrap();
rt.block_on(async { /* ... */ });
```

For sync code that needs to call `tokio::spawn` inside an existing runtime, use `rt.enter()` or `Handle::enter()` to install the runtime context on the current thread. Always shut down cleanly with `rt.shutdown_timeout(...)` when you created the runtime manually.

The `async fn main` body itself runs as a non-worker future; only sub-tasks spawned with `tokio::spawn` get the full work-stealing scheduling.

```text
   +------------------------------------------------------+
   | Do you want work-stealing or multi-thread scheduler? |
   +------------------------------------------------------+
                   | Yes              | No
                   |                  |
                   |                  |
                   v                  |
     +------------------------+       |
     | Multi-threaded Runtime |       |
     +------------------------+       |
                                      |
                                      V
                     +--------------------------------+
                     | Do you execute `!Send` Future? |
                     +--------------------------------+
                           | Yes                 | No
                           |                     |
                           V                     |
                   +---------------+             |
                   | Local Runtime |             |
                   +---------------+             |
                                                 |
                                                 v
                                     +------------------------+
                                     | Current-thread Runtime |
                                     +------------------------+
```

The above decision tree is the canonical runtime-flavor guide from the Tokio docs ([`tokio::runtime`](https://docs.rs/tokio/latest/tokio/runtime/)). It is not exhaustive, but it is the right starting point.

### Spawning tasks

[`tokio::task::spawn`](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html) is a function, not a macro. It returns a [`JoinHandle<T>`](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html). Spawning does not synchronously run the future; the runtime schedules it. Spawned tasks are lightweight green threads, not OS threads, and they are not guaranteed to run to completion — when the runtime shuts down, outstanding tasks are dropped. Calling `spawn` outside a Tokio runtime panics ([`tokio::task`](https://docs.rs/tokio/latest/tokio/task/)).

```rust
let handle = tokio::spawn(async move {
    do_work().await
});
```

Strict rule: the spawned future and its output must be `Send + 'static`.

### JoinHandle and JoinError

[`JoinHandle<T>`](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html) implements `Future<Output = Result<T, JoinError>>`.

- Dropping a `JoinHandle` detaches the task: it keeps running, but its result is lost.
- `abort()` schedules cancellation at the next `.await` inside the task. It is idempotent, but it cannot abort a `spawn_blocking` task once it is running.
- `is_finished()` is `true` only after the task has actually completed.
- `abort_handle()` returns an `AbortHandle` that can abort without owning the `JoinHandle`.

[`JoinError`](https://docs.rs/tokio/latest/tokio/task/struct.JoinError.html):

- `is_cancelled()` for cancellation.
- `is_panic()` for panic.
- `try_into_panic()` recovers the panic payload; you can re-raise it with `std::panic::resume_unwind`.

`&mut JoinHandle<T>` is cancel-safe in `tokio::select!`.

### join!, try_join!, select!

These are macros from [`tokio`](https://docs.rs/tokio/latest/tokio/) ([Asynchronous Programming in Rust: Executing Multiple Futures at a Time](https://rust-lang.github.io/async-book/06_multiple_futures/01_chapter.html)).

- [`tokio::join!`](https://docs.rs/tokio/latest/tokio/macro.join.html) waits for all branches. They run concurrently within the same task, not in parallel on different threads. It returns a tuple. It does not short-circuit on `Err`. Prefix with `biased;` for a fixed top-to-bottom polling order.
- [`tokio::try_join!`](https://docs.rs/tokio/latest/tokio/macro.try_join.html) is like `join!` but returns `Result<(A, B, ..), E>` and short-circuits on the first `Err`. The in-flight futures for the other branches are dropped.
- [`tokio::select!`](https://docs.rs/tokio/latest/tokio/macro.select.html) runs all branches and executes the handler for the first branch that completes. Non-selected branches are dropped, which usually cancels them. Branch order is randomized by default; use `biased;` for fixed order, accepting responsibility for fairness.

The `select!` syntax:

```rust
tokio::select! {
    biased;
    _ = token.cancelled() => { /* shutdown */ }
    result = work_fut => { /* work */ }
    else => { /* none ready */ }
}
```

The classic anti-pattern: serial `.await` when you meant concurrent.

```rust
// Serial: b does not start until a finishes.
let a = fetch_a().await;
let b = fetch_b().await;

// Concurrent: create futures first, then join.
let a = fetch_a();
let b = fetch_b();
let (a, b) = tokio::join!(a, b);
```

A future is **cancellation-safe** if dropping an incomplete instance and recreating it is a no-op. Cancellation safety is required for futures used in `loop { select! }`.

Cancellation-safe recv-style operations: `mpsc::Receiver::recv`, `broadcast::Receiver::recv`, `watch::Receiver::changed`, `TcpListener::accept`, `AsyncReadExt::read`, `AsyncWriteExt::write`, `StreamExt::next`, `Interval::tick`, and awaiting a `JoinHandle`.

NOT cancellation-safe: `AsyncReadExt::read_exact`, `read_to_end`, `read_to_string`, `AsyncWriteExt::write_all`, `Mutex::lock`, `RwLock::read`/`write`, `Semaphore::acquire`, `Notify::notified`, and `Barrier::wait`. Do not use these directly as a `select!` branch in a loop.

### Channels

Tokio's channels are in [`tokio::sync`](https://docs.rs/tokio/latest/tokio/sync/index.html). Match the channel to the communication pattern:

```text
| Type        | Producers      | Consumers       | Buffer     | Sees every value? | Best for                         |
|-------------|----------------|-----------------|------------|-------------------|----------------------------------|
| oneshot     | 1              | 1               | 1 value    | Yes               | request/response, single result  |
| mpsc bounded| many (Clone Tx)| 1               | N          | Only the receiver | worker queues, fan-in, backpressure |
| mpsc unbounded | many       | 1               | unbounded  | Only the receiver | non-async callers, unbounded feed|
| broadcast   | many           | many            | ring N     | Yes (if not lagged)| pub/sub fan-out, event bus       |
| watch       | 1              | many            | 1 latest   | Latest only       | config, state snapshots, shutdown flags |
```

**oneshot** ([`tokio::sync::oneshot`](https://docs.rs/tokio/latest/tokio/sync/oneshot/index.html)):

- One value, one consumer, single use.
- `send` consumes the `Sender`; the type is not `Clone`.
- Receiver drop causes `SendError`; sender drop causes `RecvError`.
- Often a `JoinHandle` can replace a oneshot for task results.

**mpsc** ([`tokio::sync::mpsc`](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html)):

- Multi-producer, single-consumer; clone the `Sender` to share.
- Bounded channels give backpressure: `send().await` waits when full. Capacity must be at least 1.
- Unbounded channels are infinite; use only when callers cannot `.await`.
- Receiver drop drains and drops unread messages. Sender drop makes `recv()` return `None`.
- Clean shutdown: call `Receiver::close()`, then drain to completion.
- Cancellation safety: dropping `send().await` mid-await loses the value. For `select!`-safe sends, use `reserve().await` to obtain a `Permit`, then `Permit::send(value)`.

**broadcast** ([`tokio::sync::broadcast`](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html)):

- Multi-producer, multi-consumer; every receiver sees every value.
- Bounded ring buffer; slow receivers get `RecvError::Lag(n)`.
- `send` returns the subscriber count; `Ok` does not mean every receiver observed the value.

**watch** ([`tokio::sync::watch`](https://docs.rs/tokio/latest/tokio/sync/watch/index.html)):

- Stores only the latest value; requires an initial value.
- `send`/`send_modify`/`send_replace` update; `borrow()` reads without marking seen, `borrow_and_update()` marks seen.
- `changed().await` waits for a value the receiver has not yet seen.
- Do not hold the `Ref` guard across `.await`: it can deadlock.
- A new subscriber sees the current value as already seen.

Heuristics:

- oneshot = request/response.
- mpsc = worker queues / fan-in.
- broadcast = pub/sub fan-out where every consumer needs every event.
- watch = config/state snapshots and latest-value shutdown flags.

### Synchronization: tokio::sync::Mutex vs std::sync::Mutex (and friends)

The decisive heuristic: it is OK and often preferred to use `std::sync::Mutex` (or `parking_lot::Mutex`) in async code for short, non-`.await` critical sections over plain data. Use [`tokio::sync::Mutex`](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html) only when you must hold the guard across an `.await`, for example a shared database connection ([`tokio::sync::Mutex`](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html)). The standard-library mutex is faster because it does not integrate with the async scheduler.

Strict rule: never hold a `std::sync::MutexGuard` across `.await`. It can deadlock the runtime and causes `Send` failures in spawned tasks.

Prefer message passing over shared mutable state.

Other primitives:

- [`tokio::sync::RwLock`](https://docs.rs/tokio/latest/tokio/sync/struct.RwLock.html): fair, write-preferring; use for read-heavy shared data that may cross `.await`.
- [`tokio::sync::Semaphore`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html): counting permits, FIFO. Use `acquire`, `try_acquire`, or `acquire_owned` (the latter needs `Arc<Self>`). `add_permits` enables token-bucket rate limiting. Use for concurrency limits, backpressure, and resource pools.
- [`tokio::sync::Notify`](https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html): condition-variable equivalent; `notify_one`/`notify_waiters`; single-permit coalescing. Use `Notified::enable` to avoid lost wakeups in hand-rolled loops.
- [`tokio::sync::Barrier`](https://docs.rs/tokio/latest/tokio/sync/struct.Barrier.html): rendezvous for N tasks; `is_leader`; reusable; not cancel-safe.
- [`tokio::sync::OnceCell`](https://docs.rs/tokio/latest/tokio/sync/struct.OnceCell.html): async one-time init with `get_or_init`.

Tokio's sync primitives do not poison on panic, unlike `std::sync`.

The `*_owned` variants require the primitive to live in `Arc<Self>` so the guard or permit can move into a `'static` spawned task.

### Cancellation: CancellationToken

IMPORTANT: `CancellationToken` is in [`tokio_util::sync::CancellationToken`](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html), not in `tokio::sync`. It requires the separate `tokio-util` crate dependency. Code that writes `use tokio::sync::CancellationToken;` will not compile.

```rust
use tokio_util::sync::CancellationToken;

let token = CancellationToken::new();
let child = token.child_token();

tokio::spawn(async move {
    tokio::select! {
        _ = do_work() => {}
        _ = child.cancelled() => {}
    }
});

token.cancel();
```

Key operations:

- `cancel()` — cancels this token and all children.
- `cancelled()` — cancel-safe future that resolves when cancellation happens.
- `child_token()` — one-way hierarchical cancellation.
- `clone()` — symmetric sharing; clones of the same token all cancel together.
- `drop_guard()` — cancel-on-drop unless disarmed.
- `run_until_cancelled(fut)` — runs a future until it completes or cancellation occurs.

Prefer `CancellationToken` over ad-hoc `oneshot` or `broadcast` for shutdown fan-out: it is cheap to clone, hierarchical, and idiomatic.

There is no async `Drop` in stable Rust today. Do not rely on `Drop` to perform async cleanup. Make cleanup idempotent or guard-driven.

### Blocking in async: spawn_blocking, block_in_place, yield_now

Strict rule: never call `std::thread::sleep`, blocking I/O (sync file reads, blocking DNS, sync database drivers), or long CPU-bound work on an async worker thread. Blocking starves the scheduler and destroys throughput.

- [`tokio::task::spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html): runs work on the dedicated blocking thread pool. Default max is `max_blocking_threads` (512). Use for CPU-bound work and blocking I/O. It returns a `JoinHandle`. A running `spawn_blocking` task cannot be aborted; runtime shutdown waits for it, so use `shutdown_timeout` when needed. For long-running background workers, prefer a dedicated `std::thread`.
- [`tokio::task::block_in_place`](https://docs.rs/tokio/latest/tokio/task/fn.block_in_place.html): runs blocking work on the current worker after moving other tasks off. Only works on the `multi_thread` runtime; panics on `current_thread`. It cannot be cancelled. Prefer `spawn_blocking` inside `join!`/`select!`.
- [`tokio::task::yield_now`](https://docs.rs/tokio/latest/tokio/task/fn.yield_now.html): cooperative yield. Use to break hot loops. It provides no hard real-time guarantee.
- `tokio::runtime::Handle::block_on` must not be called inside an async context — it panics.

### tokio::time

Never use `std::thread::sleep` in async code; use [`tokio::time`](https://docs.rs/tokio/latest/tokio/time/). The `time` feature must be enabled and the time driver must be active (`enable_time` or `enable_all`). All time futures are cancellation-safe.

- `sleep`/`sleep_until` → `Sleep` future. Millisecond granularity; Windows timing is platform-dependent.
- `timeout`/`timeout_at` → `Result<T, Elapsed>`. The inner future is dropped on timeout. Caveat: a future that never yields can overrun the deadline without returning `Elapsed`.
- `interval`/`interval_at` → `Interval`. The first tick completes immediately. `tick().await` is cancel-safe.

`MissedTickBehavior`:

- `Burst` (default) — catch up missed ticks as fast as possible.
- `Delay` — enforce minimum spacing; drifts forward.
- `Skip` — stay anchored to the start instant; drop missed ticks.

Decision rule: `Burst` for catch-up, `Delay` for minimum spacing, `Skip` for anchored scheduling.

Use `tokio::time::Instant` rather than `std::time::Instant` when you want test-clock alignment. The `test-util` feature enables `time::pause()`/`resume()`/`advance()` for deterministic tests. It affects only code that reads `tokio::time::Instant`, not `std::time::Instant`.

### tokio::process

[`tokio::process::Command`](https://docs.rs/tokio/latest/tokio/process/) mirrors `std::process::Command` but is async. It requires the `process` feature and the I/O driver (`enable_io`/`enable_all`). It works on **both** `current_thread` and `multi_thread` runtimes; the common claim that it requires `multi_thread` is wrong.

CRITICAL: by default a spawned child keeps running after the `Child` handle is dropped. Drop does not imply cancellation. Set `Command::kill_on_drop(true)` to kill the child on drop and avoid orphaned or zombie children.

On Unix the runtime best-effort reaps zombies, but prefer explicit `child.wait().await`.

Execution modes:

- `.spawn()` → `Child` (interactive, inherits stdio unless redirected).
- `.status().await` → `ExitStatus` (closes pipes, waits for exit).
- `.output().await` → `Output { status, stdout, stderr }` (forces pipes, captures output).

`spawn()` itself is synchronous; waiting is async.

`Child` methods:

- `kill().await` — sends SIGKILL and waits for the child (unlike std's sync `kill`).
- `start_kill()` — sends SIGKILL without waiting.
- `wait().await` — cancel-safe; closes stdin to avoid deadlock. `.take()` stdin first if you need it.
- `try_wait()` — non-blocking check.
- `wait_with_output(self)` — wait and capture output.
- `id()` → `Option<u32>` PID; `None` after completion.
- `.take()` stdin/stdout/stderr before use.

`ChildStdin` implements `AsyncWrite`; `ChildStdout` and `ChildStderr` implement `AsyncRead`. Compose them with `tokio::io` combinators such as `BufReader` and `AsyncBufReadExt::lines`.

### tokio::signal and graceful shutdown

[`tokio::signal`](https://docs.rs/tokio/latest/tokio/signal/) provides portable async signal handling.

- `signal::ctrl_c().await` resolves on the first Ctrl-C after the first poll.
- CAVEAT: registering for a signal installs an OS handler that replaces default behavior for the lifetime of the process, even if the `Signal` handle is dropped.

Unix: `signal::unix::signal(SignalKind)` returns a `Signal` stream. Common constructors: `interrupt()` (SIGINT), `terminate()` (SIGTERM), `hangup()` (SIGHUP), `quit()`, `user_defined1()`, `user_defined2()`.

Windows: `signal::windows::{ctrl_c, ctrl_break, ctrl_close, ctrl_logoff, ctrl_shutdown}`.

CONTAINER RULE: handle **both** SIGINT and SIGTERM. Docker, Kubernetes, and systemd send SIGTERM for graceful shutdown, then SIGKILL after the grace period. Kubernetes default `terminationGracePeriodSeconds` is 30.

Canonical graceful-shutdown pattern:

1. Spawn a supervisor.
2. `select!` on `ctrl_c()` and `signal(SignalKind::terminate())`.
3. Cancel a `CancellationToken`.
4. Workers `select!` on normal work and `token.cancelled()`.
5. Add a hard `tokio::time::timeout`/`sleep` fallback.
6. Force-kill children with `Child::kill()`.
7. Use `biased;` so the shutdown branch is always checked first.

### tokio::net (brief)

[`tokio::net`](https://docs.rs/tokio/latest/tokio/net/) provides async networking primitives.

- TCP: `TcpListener`, `TcpStream`.
- UDP: `UdpSocket`.
- Unix domain sockets: `UnixListener`, `UnixStream`, `UnixDatagram`.
- Windows named pipes.

Canonical server pattern: `loop { let (socket, addr) = listener.accept().await?; tokio::spawn(handle(socket, addr)); }`. `TcpListener::accept` is cancel-safe and can be used in `select!`.

Bind with port `0` for an OS-assigned port, then call `local_addr()` to read it back in tests.

### Async traits

Native `async fn` in traits and return-position `impl Trait` in traits (RPITIT) are stable since Rust 1.75. An `async fn fetch(&self, ...) -> T` desugars to `fn fetch(&self, ...) -> impl Future<Output = T>` ([Async fn and RPITIT in traits](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/)).

Limitations:

- Adding bounds such as `Send` on the returned future is not easy from generic code.
- Adding `+ Send` and relaxing it later is a breaking change.
- Native async traits are not object-safe: no `dyn Trait`.
- Auto-trait leakage lets `async fn` and `-> impl Future` spellings interoperate.

For public, multi-threaded APIs, use the `trait_variant::make` proc macro from rust-lang to emit a `Send` variant. For `dyn Trait` or a pre-1.75 MSRV, use the [`async-trait`](https://docs.rs/async-trait/latest/async_trait/) crate: `#[async_trait]` boxes futures to `Pin<Box<dyn Future + Send + 'a>>`; `#[async_trait(?Send)]` opts out of the `Send` bound. For internal/private traits, bare `async fn` is usually fine.

### Feature flags

Tokio uses Cargo feature flags ([Tokio feature flags](https://docs.rs/tokio/latest/tokio/#feature-flags)).

- `full` enables everything except `test-util` and unstable flags. It is fine for applications; libraries should enable only the subsets they need.
- Common library subset: `rt`, `rt-multi-thread`, `macros`, `net`, `time`, `sync`, `process`, `signal`, `fs`, `io-util`, `io-std`.
- `AsyncRead`/`AsyncWrite` traits need no feature.

## Review checklist

- [ ] No blocking calls (`std::thread::sleep`, blocking I/O, heavy CPU) inside async tasks.
- [ ] No `std::thread::sleep`; `tokio::time` is used for delays and timeouts.
- [ ] `Send` bounds are respected across `.await` in spawned tasks (`tokio::spawn`, `try_join!`, etc.).
- [ ] `std::sync::Mutex`/`RwLock` guards are not held across `.await`.
- [ ] `tokio::select!` loops use cancellation-safe recv-style futures for branches that repeat.
- [ ] The channel type matches the pattern: oneshot, mpsc, broadcast, or watch.
- [ ] CPU-bound or blocking work is placed on `spawn_blocking` (or a dedicated thread).
- [ ] External I/O and user-facing operations have timeouts.
- [ ] Runtime flavor matches workload: `multi_thread` for servers, `current_thread` for tests/CLIs.
- [ ] Subprocesses set `kill_on_drop(true)` when orphaned children are unacceptable.
- [ ] Server/container code handles both SIGINT and SIGTERM.
- [ ] Feature flags are minimal in libraries; `full` is acceptable only in applications.
- [ ] `JoinError` is handled explicitly in production code (panic vs cancellation).
- [ ] `read_exact`, `read_to_end`, `write_all`, `Mutex::lock`, `RwLock`, `Semaphore::acquire`, and `Notify::notified` are not used directly as looping `select!` branches.

## Implementation checklist

- [ ] Add `tokio` with the right feature flags for the crate (`rt-multi-thread`, `macros`, `time`, `sync`, `process`, `signal`, `net`, `fs`, `io-util`, `io-std` as needed; `full` only for apps).
- [ ] Mark `main` with `#[tokio::main]` or build a `tokio::runtime::Runtime` explicitly.
- [ ] Spawn independent work with `tokio::spawn` and await the `JoinHandle`s.
- [ ] Use `tokio::select!` for races and graceful shutdown.
- [ ] Build a `CancellationToken` tree for cooperative cancellation.
- [ ] Put blocking work on `tokio::task::spawn_blocking`.
- [ ] Wrap network, process, and user-facing calls with `tokio::time::timeout`.
- [ ] Set `kill_on_drop(true)` on subprocesses when orphaned children must be avoided.
- [ ] Handle `ctrl_c()` plus SIGTERM on Unix in long-running services.
- [ ] Pick the right channel: oneshot, bounded mpsc, unbounded mpsc, broadcast, or watch.

## Validation hooks

- `cargo check` catches most `Send`/`Sync`/pinning/type errors.
- `cargo clippy -- -D warnings` catches suspicious patterns and unused awaits.
- `cargo test` with `#[tokio::test]` runs the current-thread runtime by default.
- Use `#[tokio::test(flavor = "multi_thread")]` when testing multi-thread behavior:

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_concurrent() {
    // ...
}
```

- Enable the `test-util` feature for deterministic time tests:

```rust
#[tokio::test]
async fn test_timer() {
    tokio::time::pause();
    let start = tokio::time::Instant::now();
    tokio::time::sleep(Duration::from_secs(60)).await;
    assert_eq!(start.elapsed(), Duration::from_secs(60));
}
```

- Code-review lint against blocking calls and `std::thread::sleep`.
- For runtime introspection, use `tokio-console` (requires `tokio_unstable` and `tracing`):

```text
RUSTFLAGS="--cfg tokio_unstable" cargo run
```

## Examples

### Basic async/await

```rust
use std::future::Future;

async fn foo() -> u8 { 5 }

// Equivalent to:
fn foo_explicit() -> impl Future<Output = u8> {
    async { 5 }
}

#[tokio::main]
async fn main() {
    let fut = foo();     // Nothing runs yet — futures are lazy.
    println!("before await");
    let value = fut.await;
    println!("{}", value);
}
```

### Concurrent fan-out with join! (vs the serial anti-pattern)

```rust
use tokio::time::{sleep, Duration};

async fn fetch_a() -> u8 { sleep(Duration::from_millis(10)).await; 1 }
async fn fetch_b() -> u8 { sleep(Duration::from_millis(10)).await; 2 }

#[tokio::main]
async fn main() {
    // ANTI-PATTERN: serial awaits, ~20 ms total.
    // let a = fetch_a().await;
    // let b = fetch_b().await;

    // CORRECT: create futures first, then join, ~10 ms total.
    let a = fetch_a();
    let b = fetch_b();
    let (a, b) = tokio::join!(a, b);
    println!("sum = {}", a + b);
}
```

### Timeout over an external call

```rust
use tokio::time::{timeout, Duration};

async fn fetch() -> Result<String, std::io::Error> {
    // ... network call ...
    Ok("data".to_string())
}

#[tokio::main]
async fn main() {
    match timeout(Duration::from_secs(5), fetch()).await {
        Ok(Ok(data)) => println!("{}", data),
        Ok(Err(e)) => eprintln!("fetch failed: {e}"),
        Err(_) => eprintln!("timed out"),
    }
}
```

### mpsc bounded channel with select!-safe send via reserve/Permit

```rust
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<i32>(16);

    tokio::spawn(async move {
        while let Some(v) = rx.recv().await {
            println!("received {}", v);
        }
    });

    // A reserve/Permit send is cancellation-safe.
    let permit = tx.reserve().await.unwrap();
    permit.send(42);
}
```

### Semaphore for bounded concurrency

```rust
use std::sync::Arc;
use tokio::sync::Semaphore;

async fn limited_work(sem: Arc<Semaphore>) {
    let _permit = sem.acquire().await.expect("semaphore closed");
    // At most N tasks execute do_work concurrently.
    do_work().await;
}
```

### spawn_blocking for CPU work

```rust
#[tokio::main]
async fn main() {
    let handle = tokio::task::spawn_blocking(|| {
        // CPU-intensive or blocking synchronous work.
        expensive_computation(42)
    });

    match handle.await {
        Ok(result) => println!("{}", result),
        Err(e) => eprintln!("blocking task panicked: {e}"),
    }
}
```

### Graceful shutdown: ctrl_c + SIGTERM → CancellationToken → worker select! → hard timeout

```rust
use std::time::Duration;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

async fn worker(token: CancellationToken) {
    loop {
        tokio::select! {
            biased;
            _ = token.cancelled() => {
                println!("worker shutting down");
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                println!("tick");
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = CancellationToken::new();
    let worker_token = token.child_token();

    let handle = tokio::spawn(worker(worker_token));

    tokio::select! {
        biased;
        _ = tokio::signal::ctrl_c() => {}
        #[cfg(unix)]
        _ = {
            let mut sig = tokio::signal::unix::signal(
                tokio::signal::unix::SignalKind::terminate()
            )?;
            sig.recv()
        } => {}
    }

    println!("starting graceful shutdown");
    token.cancel();

    // Hard timeout fallback.
    if timeout(Duration::from_secs(30), handle).await.is_err() {
        eprintln!("shutdown timed out");
    }

    Ok(())
}
```

### tokio::process::Command with kill_on_drop and reading stdout line-by-line

```rust
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new("ls")
        .arg("-la")
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let stdout = child.stdout.take().expect("piped stdout");
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        println!("{}", line);
    }

    let status = child.wait().await?;
    println!("exit status: {}", status);
    Ok(())
}
```

## Common mistakes

- **Serial `.await` when concurrency was meant.** `let a = fa().await; let b = fb().await;` runs sequentially because futures are lazy. Create futures first, then `join!`.
- **Blocking in async.** `std::thread::sleep`, synchronous file reads, blocking DNS, sync database drivers, and long CPU loops on an async worker thread starve other tasks.
- **Busy-waiting with `.await`.** A loop that immediately re-awaits a ready future can still monopolize a worker. Insert `tokio::task::yield_now().await` or a real timer if needed.
- **Holding a `std::sync::MutexGuard` across `.await`.** Deadlocks the runtime and causes `Send` errors in spawned tasks.
- **Unnecessary `Arc<Mutex>`.** Prefer channels when tasks only exchange messages. Shared mutable state is harder to reason about.
- **Dropping a `JoinHandle` accidentally.** The task becomes detached; you lose the result and panic information.
- **Ignoring `JoinError`.** `handle.await.unwrap()` swallows panics and cancellations; inspect `is_cancelled()`/`is_panic()` in production.
- **Using non-cancellation-safe futures in `loop { select! }`.** `read_exact`, `read_to_end`, `write_all`, `Mutex::lock`, `RwLock::read`/`write`, `Semaphore::acquire`, and `Notify::notified` lose progress or data on cancel.
- **Holding a `watch::Ref` across `.await`.** Locks the writer and can deadlock.
- **Not setting `kill_on_drop(true)`.** Subprocesses keep running after the `Child` handle is dropped.
- **Forgetting SIGTERM.** Containers receive SIGTERM first; only handling Ctrl-C (SIGINT) causes abrupt SIGKILL-based termination.
- **Not enabling the I/O or time driver on a hand-built runtime.** `Builder::new_multi_thread().build()` without `enable_io()`/`enable_time()`/`enable_all()` gives you a runtime with no drivers.
- **Using `tokio::spawn` outside a runtime context.** It panics.
- **Using `block_on` inside an async context.** It panics.
- **Mis-locating `CancellationToken`.** It lives in `tokio_util::sync`, not `tokio::sync`, and requires the `tokio-util` crate.

## Strict vs contextual guidance

### Strict guidance

Follow these unless there is a documented, reviewed reason to deviate:

- Never block the async runtime with synchronous sleeps, blocking I/O, or long CPU work — use `spawn_blocking` or a dedicated thread.
- Never hold a `std::sync::Mutex` or `std::sync::RwLock` guard across `.await`.
- Always use `tokio::time` for delays and timeouts in async code; never `std::thread::sleep`.
- Do not spawn tasks that violate `Send + 'static`; holding non-`Send` values across `.await` in a spawned task is a compile error.
- In `loop { select! }`, only use cancellation-safe futures for recv-style branches.
- Do not use `read_exact`, `read_to_end`, `write_all`, `Mutex::lock`, `RwLock::read`/`write`, `Semaphore::acquire`, or `Notify::notified` directly as a `select!` branch in a loop — they lose progress or data on cancel.
- Do not rely on `Drop` for async cleanup; there is no async `Drop` in stable Rust.
- Do not call `tokio::spawn` or `block_on` outside a Tokio runtime context.
- Handle both SIGINT and SIGTERM in server and container code.

### Contextual guidance

Choose per workload and repo policy:

- `current_thread` vs `multi_thread` vs `local` runtime flavor.
- Message passing vs shared state for coordination.
- Bounded vs unbounded `mpsc` (backpressure vs memory).
- `std::sync::Mutex` vs `tokio::sync::Mutex` — only the across-`.await` case forces the latter.
- Native async traits vs `async-trait` crate — use `trait_variant::make` for public `Send` APIs and `async-trait` for `dyn Trait` or older MSRVs.
- `MissedTickBehavior` choice for intervals.
- `CancellationToken` vs a `broadcast` channel for shutdown fan-out.
- `kill_on_drop(true)` vs explicit `Child` lifecycle management.

## Policy decisions for individual repos

Each repository should document:

1. Default runtime flavor and `worker_threads` count.
2. Whether `async-trait` is allowed or native async traits are required, and the MSRV.
3. Maximum blocking-work duration before it must move to `spawn_blocking`.
4. Default mpsc channel capacity and backpressure strategy.
5. Default `MissedTickBehavior` for intervals.
6. Whether `tokio-util::sync::CancellationToken` is the canonical shutdown primitive.
7. Logging and metrics around cancellations, timeouts, spawn failures, aborts, and `JoinError` panics.
8. Whether `kill_on_drop(true)` is mandatory for spawned subprocesses.

## Related docs

- `docs/rust/error-handling.md` — error handling, `Result`, `anyhow`, `thiserror`, and `?` in async code.
- `docs/rust/ownership-lifetimes.md` — ownership, borrowing, lifetimes, and `Pin` fundamentals.
- `docs/rust/testing.md` — testing patterns including `#[tokio::test]` and `test-util`.
- `docs/rust/std-runtime-apis.md` — std APIs that interact with async runtimes.
- `docs/rust/smart-pointers-memory.md` — `Arc`, `Rc`, and shared ownership in async code.
- `docs/rust/types-traits-generics.md` — traits, generics, and async trait design.
- [The Rust Programming Language: Async and Await](https://doc.rust-lang.org/book/ch17-01-futures-and-syntax.html)
- [Asynchronous Programming in Rust](https://rust-lang.github.io/async-book/)

## Related skills

- `nix-usage` — for the Rust toolchain and `nix develop` workflow used by this repository.
- There are no async-specific skills in the current registry. Repository-specific Rust validation or testing skills may live under `.agents/skills/` (for example, `nix-usage` for the dev shell).
