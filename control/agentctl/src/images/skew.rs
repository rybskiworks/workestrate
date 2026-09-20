//! The skew matrix decision function (spec 21 §3.4) — PURE, exhaustively
//! unit-tested.
//!
//! Phase C feeds this the change-detection outcome: the record state (the
//! drvPath/outPath comparison itself is phase C's job — `nix eval --raw
//! <repo>#<attr>.drvPath` against the recorded `drv_path`/`out_path`, out of
//! scope for phase B) plus the msb store-tag presence and the
//! `--reload-images` force flag. The output drives build/load/record.
//!
//! Every branch re-checks INSIDE the per-tag lock (spec §7 "Concurrent
//! same-tag build" row): the winner's rebuild usually flips a waiter's
//! verdict to [`SkewDecision::Skip`].

/// Record state for a `<repo>#<tag>` key, derived by phase C (spec §3.1).
///
/// A2 (ADR 0032 §Image tags): the key carries the COMPUTED content-
/// addressed tag (`<repo>#<name:ctx.sha>`), so `Fresh` = "a record exists
/// under the computed key" (presence IS freshness) and `Stale` is
/// unreachable from the ensure/build flow — stale content computes a
/// DIFFERENT tag and keys Absent. The variant remains for the documented
/// §3.4 row-2 semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordState {
    /// No record for this key in `images.json` (first run, changed content
    /// (a new tag), or another config / the old manual ritual loaded the tag —
    /// see the TRUST branch, D1).
    Absent,
    /// Record present under the computed content-addressed tag.
    Fresh,
    /// Record present; content differs from the current eval. Unreachable
    /// under A2 content-addressed tags (kept for matrix completeness).
    Stale,
}

/// Whether the tag exists in the msb image store (ground truth for presence —
/// spec §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreTag {
    Present,
    Gone,
}

/// What the ensure-images pre-flight should do (spec §3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkewDecision {
    /// Fresh record + tag present: nothing to do (§3.4 row 1).
    Skip,
    /// Stale record + tag present, or record present + tag gone: rebuild →
    /// load → record (§3.4 rows 2–3).
    Rebuild,
    /// No record + tag present on a PLAIN `up`: TRUST the store tag
    /// (USER DECISION D1, §3.4 row 4) — the store may have been populated by
    /// the old manual ritual or another config. Records are advisory, never
    /// authoritative; the tag is used as-is and a record is written so the
    /// next run has a baseline. A future tag-digest mismatch flips this
    /// branch to rebuild (§3.5, pending the msb digest surface).
    TrustAndRecord,
    /// No record + no tag: build → load → record (§3.4 row 5).
    Build,
    /// `--reload-images` flipped a [`Skip`](SkewDecision::Skip) or
    /// [`TrustAndRecord`](SkewDecision::TrustAndRecord) verdict: rebuild →
    /// load → record regardless of change detection (§5.2), including
    /// re-establishing the record over a D1-trusted tag.
    RebuildForced,
}

