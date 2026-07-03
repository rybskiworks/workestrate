# proptest-state-machine: Stateful Property Testing

## Profile

| field | value |
| --- | --- |
| crate | `proptest-state-machine` |
| version | 0.8.0 |
| license | MIT OR Apache-2.0 |
| org | proptest-rs |
| scope | SEQUENTIAL state machine testing only |
| parallel | NOT supported (planned) |

`proptest-state-machine` extends proptest with stateful property testing: instead of generating a single value, it generates a *sequence of transitions* and applies them to both a pure model and the system under test (SUT), checking invariants after each step. This is the Rust analogue of QuickCheck/EQC's `eqc_statem`.

> "Parallel testing is planned but not yet implemented."

This contrasts with Hughes' `eqc_par_statem` (Hughes §8), which runs two sequential prefixes concurrently against the SUT to find race conditions. proptest-state-machine cannot do that today; see [the gap section](#explicit-gap-no-parallel-state-machine-testing) below.

## Setup

```toml
[dev-dependencies]
proptest = "1.11.0"
proptest-state-machine = "0.8.0"
```

## The model

Stateful property testing follows the **reference-state-machine** pattern. You write two things:

1. A `ReferenceStateMachine` — the **model**: a pure description of the ideal behaviour. It holds the abstract `State`, knows which `Transition`s are valid (`preconditions`), how to advance the state (`apply`), and how to generate transitions (`transitions`).
2. A `StateMachineTest` — the **SUT wrapper**: it constructs the real system under test (`init_test`), drives each transition against it (`apply`), asserts post-conditions (`check_invariants`), and cleans up (`teardown`).

