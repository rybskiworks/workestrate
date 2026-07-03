# fast-check Race-Condition Testing

## Purpose

This document covers `fc.scheduler()` — fast-check's unique headline feature for async race-condition detection. The scheduler controls the order in which scheduled promises resolve, letting you reproduce timing-dependent bugs deterministically. All examples target fast-check v4.8.0.

## Critical nuance: single-event-loop reordering, not true parallelism

`fc.scheduler` detects races by **reordering Promise resolution on a single event loop**. It does **not** test true parallel execution or linearizability. The scheduler intercepts async operations and decides which resolves first, exploring different interleavings of Promise resolution. This catches races that manifest as ordering bugs in async code, but it is not a linearizability checker.

For contrast, true parallel/linearizability testing (e.g. PULSE-style) runs operations concurrently on multiple processes and checks linearizability against a sequential model. fast-check's scheduler is a lighter-weight, single-threaded approximation: it explores Promise-resolution orderings, not thread-level concurrency.

## The scheduler API (v4)

The current v4 scheduler API. **Note:** `waitOne`, `waitAll`, and `count` were **deprecated since v4.2.0** — use `waitNext`, `waitIdle`, and `report` instead.

```typescript
import * as fc from 'fast-check';

const s = fc.scheduler();

// Schedule a Promise (returns a controlled Promise that resolves when the scheduler allows)
s.schedule(Promise.resolve('a'));

// Schedule a function call (the function runs when the scheduler allows)
s.scheduleFunction((x: number) => Promise.resolve(x * 2));

// Schedule a sequence of operations
s.scheduleSequence([
  { label: 'step1', builder: () => Promise.resolve(1) },
  { label: 'step2', builder: () => Promise.resolve(2) },
]);

// Wait for a specific pending Promise to resolve
await s.waitFor(somePromise);

// Resolve the next scheduled item
await s.waitNext();

// Wait until all scheduled items have resolved
await s.waitIdle();

// Get a report of the ordering (for assertions / debugging)
const report = s.report();
```

`fc.schedulerFor([1, 3, 2])` creates a scheduler with a **fixed** resolution order, useful for replaying a known interleaving.

## Example: queue serialization

A typical scheduler test wraps async calls so the scheduler controls their resolution order, then asserts the final state is consistent regardless of ordering:

```typescript
import * as fc from 'fast-check';

fc.assert(
  fc.asyncProperty(fc.scheduler(), async (s) => {
    const queue: number[] = [];
    const pending: Promise<void>[] = [];

    // Each call is scheduled — the scheduler decides resolution order
    for (let i = 0; i < 5; i++) {
      const p = s.scheduleFunction(async (n: number) => {
        queue.push(n);
      })(i);
      pending.push(p);
    }

    // Wait for all scheduled operations to complete
    await s.waitFor(Promise.all(pending));

    // Assert the invariant holds regardless of resolution order
    // (e.g. no lost writes, no duplicates beyond what the queue semantics allow)
    if (queue.length !== 5) {
      throw new Error(`expected 5 entries, got ${queue.length}`);
    }
  })
);
```

## scheduledModelRun: model-based + scheduler

`fc.scheduledModelRun` combines model-based testing with the scheduler, so you can test stateful async systems where command execution order is non-deterministic. The setup returns `{ model, real, scheduler }`:

```typescript
import * as fc from 'fast-check';

// Pseudocode shape — see fast-check-state-machine.md for the full Command pattern
fc.assert(
  fc.asyncProperty(fc.scheduler(), async (s) => {
    const setup = () => ({
      model: { /* ... */ },
      real: new AsyncSystem(),
      scheduler: s,
    });
    fc.scheduledModelRun(setup, commands);
    await s.waitIdle();
  })
);
```

## When to use the scheduler

- **Use it** when your code has async operations whose resolution order affects correctness (caches, queues, debouncers, request coalescing).
- **Do not use it** for synchronous code, or for async code where ordering is irrelevant (e.g. independent fire-and-forget calls). A plain `fc.asyncProperty` is simpler and faster.
- **Remember the limitation:** the scheduler explores Promise-resolution interleavings on one event loop. It is not a substitute for true parallel linearizability testing.

## Sources used

- [fast-check.dev — Race conditions](https://fast-check.dev/docs/advanced/race-conditions/) — fc.scheduler, schedule/scheduleFunction/scheduleSequence, waitFor/waitNext/waitIdle/report, schedulerFor
- [fast-check.dev — Detect race conditions tutorial](https://fast-check.dev/docs/tutorials/detect-race-conditions/) — queue serialization example, scheduledModelRun
- [Finding Race Conditions in Small Models — MacIver & Papadakis (ECOOP 2020)](https://smallbone.se/papers/finding-race-conditions.pdf) — PULSE linearizability contrast; scheduler is single-event-loop reordering, not true parallelism
