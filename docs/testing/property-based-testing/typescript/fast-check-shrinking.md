# fast-check Shrinking

## Purpose

This document covers fast-check's integrated automatic shrinking. Every `Arbitrary<T>` carries its own shrinker; combinators preserve shrinking through composition, so you never write a shrinker by hand. On failure, fast-check shrinks the counterexample toward the simplest value that still fails, then reports it. All examples target fast-check v4.8.0.

## Integrated shrinking

Shrinking is **integrated**: the shrinker is part of the arbitrary, not a separate concern the user supplies. When you compose arbitraries — `fc.array(fc.integer())`, `fc.record({...})`, `fc.oneof(...)` — the composed arbitrary shrinks by shrinking its components. You do not implement `shrink` yourself.

The trivial shrink targets per type:

| Type | Trivial shrink target |
|---|---|
| `number` (integer) | `0` |
| `string` | `''` |
| `boolean` | `false` |
| `bigint` | `0n` |
| `Array` | `[]` |
| `Record` | all fields shrunk independently |

## Example: a commutative-add bug

Consider a buggy `add` that breaks when both arguments are equal:

```typescript
function add(a: number, b: number): number {
  // BUG: returns 0 when a === b
  if (a === b) return 0;
  return a + b;
}
```

A commutativity property catches it:

```typescript
import * as fc from 'fast-check';

fc.assert(
  fc.property(fc.integer(), fc.integer(), (a, b) => {
    return add(a, b) === add(b, a);
  })
);
```

fast-check generates a failing pair, then shrinks both integers toward `0`. The shrunk counterexample is `[0, 0]` — the smallest pair where `a === b` still triggers the bug. The failure report looks like:

```text
Property failed after 1 tests
{ seed: ..., path: "0:0", endOnFailure: true }
counterexample: [0, 0]
shrunk 1 time(s)
```

This is the value of integrated shrinking: the report points directly at the minimal reproducing input, not at whatever large random value happened to fail first.

## Shrinking controls

Two combinators modify shrinking behavior. Both are **not recommended** in general — integrated shrinking is almost always what you want.

```typescript
import * as fc from 'fast-check';

// Disable shrinking entirely (not recommended).
fc.noShrink(fc.integer());

// Limit the shrink depth to max levels.
fc.limitShrink(fc.integer(), 10);
```

`fc.noShrink` removes the shrinker from an arbitrary. Use it only when shrinking is provably too slow for a specific arbitrary (e.g. very large generated structures where shrinking explores too many branches). `fc.limitShrink` caps the shrink depth, trading counterexample minimality for speed.

## Command-aware shrinking

For model-based tests (see [`fast-check-state-machine.md`](fast-check-state-machine.md)), fast-check shrinks the **command sequence** rather than each command's arguments independently. This is more efficient than shrinking an array of `fc.oneof(...)` commands, because the shrinker understands the command structure: it can drop whole commands, reorder, and shrink command arguments while preserving the model's preconditions.

Replay a shrunk command sequence via the `seed` and `path` (the `replayPath`) reported on failure:

```typescript
import * as fc from 'fast-check';

fc.assert(
  fc.property(fc.commands(allCommands), (cmds) => {
    fc.modelRun(() => ({ model: initialState, real: new System() }), cmds);
  }),
  { seed: 12345, path: '0:0' } // replay the exact failing sequence
);
```

## Why integrated shrinking matters

Without integrated shrinking, you must hand-write a shrinker for every custom type — and hand-written shrinkers are a common source of bugs (they can skip the minimal counterexample, or even loop). fast-check's design — generator and shrinker paired in the arbitrary, preserved by every combinator — eliminates that class of bug. The trade-off is that the shrinker is fixed per arbitrary, which is why the trivial targets (0, '', false) are chosen to be universally useful.

## Sources used

- [fast-check.dev — Shrinking behavior](https://fast-check.dev/docs/core-blocks/runners/) — integrated shrinking, trivial targets, noShrink, limitShrink
- [Finding Race Conditions in Small Models — MacIver (ECOOP 2020)](https://www.doc.ic.ac.uk/~lcwm2/papers/finding-race-conditions.pdf) — shrinking theory and integrated shrinking
- [Integrated Shrinking — well-typed.com](https://well-typed.com/blog/2019/05/integrated-shrinking/) — why generator+shrinker pairing matters
