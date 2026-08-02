# fast-check Arbitraries

## Purpose

This document catalogs the fast-check built-in arbitraries, organized into five families. Every arbitrary is a generator paired with an integrated shrinker, and every combinator preserves the shrinking of its inputs — so composed arbitraries shrink toward simple values automatically. All examples target fast-check v4.8.0.

The five families are: **primitives**, **composites**, **combiners**, **recursive**, and **fake data**, plus a small group of **other** arbitraries (falsy, context, commands, gen, scheduler).

## Primitives

Generators for scalar values. Each shrinks toward its type's trivial value (0, '', false, 0n).

```typescript
import * as fc from 'fast-check';

fc.boolean();                              // true | false, shrinks to false
fc.integer();                              // any safe integer, shrinks to 0
fc.integer({ min: -99, max: 99 });         // bounded integer
fc.nat(1000);                              // 0..1000 inclusive
fc.float({ min: 0, max: 1, noNaN: true }); // float in [0,1], no NaN
fc.bigInt();                               // bigint, shrinks to 0n
fc.string();                               // unicode string, shrinks to ''
fc.string({ minLength: 3, maxLength: 6 }); // bounded length
fc.string({ unit: 'grapheme' });           // grapheme-cluster units
fc.date();                                 // Date, shrinks toward epoch
```

## Composites

Generators that build structured values from other arbitraries.

```typescript
import * as fc from 'fast-check';

// Tuple: fixed-length, heterogeneous
fc.tuple(fc.integer(), fc.string(), fc.boolean());

// Array of integers
fc.array(fc.integer());
fc.array(fc.integer(), { minLength: 1, maxLength: 10 });

// Array of unique values (added in 4.4.0)
fc.uniqueArray(fc.integer());

// Record: object with one arbitrary per key
fc.record({
  id: fc.uuid(),
  name: fc.string(),
  age: fc.integer({ min: 0, max: 120 }),
});

// Record with required keys (others optional)
fc.record(
  { id: fc.uuid(), name: fc.string(), age: fc.integer() },
  { requiredKeys: ['id', 'name'] }
);

// Dictionary: string-keyed map of homogeneous values
fc.dictionary(fc.string(), fc.integer());

// Set of unique values (added in 4.4.0)
fc.set(fc.integer(), { minLength: 2, maxLength: 10 });

// Map of unique keys (added in 4.4.0)
fc.map(fc.string(), fc.integer());

// Arbitrary JSON-like values
fc.object();
fc.anything(); // alias of fc.object

// Function returning a value from an arbitrary
fc.func(fc.integer());

// Typed arrays
fc.int32Array();
fc.float32Array();
fc.float64Array();
fc.uint8Array();
```

## Combiners

Operators that select, wrap, or transform arbitraries.

```typescript
import * as fc from 'fast-check';

// Constant single value
fc.constant(42);

// Constant from a fixed set
fc.constantFrom('read', 'write', 'delete');

// Subarray (preserves order) / shuffled subarray
fc.subarray([1, 2, 3, 4, 5]);
fc.shuffledSubarray([1, 2, 3, 4, 5]);

// Optional / nullable
fc.option(fc.string()); // string | null (default nil)

// Union: pick one of several arbitraries
fc.oneof(fc.integer(), fc.string(), fc.boolean());

// Weighted union
fc.oneof(
  { arbitrary: fc.constant('small'), weight: 5 },
  { arbitrary: fc.constant('large'), weight: 1 }
);

// Clone a value across an arbitrary (same value N times)
fc.clone(fc.integer(), 3);

// Chain until a predicate holds (added in 4.8.0)
fc.chainUntil(fc.integer(), (n) => fc.constant(n), { maxGeneratedValues: 100 });

// Remove bias toward small values
fc.noBias(fc.integer());

// Disable shrinking entirely (not recommended — see shrinking doc)
fc.noShrink(fc.integer());

// Limit shrink depth
fc.limitShrink(fc.integer(), 10);

// Filter generated values
fc.integer().filter((n) => n > 0);

// Transform generated values
fc.integer().map((n) => n * 2);

// Dependent generation
fc.integer({ min: 1, max: 10 }).chain((n) =>
  fc.tuple(fc.constant(n), fc.integer({ min: 0, max: n }))
);
```

## Recursive

For self-referential structures (trees, JSON), use `fc.letrec`. Prefer `letrec` over `fc.memo`.

```typescript
import * as fc from 'fast-check';

// A recursive JSON-like value via letrec with depthFactor
const jsonArb = fc.letrec((tie) => ({
  value: fc.oneof(
    { depthFactor: 0.5 },
    fc.constant(null),
    fc.boolean(),
    fc.integer(),
    fc.string(),
    fc.array(tie('value')),
    fc.dictionary(fc.string(), tie('value'))
  ),
}));

// A binary tree
const treeArb = fc.letrec((tie) => ({
  node: fc.record({
    value: fc.integer(),
    left: fc.option(tie('node'), { nil: null }),
    right: fc.option(tie('node'), { nil: null }),
  }),
}));

// fc.memo exists but letrec is preferred
// fc.memo((depth) => depth <= 0 ? fc.constant('leaf') : fc.oneof(...))

// Entity graph (relationship between generated entities)
fc.entityGraph();

// String matching a regex (generates strings matching the pattern)
fc.stringMatching(/^[a-z]+@[a-z]+\.[a-z]{2,}$/);
```

## Fake data

Generators for common formatted strings. **Warning:** these have a narrower shrink space than primitive arbitraries — use them only when your code branches on the format. If your code treats the value as an opaque string, prefer `fc.string()`.

```typescript
import * as fc from 'fast-check';

fc.ulid();                       // ULID identifier
fc.uuid({ version: 4 });        // UUID v4
fc.ipV4();                       // IPv4 address
fc.ipV6();                       // IPv6 address
fc.domain();                     // domain name
fc.webUrl();                     // http(s) URL
fc.emailAddress();               // email address
fc.base64String();               // base64-encoded string
```

## Other arbitraries

```typescript
import * as fc from 'fast-check';

fc.falsy();        // a falsy value (false, 0, '', null, undefined, NaN)
fc.context();      // a context object for logging inside properties
fc.commands([...]); // command sequence for model-based testing
fc.gen();          // generator-based arbitrary (advanced)
fc.scheduler();    // async scheduler for race-condition testing
```

See [`fast-check-state-machine.md`](fast-check-state-machine.md) for `fc.commands` and [`fast-check-race-conditions.md`](fast-check-race-conditions.md) for `fc.scheduler`.

## Sources used

- [fast-check.dev — Arbitraries](https://fast-check.dev/docs/core-blocks/arbitraries/) — overview of the five families
- [fast-check.dev — Primitives](https://fast-check.dev/docs/core-blocks/arbitraries/primitives/) — boolean, integer, nat, float, bigInt, string, date
- [fast-check.dev — Composites](https://fast-check.dev/docs/core-blocks/arbitraries/composites/) — tuple, array, uniqueArray, record, dictionary, set, map, object, func, typed arrays
- [fast-check.dev — Combiners](https://fast-check.dev/docs/core-blocks/arbitraries/combiners/) — constant, constantFrom, subarray, option, oneof, chain, noBias, noShrink, limitShrink
- [fast-check.dev — Recursive](https://fast-check.dev/docs/core-blocks/arbitraries/recursive/) — letrec, memo, entityGraph, stringMatching
- [fast-check.dev — Fake data](https://fast-check.dev/docs/core-blocks/arbitraries/fake-data/) — ulid, uuid, ipV4, ipV6, domain, webUrl, emailAddress, base64String
