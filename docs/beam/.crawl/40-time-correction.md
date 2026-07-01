# Crawl: erts/time_correction.html (focused)
- seed_url: https://www.erlang.org/doc/apps/erts/time_correction.html
- canonical_url: https://www.erlang.org/doc/apps/erts/time_correction.html
- family: Erlang/OTP ERTS docs
- fetch: 200
- otp_version: OTP 29.0.2 (erts 17.0.2)
- feeds_docs: timers.md, runtime-debugging.md

## Purpose
ERTS reference chapter defining how the Erlang runtime models time, how timers
fire, and how the runtime reacts to OS clock changes (NTP, leap seconds, manual
clock changes, suspended systems). It is the canonical source for the
`+C` (time warp mode) and `+c` (time correction) `erl` flags, and for the rule
that **multi-time-warp mode requires all executing code to be time-warp-safe**.
Directly governs timer accuracy guarantees and the deprecation of `erlang:now/0`.

## Monotonic time vs wall clock
Two distinct clocks coexist in the runtime:

- **Erlang Monotonic Time** — monotonically increasing time, the runtime's
  internal "time engine". Retrieved via `erlang:monotonic_time/0,1`. Increases
  since an unspecified epoch. Its accuracy/precision depends on OS monotonic
  time, OS system time, and the active time warp mode. On systems without OS
  monotonic time it guarantees only monotonicity. **All timers (receive ... after,
  BIF timers, `timer` module) are triggered relative to Erlang monotonic time.**
  Even Erlang system time is derived from it: `Erlang system time = Erlang
  monotonic time + time offset`.

- **Erlang System Time** — the runtime's view of POSIX time (the "wall clock").
  Retrieved via `erlang:system_time/0,1`. May or may not be accurate / aligned
  with OS system time. The runtime works toward alignment; depending on the time
  warp mode this is achieved by letting Erlang system time perform a **time warp**
  (a leap forwards or backwards where the difference of values before/after does
  not equal actual elapsed time).

- **Time Offset** — the value added to Erlang monotonic time to obtain Erlang
  system time. Retrieved via `erlang:time_offset/0`. Managed differently per
  time warp mode: fixed (no_time_warp), finalized once (single_time_warp), or
  freely variable at any time (multi_time_warp).

Key distinction: monotonic time never warps (when time correction is on);
system time may warp. Use monotonic time for measuring elapsed time and ordering
events; use system time only when you need a POSIX/wall-clock value.

## Time warp modes + time-warp-safe code requirement
Set via `erl +C [no_time_warp|single_time_warp|multi_time_warp]`. Time
correction set separately via `erl +c [true|false]`.

- **No Time Warp Mode** (`+C no_time_warp`) — time offset fixed at runtime start,
  never changes. Default prior to OTP 26 (ERTS 14.0), and the only behavior
  prior to OTP 18 (ERTS 7.0). Because the offset cannot change, time correction
  must align Erlang system time with OS system time by **adjusting the frequency
  of the Erlang monotonic clock smoothly** — introducing a deliberate frequency
  error up to ~1% that shows up in *all* time measurements. If time correction
  is disabled, Erlang monotonic time **freezes** when OS system time leaps
  backwards (can last years/decades) and leaps forward when OS time leaps
  forward.

- **Single Time Warp Mode** (`+C single_time_warp`) — backward-compatibility
  mode for embedded systems booting before OS time is corrected. Two phases:
  *Preliminary* (offset fixed from preliminary OS time; no attempt to align
  system times) and *Final* (begun by `erlang:system_flag(time_offset,
  finalize)`, callable once; offset is adjusted so Erlang system time aligns
  with OS system time, possibly warping forward; thereafter behaves as
  no_time_warp). Requirements: the warp on finalization must be **forwards**
  (OS time must be set ≤ actual POSIX time before starting erl), and OS system
  time must be correct at finalization. Code executing in both phases must be
  time-warp-safe.

- **Multi-Time Warp Mode** (`+C multi_time_warp`) — **preferred configuration**
  and **default as of OTP 26 (ERTS 14.0)** (in combination with time
  correction). Better performance, scalability, accuracy, and precision on
  almost all platforms. The time offset can change at any time without
  limitation, so Erlang system time may warp forwards or backwards at any time.
  Because alignment is done via the offset, time correction can keep the
  Erlang monotonic clock frequency as correct as possible (no ~1% error).
  If time correction is disabled, monotonic time leaps forward on forward OS
  leaps and only briefly stops (does not freeze for extended periods) on
  backward OS leaps.

**Time-warp-safe code requirement (verbatim warnings):**
- Single time warp mode: *"To use this mode, ensure that all Erlang code that
  will execute in both phases is time warp safe. Code executing only in the
  final phase does not have to be able to cope with the time warp."*
- Multi-time warp mode: *"To use this mode, ensure that all Erlang code that
  will execute on the runtime system is time warp safe."*
- Extended Time Functionality note: *"As of Erlang/OTP 26 (ERTS 14.0) the
  multi time warp mode is enabled by default. This assumes that all code
  executing on the system is time warp safe."*