proptest generates a sequence of transitions, applies each one to *both* the model and the SUT, and checks invariants after every step. On failure it shrinks the transition sequence (see [shrinking strategy](#shrinking-strategy)).

## The `ReferenceStateMachine` trait

The model trait, verbatim from docs.rs/proptest-state-machine:

```rust
pub trait ReferenceStateMachine {
    type State;
    type Transition;
    fn init_state() -> Self::State;
    fn transitions(state: &Self::State) -> BoxedStrategy<Self::Transition>;
    fn apply(state: &mut Self::State, transition: &Self::Transition);
    fn preconditions(state: &Self::State, transition: &Self::Transition) -> bool;
}
```

| method | role |
| --- | --- |
| `init_state` | construct the initial model state |
| `transitions` | return a `Strategy` that generates a transition valid for the given state |
| `apply` | advance the model state by a transition (pure, no SUT) |
| `preconditions` | reject transitions that are not valid in the current state (e.g. `Pop` on an empty heap) |

## The `StateMachineTest` trait

The SUT wrapper trait, verbatim:

```rust
pub trait StateMachineTest {
    type State;
    type Transition;
    fn init_test() -> Self;
    fn apply(&mut self, state: &Self::State, transition: &Self::Transition);
    fn check_invariants(&self, state: &Self::State);
    fn teardown(&mut self);
}
```

| method | role |
| --- | --- |
| `init_test` | construct the SUT (one fresh SUT per generated sequence) |
| `apply` | drive a transition against the SUT; the current model `state` is passed for comparison |
| `check_invariants` | assert post-conditions comparing SUT to model |
| `teardown` | release SUT resources |

## The `prop_state_machine!` macro

The macro generates a `#[test]` fn that runs the state machine. The sequential-only form:

```rust
prop_state_machine! {
    #[test]
    fn name_of_test(sequential 1..100 => MyStateMachine) {
        // body optional
    }
}
```

The `sequential` keyword is **required**. There is no `parallel` arm.

## Full example: `state_machine_heap.rs`

This is the canonical runnable example, adapted from the proptest repo's `proptest-state-machine/examples/state_machine_heap.rs`. It models a min-heap (a `Vec`-based reference) against a deliberately buggy `MyHeap` SUT whose `pop` returns the *last* element instead of the minimum. The invariant compares the SUT's popped value against the model's minimum, so the bug is caught.

```rust
#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use proptest_state_machine::{
        prop_state_machine, ReferenceStateMachine, StateMachineTest,
    };

    // ---- System under test (intentionally buggy) ----
    #[derive(Clone, Debug)]
    struct MyHeap<T: Ord + Clone> {
        data: Vec<T>,
    }

    impl<T: Ord + Clone> MyHeap<T> {
        fn new() -> Self {
            MyHeap { data: Vec::new() }
        }
        fn push(&mut self, item: T) {
            self.data.push(item);
        }
        // BUG: returns the LAST element instead of the minimum.
        fn pop(&mut self) -> Option<T> {
            self.data.pop()
        }
        fn len(&self) -> usize {
            self.data.len()
        }
    }

    // ---- Transitions ----
    #[derive(Clone, Debug)]
    enum HeapTransition {
        Push(i32),
        Pop,
    }

    // ---- Reference state machine (the model) ----
    struct HeapRefStateMachine;

    impl ReferenceStateMachine for HeapRefStateMachine {
        type State = Vec<i32>;
        type Transition = HeapTransition;

        fn init_state() -> Self::State {
            Vec::new()
        }

        fn transitions(state: &Self::State) -> BoxedStrategy<Self::Transition> {
            prop_oneof![
                any::<i32>().prop_map(HeapTransition::Push),
                Just(HeapTransition::Pop),
            ]
            .boxed()
        }

        fn apply(state: &mut Self::State, transition: &Self::Transition) {
            match transition {
                HeapTransition::Push(v) => state.push(*v),
                HeapTransition::Pop => {
                    state.pop();
                }
            }
        }

        fn preconditions(state: &Self::State, transition: &Self::Transition) -> bool {
            match transition {
                HeapTransition::Pop => !state.is_empty(),
                _ => true,
            }
        }
    }

    // ---- SUT wrapper ----
    struct HeapStateMachineTest {
        heap: MyHeap<i32>,
    }

    impl StateMachineTest for HeapStateMachineTest {
        type State = Vec<i32>;
        type Transition = HeapTransition;

        fn init_test() -> Self {
            HeapStateMachineTest {
                heap: MyHeap::new(),
            }
        }

        fn apply(&mut self, state: &Self::State, transition: &Self::Transition) {
            match transition {
                HeapTransition::Push(v) => self.heap.push(*v),
                HeapTransition::Pop => {
                    // Pop from the SUT and compare against the model's minimum.
                    let sut_popped = self.heap.pop();
                    let model_min = state.iter().min().copied();
                    prop_assert_eq!(sut_popped, model_min);
                }
            }
        }

        fn check_invariants(&self, state: &Self::State) {
            // The SUT length must match the model length after every step.
            prop_assert_eq!(self.heap.len(), state.len());
        }

        fn teardown(&mut self) {}
    }

    prop_state_machine! {
        #[test]
        fn heap_sequential(sequential 1..100 => HeapRefStateMachine) {
            HeapStateMachineTest
        }
    }
}
```

Because the buggy `pop` returns the last pushed element rather than the minimum, the `prop_assert_eq!(sut_popped, model_min)` inside `apply` will fail for any sequence where the last element differs from the minimum — i.e. almost every non-trivial sequence. proptest will then shrink the sequence to a minimal failing case (typically `Push(a), Push(b)` with `a > b`, then `Pop`).

## Shrinking strategy

proptest shrinks a failing transition sequence by:

1. **Deleting transitions** from the back of the sequence (removing trailing steps that do not affect the failure).
2. **Shrinking each individual transition** via its underlying `Strategy` (e.g. shrinking the `i32` inside `HeapTransition::Push(i32)` toward smaller values).
3. **Shrinking the initial state** produced by `init_state`.

The result is a minimal failing sequence. Because shrinking is *integrated* (see [shrinking](./proptest-shrinking.md)), the shrunk sequence is always a valid sequence that the same generator could have produced — no hand-written shrinker is required.

## Explicit gap: NO parallel state machine testing

> "Parallel testing is planned but not yet implemented."

There is no `parallel` arm in `prop_state_machine!`. This is the central limitation of `proptest-state-machine` 0.8.0. Contrast Hughes §8 `eqc_par_statem`, which finds race conditions by generating two sequential prefixes, running them concurrently against the SUT, and checking that the resulting state is a valid merge of the two model states. The parallel state-machine theory proptest intends to follow is described in Smallbone & Wadler's *Finding Race Conditions in Erlang with QuickCheck*:

- [smallbone.se/papers/finding-race-conditions.pdf](https://smallbone.se/papers/finding-race-conditions.pdf) — parallel state-machine theory.

Until that lands, race-condition testing in Rust must use other tooling (e.g. `loom`, `shuttle`).

## See also

- [overview](./proptest-overview.md) — proptest profile, setup, macro, config.
- [generators](./proptest-generators.md) — `Strategy`/`ValueTree` model and combinators.
- [shrinking](./proptest-shrinking.md) — integrated shrinking via `ValueTree`.

## Sources used

- [docs.rs/proptest-state-machine](https://docs.rs/proptest-state-machine) — `ReferenceStateMachine` and `StateMachineTest` trait signatures.
- [proptest Book: state machine](https://proptest-rs.github.io/proptest/proptest/state-machine.html) — `prop_state_machine!` macro grammar.
- [github.com/proptest-rs/proptest: state_machine_heap.rs](https://github.com/proptest-rs/proptest/blob/main/proptest-state-machine/examples/state_machine_heap.rs) — canonical heap example.
- [smallbone.se/papers/finding-race-conditions.pdf](https://smallbone.se/papers/finding-race-conditions.pdf) — parallel state-machine theory (Smallbone & Wadler).
