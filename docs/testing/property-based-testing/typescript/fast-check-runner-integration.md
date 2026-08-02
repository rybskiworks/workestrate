# fast-check Runner Integration

## Purpose

fast-check is framework-agnostic at its core (`fc.assert` works inside any `test()` block), but the `dubzzz/fast-check` monorepo ships first-party adapters that integrate property tests natively with Jest, Vitest, and AVA. This document covers each adapter's API, the Bun approach (no dedicated adapter, by design), and the Node built-in test runner. Every example is runnable.

## Adapter versions

All adapters live in the `dubzzz/fast-check` monorepo and re-export `fc` so you do not need a separate `fast-check` dependency:

| Adapter | Version | Runner |
|---|---|---|
| `@fast-check/jest` | 2.2.0 | Jest |
| `@fast-check/vitest` | 0.4.1 | Vitest |
| `@fast-check/ava` | 3.0.1 | AVA |

## @fast-check/jest

`@fast-check/jest` re-exports `test` and `fc`. Use `test.prop([arb, arb, ...])('name', (a, b, ...) => ...)`:

```typescript
// using @fast-check/jest
import { test, fc } from '@fast-check/jest';

test.prop([fc.string(), fc.string(), fc.string()])(
  'concatenated string contains the middle part',
  (a, b, c) => (a + b + c).includes(b)
);
```

The named-argument form passes arbitraries as a record, and the predicate receives a matching object:

```typescript
import { test, fc } from '@fast-check/jest';

test.prop({
  a: fc.string(),
  b: fc.string(),
  c: fc.string(),
})('concatenated string contains the middle part', ({ a, b, c }) =>
  (a + b + c).includes(b)
);
```

Jest modifiers are available: `test.only.prop(...)`, `test.skip.prop(...)`, `test.todo` (no `.prop`), etc.

The worker runner lives at `@fast-check/jest/worker` and is **ESM-only** — it runs each property in a worker thread for isolation.

## @fast-check/vitest

`@fast-check/vitest` requires **vitest ^4.1.0** and **Node ≥ 20.19.0**. There are two modes.

**Mode A — one-time random via `g()`.** `g()` produces a single random value from an arbitrary; useful for fixtures:

```typescript
import { test, expect, g } from '@fast-check/vitest';
import { fc } from '@fast-check/vitest';

test('a generated integer is a number', () => {
  const n = g(fc.integer());
  expect(typeof n).toBe('number');
});
```

**Mode B — `test.prop`.** The full property form, mirroring the Jest adapter:

```typescript
import { test, expect } from '@fast-check/vitest';
import { fc } from '@fast-check/vitest';

test.prop([fc.integer(), fc.integer()])('addition is commutative', (a, b) => {
  expect(a + b).toBe(b + a);
});
```

## @fast-check/ava

`@fast-check/ava` re-exports `testProp` and `fc`. **Note:** AVA assertions use `t.true(...)` (or another `t.*` assertion) — you do **not** return a boolean from the predicate:

```typescript
import { testProp, fc } from '@fast-check/ava';

testProp(
  'addition is commutative',
  [fc.integer(), fc.integer()],
  (t, a, b) => {
    t.true(a + b === b + a);
  }
);
```

## Bun

Bun has **no dedicated fast-check adapter by design** — Bun's `bun:test` runner is simple enough that raw `fc.assert` inside a `test()` block is the intended usage:

```typescript
import { test, expect } from 'bun:test';
import * as fc from 'fast-check';

test('reverse is involutive', () => {
  fc.assert(
    fc.property(fc.string(), (s) => {
      const twice = s.split('').reverse().reverse().join('');
      return twice === s;
    })
  );
});
```

## Node built-in test runner

The Node.js built-in test runner (`node:test`) works with raw `fc.assert`. With CommonJS (pre-v4 fast-check supported CJS; v4 is ESM-only, so use ESM `import`):

```typescript
import { test } from 'node:test';
import * as fc from 'fast-check';

test('addition is commutative', () => {
  fc.assert(
    fc.property(fc.integer(), fc.integer(), (a, b) => a + b === b + a)
  );
});
```

## Sources used

- [npmjs.com/package/@fast-check/jest](https://www.npmjs.com/package/@fast-check/jest) — test.prop API, named-arg form, worker runner, version 2.2.0
- [npmjs.com/package/@fast-check/vitest](https://www.npmjs.com/package/@fast-check/vitest) — g() and test.prop modes, vitest ^4.1.0, Node ≥20.19.0, version 0.4.1
- [npmjs.com/package/@fast-check/ava](https://www.npmjs.com/package/@fast-check/ava) — testProp, t.true assertion style, version 3.0.1
- [fast-check.dev — Bun tutorial](https://fast-check.dev/docs/tutorials/setting-up-your-test-environment/property-based-testing-with-bun/) — no dedicated adapter, raw fc.assert
- [fast-check.dev — Node test runner](https://fast-check.dev/docs/tutorials/setting-up-your-test-environment/) — node:test integration
