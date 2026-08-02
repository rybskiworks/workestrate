# fast-check Core Concepts

## Purpose

This document covers the fast-check execution model: the `Arbitrary<T>` (a generator paired with an integrated shrinker), the two runners (`fc.assert` throws on failure, `fc.check` returns `RunDetails`), synchronous and async properties, pre-conditions, and the run-parameters object that controls run count, seed, path, and verbosity. Every example is runnable TypeScript against fast-check v4.8.0.

## Library profile

- **Package:** `fast-check` v4.8.0 (May 2026).
- **License:** MIT.
- **Downloads:** ~30.5M weekly on npm.
- **Maintainer:** Nicolas Dubien (sole maintainer of the `dubzzz/fast-check` monorepo).
- **Module system:** v4 is **ESM-only**. No CommonJS build. Requires Node ≥ 12.17.0 and TypeScript ≥ 5.0.

## Setup

Install fast-check plus the adapter for your test runner:

```json
// package.json
{
  "devDependencies": {
    "fast-check": "^4.8.0",
    "@fast-check/jest": "^2.2.0"
  }
}
```

For Vitest, use `@fast-check/vitest` 0.4.1 instead; for AVA, use `@fast-check/ava` 3.0.1. See [`fast-check-runner-integration.md`](fast-check-runner-integration.md) for adapter details.

## The Arbitrary model

An **arbitrary** is a generator paired with a shrinker. fast-check calls this `Arbitrary<T>`. You rarely implement the shrinker yourself; you compose built-in arbitraries and combinators, and every combinator preserves the integrated shrinking of its inputs.

```typescript
import * as fc from 'fast-check';

// fc.integer() is an Arbitrary<number>: it generates integers AND knows
// how to shrink any generated integer toward 0.
const intArb: fc.Arbitrary<number> = fc.integer();
```

## fc.assert vs fc.check

- **`fc.assert(property, params?)`** — runs the property; **throws** on failure with a shrunk counterexample. Use this inside test runners.
- **`fc.check(property, params?)`** — runs the property; **returns** a `RunDetails<Ts>` object. Use this when you want to inspect the result programmatically.

```typescript
import * as fc from 'fast-check';

// fc.assert throws on failure — the normal test-runner path.
fc.assert(
  fc.property(fc.integer(), fc.integer(), (a, b) => {
    return a + b === b + a;
  })
);

// fc.check returns RunDetails — inspect failed, numRuns, counterexample, etc.
const result = fc.check(
  fc.property(fc.integer(), (n) => n === n)
);
// result.failed === false
// result.numRuns === 100
```

## A minimal property: substring

The canonical fast-check example tests that a string always contains itself:

```typescript
import * as fc from 'fast-check';

fc.assert(
  fc.property(fc.string(), (text) => {
    return contains(text, text);
  })
);

function contains(source: string, pattern: string): boolean {
  return source.includes(pattern);
}
```

## Multi-argument properties

Pass multiple arbitraries; the predicate receives one argument per arbitrary, in order:

```typescript
import * as fc from 'fast-check';

fc.assert(
  fc.property(
    fc.string(),
    fc.string(),
    fc.string(),
    (a, b, c) => {
      // (a + b + c) must contain b as a substring
      return (a + b + c).includes(b);
    }
  )
);
```

## Async properties

For predicates that return a `Promise`, use `fc.asyncProperty`. The predicate may return `Promise<boolean>` or `Promise<void>` (void means success):

```typescript
import * as fc from 'fast-check';

fc.assert(
  fc.asyncProperty(fc.string(), async (s) => {
    const result = await someAsyncOperation(s);
    return typeof result === 'string';
  })
);

async function someAsyncOperation(input: string): Promise<string> {
  return input.toUpperCase();
}
```

## Pre-conditions

`fc.pre(condition)` skips the current run when the condition is false — the run does not count as a failure. Use it for invariants that only hold on a subset of the generated space, when constraining the arbitrary would be too aggressive:

```typescript
import * as fc from 'fast-check';

fc.assert(
  fc.property(fc.integer(), fc.integer(), (a, b) => {
    fc.pre(b !== 0); // skip division by zero
    return Math.floor(a / b) * b + (a % b) === a;
  })
);
```

## Run parameters

Control generation, replay, and reporting via the params object:

```typescript
import * as fc from 'fast-check';

fc.assert(
  fc.property(fc.integer(), fc.integer(), (a, b) => a + b === b + a),
  {
    numRuns: 1000,            // default 100
    seed: 1527422598337,      // pin for reproducibility
    path: '0:0',              // replay a specific case within the seed
    endOnFailure: true,       // stop at the first failure
  }
);
```

- `numRuns` — number of generated values (default 100).
- `seed` — integer; pin to reproduce a failure.
- `path` — string like `'0:0'`; selects a specific case within a seed for replay.
- `endOnFailure` — stop immediately on the first failing case instead of continuing.

## Reading a failure report

When `fc.assert` fails, it prints a report with the seed, path, counterexample, and shrink count. The verbatim shape:

```text
Property failed after 1 tests
{ seed: -1527422598337, path: "0:0", endOnFailure: true }
counterexample: ["", "", ""]
shrunk 1 time(s)
Got error: Error: Property failed
```

To reproduce, copy the `seed` and `path` into the params object of the same property. See [`fast-check-shrinking.md`](fast-check-shrinking.md) for shrinking details and [`fast-check-runner-integration.md`](fast-check-runner-integration.md) for framework-specific reporting.

## Sources used

- [fast-check.dev — Runners](https://fast-check.dev/docs/core-blocks/runners/) — fc.assert, fc.check, RunDetails, run parameters
- [fast-check.dev — Properties](https://fast-check.dev/docs/core-blocks/properties/) — fc.property, fc.asyncProperty, fc.pre
- [github.com/dubzzz/fast-check — README](https://github.com/dubzzz/fast-check#readme) — overview, ESM-only note, version
- [npmjs.com/package/fast-check](https://www.npmjs.com/package/fast-check) — version 4.8.0, downloads, license, Node ≥12.17.0
