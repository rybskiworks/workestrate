# proptest: Generators — Strategies and ValueTrees

## The two-step model

proptest is per-**value**, not per-type. This is the central contrast with `quickcheck`'s per-type `Arbitrary` typeclass (see [proptest vs quickcheck](./proptest-vs-quickcheck.md) and [quickcheck](./quickcheck.md)). A generator is a `Strategy`: it produces, for each generated value, a `ValueTree` that holds both the current value and the ability to shrink it. Shrinking therefore composes automatically across compound generators because each component carries its own `ValueTree`. See [shrinking](./proptest-shrinking.md) for the `ValueTree` mechanics.

## Primitive strategies

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn primitives(
        n in prop::num::i32::ANY,
        b in prop::bool::ANY,
        s in prop::string::string("[a-z]{1,10}"),
    ) {
        // just exercise the generators
        let _ = (n, b, s);
    }
}
```

Notes:

- `prop::string::string(pattern)` takes a regex.
- `prop::num::*::ANY` covers `i8..i64`, `u8..u64`, `f32`/`f64`, `isize`/`usize`.
- `prop::bool::ANY` generates booleans.

## Collections

```rust
use proptest::prelude::*;
use proptest::collection::vec;

proptest! {
    #[test]
    fn collections(v in vec(prop::num::i32::ANY, 0..100)) {
        prop_assert!(v.len() <= 100);
    }
}
```

`vec(element_strategy, size_range)` generates a `Vec<T>` whose length is drawn from `size_range`.

## `prop_oneof!` — uniform and weighted

```rust
use proptest::prelude::*;

#[derive(Debug, Clone)]
enum MyEnum { A, B(u32), C(String) }

fn my_enum_strategy() -> impl Strategy<Value = MyEnum> {
    prop_oneof![
        Just(MyEnum::A),
        any::<u32>().prop_map(MyEnum::B),
        any::<String>().prop_map(MyEnum::C),
    ]
}
```

The weighted form assigns relative frequencies:

```rust
use proptest::prelude::*;

#[derive(Debug, Clone)]
enum MyEnum { A, B(u32), C(String) }

fn my_enum_weighted() -> impl Strategy<Value = MyEnum> {
    prop_oneof![
        1 => Just(MyEnum::A),
        2 => any::<u32>().prop_map(MyEnum::B),
        1 => any::<String>().prop_map(MyEnum::C),
    ]
}
```

## `prop_compose!` — independent and dependent

The 2-arg (independent) form bundles several strategies into a struct:

```rust
use proptest::prelude::*;

#[derive(Debug, Clone)]
struct MyStruct { a: i32, b: bool }

prop_compose! {
    fn my_struct_strategy()(a in prop::num::i32::ANY, b in prop::bool::ANY) -> MyStruct {
        MyStruct { a, b }
    }
}
```

The 3-arg (dependent) form lets later fields depend on earlier ones — the `nearby_numbers` example from the Book:

```rust
use proptest::prelude::*;

prop_compose! {
    fn nearby_numbers()
        (base in 0i32..1000,
         offset in -100i32..100,
         // dependent on base and offset
         nearby in (base + offset)..(base + offset + 10))
        -> (i32, i32, i32) {
        (base, offset, nearby)
    }
}
```

## Strategy combinators

| combinator | signature sketch | purpose |
| --- | --- | --- |
| `prop_map` | `.prop_map(\|v\| transform(v))` | map the output value |
| `prop_flat_map` | `.prop_flat_map(\|v\| strat(v))` | dependent generation |
| `prop_filter` | `.prop_filter("reason", \|v\| pred(v))` | filter values (see [shrinking](./proptest-shrinking.md) sharp edge) |
| `prop_recursive` | `.prop_recursive(depth, size, min, \|inner\| ...)` | recursive self-referential strategies |
| `prop_shuffle` | `.prop_shuffle()` | shuffle a slice |
| `boxed` | `.boxed()` | erase the strategy type via `BoxedStrategy<T>` |
| `no_shrink` | `.no_shrink()` | disable shrinking for this strategy |

## Recursive JSON AST via `prop_recursive`

The canonical recursive-data example: a JSON AST generated with `prop_recursive` + `prop_oneof` + `prop::collection::vec`:

```rust
use proptest::prelude::*;
use proptest::collection::vec;

#[derive(Debug, Clone)]
enum JsonNode {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonNode>),
    Object(Vec<(String, JsonNode)>),
}

fn json_node() -> impl Strategy<Value = JsonNode> {
    let leaf = prop_oneof![
        Just(JsonNode::Null),
        any::<bool>().prop_map(JsonNode::Bool),
        any::<f64>().prop_map(JsonNode::Number),
        any::<String>().prop_map(JsonNode::String),
    ];
    leaf.prop_recursive(
        4,   // max depth
        256, // max total size
        1,   // items per collection lower bound
        |inner| {
            prop_oneof![
                vec(inner.clone(), 0..10).prop_map(JsonNode::Array),
                vec((any::<String>(), inner), 0..10).prop_map(JsonNode::Object),
            ]
        },
    )
}
```

## `Arbitrary` trait and `any::<T>()`

proptest also provides an `Arbitrary` trait (in `proptest::arbitrary`) and the `any::<T>()` / `any_with::<T>(args)` helpers for types that implement it. Many stdlib types implement `Arbitrary`:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn arbitrary_example(v in any::<Vec<u32>>()) {
        prop_assert!(v.iter().all(|&x| x == x)); // trivial
    }
}
```

`Arbitrary` is the bridge to the per-type world: when a type implements `Arbitrary`, `any::<T>()` yields a `Strategy<Value = T>`. But the shrinking still lives in the per-value `ValueTree`, not in a per-type `shrink` function.

## See also

- [shrinking](./proptest-shrinking.md) — `ValueTree` and integrated shrinking.
- [overview](./proptest-overview.md) — setup, macro, config, failure persistence.

## Sources used

- [proptest Book — Strategy basics](https://proptest-rs.github.io/proptest/proptest/tutorial/strategy-basics.html) — primitive and compound strategies.
- [proptest Book — Compound strategies](https://proptest-rs.github.io/proptest/proptest/tutorial/compound-strategies.html) — `prop_oneof!`, `prop_recursive`.
- [proptest Book — `prop_compose!`](https://proptest-rs.github.io/proptest/proptest/tutorial/macro-prop-compose.html) — independent and dependent composition.
- [docs.rs/proptest](https://docs.rs/proptest) — `Strategy` trait, `Arbitrary` trait, combinator signatures.
