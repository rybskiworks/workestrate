# NEXT-SESSION — workestrate resumption context

> Narrative resumption notes. Task lists link to `wrk-*` beads IDs; this doc
> carries context, not work items (see BEADS.md for the boundary).

## Open follow-ups

- **wrk-bvu (spec 21 epic) — revisit the KVM-test `MSB_HOME` convention for
  a more idiomatic approach.** The `common::short_msb_home()` `/tmp`-based
  helper + the `MSB_HOME`-vs-`HOME` decoupling in `lifecycle_detached.rs` /
  `ensure_images_e2e.rs` is a pragmatic workaround for the 108-byte unix
  socket limit; brainstorm a configurable msb run/socket dir or a canonical
  short-`MSB_HOME` test convention and refactor. See spec 21 §14.
