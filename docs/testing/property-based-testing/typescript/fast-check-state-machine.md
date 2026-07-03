# fast-check State-Machine Testing

## Purpose

This document covers model-based testing in fast-check via `fc.commands` and the `fc.modelRun` / `fc.asyncModelRun` / `fc.scheduledModelRun` runners. Model-based testing generates sequences of commands against a simplified **model** and the real **system under test (SUT)**, asserting they stay in sync. fast-check shrinks failing command sequences to the minimal reproducing sequence. All examples target fast-check v4.8.0.

## The model + SUT + Command pattern

A model-based test has three parts:

1. **Model** — a pure, simplified representation of the expected state.
2. **Real (SUT)** — the actual implementation under test.
3. **Commands** — classes implementing `fc.Command<Model, Real>`, each with:
   - `check(m: Readonly<Model>): boolean` — a precondition; the command only runs when this is true.
   - `run(m: Model, r: Real): void` — mutate the model and the real, then assert they agree.
   - `toString(): string` — a label for the failure report.

## Runner signatures

- `fc.modelRun(setup, commands)` — synchronous model run.
- `fc.asyncModelRun(setup, commands)` — async model run (commands may be async).
- `fc.scheduledModelRun(setup, commands)` — model run combined with `fc.scheduler` for race-condition testing (see [`fast-check-race-conditions.md`](fast-check-race-conditions.md)).

`setup` is a function `() => { model, real }` that creates a fresh model and SUT for each generated command sequence.

## Full example: a List

This is the canonical fast-check model-based example: a `List` with `push`, `pop`, and `size`, tested against a model that tracks the count.

```typescript
import * as fc from 'fast-check';

// --- System under test ---
class List {
  private data: number[] = [];
  push(v: number): void {
    this.data.push(v);
  }
  pop(): number | undefined {
    return this.data.pop();
  }
  size(): number {
    return this.data.length;
  }
}

// --- Model ---
interface Model {
  num: number;
}

// --- Commands ---
class PushCommand implements fc.Command<Model, List> {
  constructor(private readonly value: number) {}
  check(_m: Readonly<Model>): boolean {
    return true; // push is always valid
  }
  run(m: Model, r: List): void {
    r.push(this.value);
    m.num += 1;
    if (r.size() !== m.num) {
      throw new Error(`push: size ${r.size()} !== model ${m.num}`);
    }
  }
  toString(): string {
    return `Push(${this.value})`;
  }
}

class PopCommand implements fc.Command<Model, List> {
  check(m: Readonly<Model>): boolean {
    return m.num > 0; // guard: only pop when non-empty
  }
  run(m: Model, r: List): void {
    r.pop();
    m.num -= 1;
    if (r.size() !== m.num) {
      throw new Error(`pop: size ${r.size()} !== model ${m.num}`);
    }
  }
  toString(): string {
    return 'Pop';
  }
}

class SizeCommand implements fc.Command<Model, List> {
  check(_m: Readonly<Model>): boolean {
    return true;
  }
  run(m: Model, r: List): void {
    if (r.size() !== m.num) {
      throw new Error(`size: ${r.size()} !== model ${m.num}`);
    }
  }
  toString(): string {
    return 'Size';
  }
}

// --- The property ---
const allCommands = [
  fc.integer().map((v) => new PushCommand(v)),
  fc.constant(new PopCommand()),
  fc.constant(new SizeCommand()),
];

fc.assert(
  fc.property(
    fc.commands(allCommands, { size: '+1' }),
    (cmds) => {
      const s = () => ({ model: { num: 0 }, real: new List() });
      fc.modelRun(s, cmds);
    }
  )
);
```

The `{ size: '+1' }` option biases command-sequence length slightly above the default, exercising longer interactions. The `check` method on `PopCommand` is the **guard**: fast-check only runs `Pop` when the model says the list is non-empty, so the generated sequence never pops from an empty list.

## Replay

On failure, fast-check reports `seed` and `path` (the `replayPath`). Pass them back to reproduce the exact failing command sequence:

```typescript
fc.assert(
  fc.property(fc.commands(allCommands), (cmds) => {
    const s = () => ({ model: { num: 0 }, real: new List() });
    fc.modelRun(s, cmds);
  }),
  { seed: 12345, path: '0:0' }
);
```

## Command-aware shrinking

fast-check shrinks the command sequence, not just each command's arguments. It can drop commands, reorder them, and shrink arguments while respecting `check` preconditions. This is more efficient than shrinking a raw `fc.array(fc.oneof(...))` because the shrinker understands the command structure. See [`fast-check-shrinking.md`](fast-check-shrinking.md).

## Sources used

- [fast-check.dev — Model-based testing](https://fast-check.dev/docs/advanced/model-based-testing/) — fc.commands, Command interface, modelRun/asyncModelRun/scheduledModelRun, the List example
- [fast-check.dev — Replay](https://fast-check.dev/docs/tutorials/quick-start/read-test-reports/) — seed, path, replayPath
- [fast-check.dev — Shrinking](https://fast-check.dev/docs/core-blocks/runners/) — command-aware shrinking