**What "time-warp-safe" means:** code that can handle a time warp of Erlang
system time. `erlang:now/0` is the canonical time-warp-*unsafe* primitive: on a
backward warp its returned values freeze (apart from microsecond increments)
until OS time catches up — a freeze that can last years/decades/longer. Uses of
`erlang:now/0` that do not read time are technically safe but are all
suboptimal for performance/scalability and should be replaced. The chapter's
summary line: *"To sum up this section: Do not use erlang:now/0."*

## Timers / accuracy (the #timers section, verbatim)
> All timers are triggered relative Erlang monotonic time. All timers currently
> have millisecond resolution both in the API and internally in the runtime
> system. That is, resolution (as well as precision and accuracy) will not be
> higher than millisecond. If Erlang monotonic time has a lower resolution than
> millisecond, the timer resolution will be lower than millisecond as well.
>
> Timers can only be triggered on whole milliseconds since the runtime system
> start. A timer is not allowed to trigger before the timeout time given by the
> user. That is, assuming that the system is not heavily loaded, a timer will
> typically be triggered in the range [T, T+1) milliseconds when the user has
> given the timeout time T. If the system is heavily loaded, it may take an even
> longer time until a timer is triggered.

Implications for timer guidance:
- Timer resolution is capped at **1 ms** (API + internal). Sub-millisecond
  timers are impossible via the standard timer API.
- Timers fire on **whole milliseconds since runtime start** (the scheduler
  time-wheel is millisecond-granular and driven by Erlang monotonic time).
- **Never fire early**: a timer with timeout T fires in `[T, T+1)` ms under
  normal load; under heavy load it fires later, never earlier.
- Because timers are relative to **monotonic** time, they are immune to wall-
  clock warps (NTP adjustments, leap seconds, manual clock changes) — a timer
  scheduled for T ms will still fire ~T ms of *monotonic* time later regardless
  of what happens to Erlang/OS system time. This is the reason time-warp-safe
  code is compatible with timers.
- Timer accuracy degrades if Erlang monotonic time itself is low-resolution or
  if the monotonic clock frequency is deliberately skewed (the ~1% error in
  `no_time_warp` mode affects timer pacing).

## Strict rules
1. **Default to multi-time-warp + time correction** (OTP 26+ default). Only fall
   back to `no_time_warp`/`single_time_warp` when running legacy time-warp-
   unsafe code that cannot be fixed.
2. **All code must be time-warp-safe under multi-time-warp.** This is a hard
   precondition stated as a Warning in the chapter.
3. **Never use `erlang:now/0`** — not for time, not for unique values, not for
   event ordering, not for RNG seeding. Replace per the "How to Work with the
   New API" table (see Verbatim quotes).
4. **Measure elapsed time with `erlang:monotonic_time/0,1`** and ordinary
   subtraction (native time unit); convert with `erlang:convert_time_unit/3` or
   request a unit via `erlang:monotonic_time/1` (may lose accuracy/precision).
5. **Order events with `erlang:unique_integer([monotonic])`**; if you also need
   the time of the event, store `{erlang:monotonic_time(),
   erlang:unique_integer([monotonic])}` (monotonic time MUST be the first/most-
   significant element). Add `erlang:time_offset/0` as a third element if you
   need the actual Erlang system time at the event and the offset may change.
6. **Retrieve Erlang system (wall-clock) time with `erlang:system_time/1`** (or
   `erlang:timestamp/0` for the old `now/0`-style format). Never use `now/0`.
7. **Do not disable time correction** (`+c false`) — it causes monotonic time to
   warp/stop/freeze and yields bad scalability, performance, and measurements.
8. **Timer expectations**: resolution ≤ 1 ms; fires in `[T, T+1)` ms when
   unloaded, later when loaded, never before T. Do not design for sub-ms or
   early-fire timers.
9. **Single-time-warp only for embedded pre-NTP boot**: OS time must be set ≤
   actual POSIX time before starting erl, and must be correct at finalization;
   finalize exactly once with `erlang:system_flag(time_offset, finalize)`.

## Verbatim quotes
- Timers (see dedicated section above).
- *"Time warp safe code can handle a time warp of Erlang system time."*
- *"erlang:now/0 behaves bad when Erlang system time warps. When Erlang system
  time does a time warp backwards, the values returned from erlang:now/0 freeze
  (if you disregard the microsecond increments made because of the actual call)
  until OS system time reaches the point of the last value returned by
  erlang:now/0. This freeze can continue for a long time. It can take years,
  decades, and even longer until the freeze stops."*
- *"All uses of erlang:now/0 are not necessarily time warp unsafe. If you do not
  use it to get time, it is time warp safe. However, all uses of erlang:now/0
  are suboptimal from a performance and scalability perspective."*
- *"Current Erlang system time is determined by adding the current Erlang
  monotonic time with current time offset. The time offset is managed
  differently depending on which time warp mode you use."*