/// The §3.4 skew matrix, plus the `--reload-images` flip (§5.2).
///
/// `force` is the parent-side `--reload-images` flag. It flips ONLY the
/// Skip/Trust verdicts to [`SkewDecision::RebuildForced`] — Rebuild/Build
/// rows already rebuild, so force is a no-op on them (it changes nothing to
/// "force" a build that was already going to happen).
pub fn decide_skew(record: RecordState, store: StoreTag, force: bool) -> SkewDecision {
    let base = match (record, store) {
        // §3.4 row 1: fresh + present → skip.
        (RecordState::Fresh, StoreTag::Present) => SkewDecision::Skip,
        // §3.4 row 2: stale + present → rebuild.
        (RecordState::Stale, StoreTag::Present) => SkewDecision::Rebuild,
        // §3.4 row 3: record present + tag gone → rebuild (fresh or stale —
        // the recorded bits are not in the store either way).
        (RecordState::Fresh | RecordState::Stale, StoreTag::Gone) => SkewDecision::Rebuild,
        // §3.4 row 4 (USER DECISION D1): absent + present → TRUST on plain up.
        (RecordState::Absent, StoreTag::Present) => SkewDecision::TrustAndRecord,
        // §3.4 row 5: absent + absent → build.
        (RecordState::Absent, StoreTag::Gone) => SkewDecision::Build,
    };
    if force {
        match base {
            // §5.2: `--reload-images` flips skip/trust to a forced rebuild —
            // "the explicit escape that re-establishes the record" (§3.4).
            SkewDecision::Skip | SkewDecision::TrustAndRecord => SkewDecision::RebuildForced,
            other => other,
        }
    } else {
        base
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;

    // ---- §3.4 rows, one dedicated test each (plain `up`, force=false) ----

    /// §3.4 row 1: fresh (drvPath + outPath match current eval) + present →
    /// **skip** — nothing to do.
    #[test]
    fn row1_fresh_record_present_tag_skips() {
        assert_eq!(
            decide_skew(RecordState::Fresh, StoreTag::Present, false),
            SkewDecision::Skip
        );
    }

    /// §3.4 row 2: stale (drvPath or outPath differs) + present → **rebuild**
    /// → load → record.
    #[test]
    fn row2_stale_record_present_tag_rebuilds() {
        assert_eq!(
            decide_skew(RecordState::Stale, StoreTag::Present, false),
            SkewDecision::Rebuild
        );
    }

    /// §3.4 row 3: record present + tag gone → **rebuild** → load → record.
    /// Both record states rebuild: the store does not hold the recorded bits.
    #[test]
    fn row3_record_present_tag_gone_rebuilds() {
        assert_eq!(
            decide_skew(RecordState::Fresh, StoreTag::Gone, false),
            SkewDecision::Rebuild,
            "fresh record but the tag vanished from the store → rebuild"
        );
        assert_eq!(
            decide_skew(RecordState::Stale, StoreTag::Gone, false),
            SkewDecision::Rebuild,
            "stale record and no tag → rebuild"
        );
    }

    /// §3.4 row 4 (USER DECISION D1): record absent + tag present → **TRUST
    /// the store tag on plain `up`**, then record. The TRUST branch exists
    /// because the store may have been populated by the old manual ritual (or
    /// another config) before this tool ever recorded it — refusing to trust it
    /// would force a spurious rebuild of every pre-existing tag on first run.
    #[test]
    fn row4_absent_record_present_tag_trusts_on_plain_up() {
        assert_eq!(
            decide_skew(RecordState::Absent, StoreTag::Present, false),
            SkewDecision::TrustAndRecord
        );
    }

    /// §3.4 row 5: record absent + tag absent → **build** → load → record.
    #[test]
    fn row5_absent_record_absent_tag_builds() {
        assert_eq!(
            decide_skew(RecordState::Absent, StoreTag::Gone, false),
            SkewDecision::Build
        );
    }

    /// A2 stage 2 GC framing (ADR 0032 §Image tags — RESOLVED user decision
    /// 3): a keep-last-N-pruned tag reads `StoreTag::Gone` at the next
    /// ensure/build. That is §3.4 ROW 3 verbatim — Fresh|Stale + Gone →
    /// **Rebuild**, never an error: "a pruned tag = rebuild-from-store on
    /// recreate". The prune itself never touches a running sandbox and the
    /// recreate path needs no special case; this test pins the invariant so
    /// a future matrix edit cannot turn pruned-tag recreation into a hard
    /// failure.
    #[test]
    fn gc_pruned_tag_reads_store_gone_and_rebuilds_never_errors() {
        // The pruned tag's record still exists (records are only dropped by
        // the prune's state cleanup) but the store no longer holds the bits.
        assert_eq!(
            decide_skew(RecordState::Fresh, StoreTag::Gone, false),
            SkewDecision::Rebuild,
            "pruned tag with a surviving record → row 3 Rebuild"
        );
        assert_eq!(
            decide_skew(RecordState::Stale, StoreTag::Gone, false),
            SkewDecision::Rebuild,
            "pruned tag with a stale record → row 3 Rebuild"
        );
        // If the prune ALSO dropped the record (already-gone cleanup), the
        // recreate is row 5 Build — still a rebuild-shaped success.
        assert_eq!(
            decide_skew(RecordState::Absent, StoreTag::Gone, false),
            SkewDecision::Build
        );
        // --reload-images changes nothing on these rows (force is a no-op on
        // rows that already rebuild).
        assert_eq!(
            decide_skew(RecordState::Fresh, StoreTag::Gone, true),
            SkewDecision::Rebuild
        );
    }

    // ---- `--reload-images` force variants (§5.2) ----

    /// §5.2: force flips the row-1 SKIP to a forced rebuild.
    #[test]
    fn force_flips_skip_to_rebuild_forced() {
        assert_eq!(
            decide_skew(RecordState::Fresh, StoreTag::Present, true),
            SkewDecision::RebuildForced
        );
    }

    /// §5.2 + §3.4: force flips the D1 TRUST branch — "the explicit escape
    /// that re-establishes the record" (and the §12.3 record-seeding path:
    /// the first `--reload-images` moves a repo from manual-ritual provenance
    /// to tool-recorded provenance).
    #[test]
    fn force_flips_d1_trust_to_rebuild_forced() {
        assert_eq!(
            decide_skew(RecordState::Absent, StoreTag::Present, true),
            SkewDecision::RebuildForced
        );
    }

    /// Force is a no-op on rows that already rebuild: nothing to "force" —
    /// the verdict stays Rebuild (not RebuildForced, which is reserved for
    /// the skip/trust flips so callers can distinguish "forced past a clean
    /// check" from "was going to rebuild anyway").
    #[test]
    fn force_keeps_rebuild_rows_unchanged() {
        assert_eq!(
            decide_skew(RecordState::Stale, StoreTag::Present, true),
            SkewDecision::Rebuild
        );
        assert_eq!(
            decide_skew(RecordState::Fresh, StoreTag::Gone, true),
            SkewDecision::Rebuild
        );
        assert_eq!(
            decide_skew(RecordState::Stale, StoreTag::Gone, true),
            SkewDecision::Rebuild
        );
    }

    /// Force is a no-op on row 5: absent + absent builds either way.
    #[test]
    fn force_keeps_build_row_unchanged() {
        assert_eq!(
            decide_skew(RecordState::Absent, StoreTag::Gone, true),
            SkewDecision::Build
        );
    }

    /// Exhaustive sweep: every (record, store, force) triple maps to the §3.4
    /// table + §5.2 flip — a guard against silent matrix edits.
    #[test]
    fn exhaustive_matrix_matches_spec_table() {
        use RecordState::*;
        use SkewDecision::*;
        use StoreTag::*;
        let expected = [
            ((Fresh, Present, false), Skip),
            ((Stale, Present, false), Rebuild),
            ((Fresh, Gone, false), Rebuild),
            ((Stale, Gone, false), Rebuild),
            ((Absent, Present, false), TrustAndRecord),
            ((Absent, Gone, false), Build),
            ((Fresh, Present, true), RebuildForced),
            ((Stale, Present, true), Rebuild),
            ((Fresh, Gone, true), Rebuild),
            ((Stale, Gone, true), Rebuild),
            ((Absent, Present, true), RebuildForced),
            ((Absent, Gone, true), Build),
        ];
        for ((record, store, force), want) in expected {
            assert_eq!(
                decide_skew(record, store, force),
                want,
                "record={record:?} store={store:?} force={force}"
            );
        }
    }
}