- No Time Warp Mode: *"As the time offset is not allowed to change, time
  correction must adjust the frequency of the Erlang monotonic clock to align
  Erlang system time with OS system time smoothly. A significant downside of
  this approach is that we on purpose will use a faulty frequency on the Erlang
  monotonic clock if adjustments are needed. This error can be as large as 1%.
  This error will show up in all time measurements in the runtime system."*
- Multi-Time Warp Mode: *"Multi-time warp mode in combination with time
  correction is the preferred configuration. ... As of OTP 26 (ERTS 14.0) this
  is also the default."*
- Multi-Time Warp Mode warning: *"To use this mode, ensure that all Erlang code
  that will execute on the runtime system is time warp safe."*
- Single-Time Warp Mode warning: *"To use this mode, ensure that all Erlang code
  that will execute in both phases is time warp safe."*
- Time Correction: *"You typically never want to disable time correction."*
- New API summary: *"To sum up this section: Do not use erlang:now/0."*
- New API Do/Don't table (condensed):
  - Retrieve system time: Don't `erlang:now/0`; Do `erlang:system_time/1` (or
    `erlang:timestamp/0`).
  - Measure elapsed time: Don't `now/0` + `timer:now_diff/2`; Do
    `erlang:monotonic_time/0` + subtraction.
  - Order events: Don't `now/0`; Do `erlang:unique_integer([monotonic])`.
  - Order events + time: Don't `now/0`; Do `{monotonic_time(),
    unique_integer([monotonic])}` (optionally add `time_offset/0` as 3rd elem).
  - Unique name: Don't `now/0`; Do `erlang:unique_integer/0` (`[positive]` if
    needed).
  - Seed RNG: Don't `now/0`; Do a combination of `monotonic_time/0`,
    `time_offset/0`, `unique_integer/0`, and other functionality.

## Version notes
- Page meta: **OTP 29.0.2 (erts 17.0.2)**.
- Extended time functionality introduced in **Erlang/OTP 18 (ERTS 7.0)**.
- **Multi-time-warp became the default in OTP 26 (ERTS 14.0)** (previously
  `no_time_warp` was default). This is a behavior change: systems with old
  time-warp-unsafe code must now explicitly start with `+C no_time_warp` (or
  `single_time_warp` if partially safe).
- Source markdown on GitHub at tag OTP-29.0.2:
  `erts/doc/guides/time_correction.md`.
- A compatibility helper exists at `assets/time_compat.erl`
  (`$ERL_TOP/erts/example/time_compat.erl`) for code supporting both old and
  new OTP releases.

## Discovered links

### Relevant (crawl later)
- erl_cmd.html — `erl` command-line flags reference (`+C`, `+c`); needed for
  runtime-debugging/flags docs. (https://www.erlang.org/doc/apps/erts/erl_cmd.html)
- erlang.html — `erlang` BIF reference (`monotonic_time/0,1`, `system_time/0,1`,
  `time_offset/0,1`, `timestamp/0`, `unique_integer/0,1`, `now/0`,
  `system_flag(time_offset, ...)`, `system_info(time_correction|time_warp_mode|
  time_offset|os_monotonic_time_source|os_system_time_source|start_time|
  end_time)`). (https://www.erlang.org/doc/apps/erts/erlang.html)
- ../../apps/stdlib/timer.html — `timer` module reference (already crawled as
  16-timer.md; `now_diff/2` referenced here). (https://www.erlang.org/doc/apps/stdlib/timer.html)
- ../../apps/kernel/os.html — `os:system_time/0,1` reference.
  (https://www.erlang.org/doc/apps/kernel/os.html)
- assets/time_compat.erl — example compatibility module for old/new OTP.
  (https://www.erlang.org/doc/apps/erts/assets/time_compat.erl)
- communication.html — referenced from this chapter (Distributed Erlang).
  (https://www.erlang.org/doc/apps/erts/communication.html)
- match_spec.html — referenced from this chapter (match specifications).
  (https://www.erlang.org/doc/apps/erts/match_spec.html)

### Skipped
- In-page anchors (`#timers`, `#time-warp-modes`, `#multi-time-warp-mode`,
  `#no-time-warp-mode`, `#single-time-warp-mode`, `#time-warp-safe-code`,
  `#new-time-api`, `#terminology`, `#ut1`, `#utc`, `#posix-time`,
  `#time-resolution`, `#time-precision`, `#time-accuracy`, `#time-warp`,
  `#os-system-time`, `#os-monotonic-time`, `#erlang-system-time`,
  `#erlang-monotonic-time`, `#how-to-work-with-the-new-api`,
  `#support-of-both-new-and-old-otp-releases`, `#Dos_and_Donts`) — internal.
- POSIX/UTC external spec links (pubs.opengroup.org) — out of scope (standards
  refs, not Erlang docs).
- llms.txt, https://erlang.org, https://www.ericsson.com,
  https://github.com/elixir-lang/ex_doc — site chrome.
- GitHub source link (github.com/erlang/otp/.../time_correction.md) — raw
  source mirror, not a doc page.
