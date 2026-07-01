# Timers

## Purpose

BEAM timers come in two layers: the `timer` module (STDLIB) providing
one-shot/interval send/apply/exit/kill scheduling, sleep, and `tc` measurement;
and the BIF timers (`erlang:send_after/3`, `erlang:start_timer/3`,
`erlang:cancel_timer`, `erlang:read_timer`) which are more efficient and the
preferred choice at scale. This doc covers both, the timer-module bottleneck
caveat, accuracy, and monotonic time. It also covers the ERTS time model
(monotonic vs wall clock, time-warp modes, the time-warp-safe-code requirement)
and the timer-accuracy guarantees from the ERTS Time and Time Correction guide
(crawl 40).

## Sources used

- Crawl `16-timer.md` — stdlib `timer.html` — https://www.erlang.org/doc/apps/stdlib/timer.html
- Crawl `34-commoncaveats.md` — system `commoncaveats.html` — https://www.erlang.org/doc/system/commoncaveats.html
- Crawl `17-erlang-bifs.md` — erts `erlang.html` — https://www.erlang.org/doc/apps/erts/erlang.html (BIF timers)
- Crawl `40-time-correction.md` — erts `time_correction.html` — https://www.erlang.org/doc/apps/erts/time_correction.html

## Core guidance

### The `timer` module

"This module provides useful functions related to time. Unless otherwise stated,
time is always measured in milliseconds. All timer functions return immediately,
regardless of work done by another process." "The time-outs are not exact, but
are at least as long as requested."

One-shot timers (not linked to any process; removed at time-out or via
`cancel/1`):
- `send_after/2` — `send_after(Time, Message)` == `send_after(Time, self(), Message)`.
- `send_after/3` — `send_after(Time, Destination, Message)`. **No `/4` exists in
  the `timer` module.**
- `apply_after/2,3` (OTP 27.0) — `apply_after(Time, Function[, Arguments])`.
- `apply_after/4` — `apply_after(Time, Module, Function, Arguments)`.
- `exit_after/2` — `exit_after(Time, Reason)` == `exit_after(Time, self(), Reason)`.
- `exit_after/3` — `exit_after(Time, Target, Reason)`.
- `kill_after/1` — `kill_after(Time)` == `exit_after(Time, self(), kill)`.
- `kill_after/2` — `kill_after(Time, Target)`.

Interval timers (linked to the process the timer performs its task for):
- `send_interval/2` — `send_interval(Time, Message)` == `send_interval(Time, self(), Message)`.
- `send_interval/3` — `send_interval(Time, Destination, Message)`. **No `/4`.**
- `apply_interval/2,3` (OTP 27.0) — `apply_interval(Time, Function[, Arguments])`.
- `apply_interval/4` — `apply_interval(Time, Module, Function, Arguments)`.
- `apply_repeatedly/2,3` (OTP 27.0), `apply_repeatedly/4` (OTP 26.0) — waits for
  the spawned process to finish before the next interval.

`apply_interval` vs `apply_repeatedly`: `apply_interval/*` spawns a NEW process
at each interval irrespective of whether a previously spawned process has
finished — can run many concurrent processes; `apply_repeatedly/*` waits for the
spawned process to finish before starting the next.

Cancellation / control:
- `cancel(TRef) -> {ok, cancel} | {error, Reason}` — "A timer can always be
  removed by cancel/1."
- `start/0` — starts the timer server (normally started dynamically).

Time helpers: `seconds/1`, `minutes/1`, `hours/1`, `hms/3` (return ms).

Measurement / misc:
- `sleep(Time) -> ok` — `Time` may be `infinity`; does NOT return immediately.
  Since OTP 25, arbitrarily high integers are accepted (pre-OTP 25 capped at
  2^32-1).
- `tc/1` (OTP R14B03), `tc/2,3,4` — measure execution time; `tc/4` (OTP 26.0)
  uses `erlang:monotonic_time/0`. Default unit microsecond; configurable via
  `erlang:time_unit()`.
- `now_diff(T2, T1)` — microseconds; T1/T2 from `erlang:timestamp/0` or
  `os:timestamp/0`.

`TRef` is an opaque `tref()`; "a TRef is an Erlang term, which contents must not
be changed."

### BIF timers (preferred at scale)

`erlang:send_after/3` — `send_after(Time, Dest, Msg) -> TimerRef`. `Dest ::
pid() | atom()`.
`erlang:send_after/4` (OTP 18.0) — adds `Options :: [{abs, Abs :: boolean()}]`.
On expiry, sends `Msg` to `Dest`.
`erlang:start_timer/3` — `start_timer(Time, Dest, Msg) -> TimerRef`. On expiry,
sends `{timeout, TimerRef, Msg}` to `Dest`.
`erlang:start_timer/4` (OTP 18.0) — adds `[{abs, Abs}]`. `{abs, true}` interprets
`Time` as absolute Erlang monotonic time in ms.
`erlang:cancel_timer/1,2` (OTP 18.0) — cancels; returns ms left or `false`.
  Options `{async, Async}`, `{info, Info}`.
`erlang:read_timer/1,2` (OTP 18.0) — reads ms left or `false`. Option
  `{async, Async}`.

"Creating timers using erlang:send_after/3 and erlang:start_timer/3 is more
efficient than using the timers provided by this module." BIF timer `Dest` can be
a local/remote pid, registered-name atom, or `{RegName, Node}` tuple. If `Dest`
is a pid, the timer is auto-canceled when the process dies; if `Dest` is an
atom, the name is looked up at expiry (not auto-canceled).

### The timer-module bottleneck caveat

From the Common Caveats chapter: "The timer module uses a separate single
process to manage the timers." Before OTP 25 this overhead was substantial and
scaled poorly with the number of (especially short-lived) timers. "However, the
timer module has been improved in OTP 25, making it more efficient and less
susceptible to being overloaded." "Still, the timer server remains a single
process, and it may at some point become a bottleneck of an application."

Recommended practice: "Creating timers using `erlang:send_after/3` and
`erlang:start_timer/3`, is more efficient than using the timers provided by the
`timer` module in STDLIB." Note: "The functions in the `timer` module that do not
manage timers (such as `timer:tc/3` or `timer:sleep/1`), do not call the
timer-server process and are therefore harmless."

### Accuracy and monotonic time

Time-outs are not exact but at least as long as requested (timer-wheel /
scheduler-driven; see the Timers section of the ERTS Time and Time Correction
guide — now integrated from crawl 40). `erlang:monotonic_time/0,1` (OTP 18.0) is
the monotonic, time-warp-safe basis for `tc/4` and `{abs, Time}` BIF timers. It
is "monotonically increasing" but not strictly monotonic (consecutive calls may
return the same value). Different runtime instances use different base points —
do not compare monotonic times across nodes. `erlang:system_info(start_time)` /
`end_time` bound the valid absolute-timer interval.

The monotonic-vs-wall-clock distinction (from crawl 40 "Monotonic time vs wall
clock"):

- **Erlang Monotonic Time** — monotonically increasing; the runtime's internal
  "time engine". Retrieved via `erlang:monotonic_time/0,1`. Increases since an
  unspecified epoch. Key fact (verbatim from crawl 40): "All timers (receive ...
  after, BIF timers, `timer` module) are triggered relative to Erlang monotonic
  time." Note: `Erlang system time = Erlang monotonic time + time offset`.
- **Erlang System Time** — the runtime's view of POSIX time (the "wall clock").
  Retrieved via `erlang:system_time/0,1`. May warp (leap forwards or backwards).
- **Time Offset** — the value added to monotonic time to obtain system time.
  Retrieved via `erlang:time_offset/0`. Managed differently per time-warp mode.
- Key distinction: monotonic time never warps (when time correction is on);
  system time may warp. Use monotonic time for measuring elapsed time and
  ordering events; use system time only when you need a POSIX/wall-clock value.

### Time warp modes + time-warp-safe code (crawl 40)

Set via `erl +C [no_time_warp|single_time_warp|multi_time_warp]`; time
correction via `erl +c [true|false]`.

The three modes:

- **No Time Warp Mode** (`+C no_time_warp`) — time offset fixed at runtime start.
  Default prior to OTP 26. Time correction aligns system time by adjusting the
  monotonic clock frequency smoothly — a deliberate frequency error up to ~1%
  that shows up in ALL time measurements. Verbatim from crawl 40: "As the time
  offset is not allowed to change, time correction must adjust the frequency of
  the Erlang monotonic clock to align Erlang system time with OS system time
  smoothly. A significant downside of this approach is that we on purpose will
  use a faulty frequency on the Erlang monotonic clock if adjustments are needed.
  This error can be as large as 1%. This error will show up in all time
  measurements in the runtime system."
- **Single Time Warp Mode** (`+C single_time_warp`) — backward-compatibility
  mode for embedded systems booting before OS time is corrected. Two phases
  (Preliminary/Final); finalized once via
  `erlang:system_flag(time_offset, finalize)`. The warp on finalization must be
  forwards.
- **Multi-Time Warp Mode** (`+C multi_time_warp`) — PREFERRED configuration and
  DEFAULT as of OTP 26 (ERTS 14.0) (in combination with time correction). Better
  performance, scalability, accuracy, precision. The time offset can change at
  any time; system time may warp forwards or backwards. Verbatim from crawl 40:
  "Multi-time warp mode in combination with time correction is the preferred
  configuration. ... As of OTP 26 (ERTS 14.0) this is also the default."

The time-warp-safe-code requirement (verbatim warnings from crawl 40):

- Single time warp mode: "To use this mode, ensure that all Erlang code that
  will execute in both phases is time warp safe. Code executing only in the
  final phase does not have to be able to cope with the time warp."
- Multi-time warp mode: "To use this mode, ensure that all Erlang code that
  will execute on the runtime system is time warp safe."
- Extended Time Functionality note: "As of Erlang/OTP 26 (ERTS 14.0) the multi
  time warp mode is enabled by default. This assumes that all code executing on
  the system is time warp safe."

What "time-warp-safe" means: code that can handle a time warp of Erlang system
time. `erlang:now/0` is the canonical time-warp-UNSAFE primitive: on a backward
warp its returned values freeze (apart from microsecond increments) until OS
time catches up — a freeze that can last years/decades. Verbatim from crawl 40:
"erlang:now/0 behaves bad when Erlang system time warps. When Erlang system
time does a time warp backwards, the values returned from erlang:now/0 freeze
(if you disregard the microsecond increments made because of the actual call)
until OS system time reaches the point of the last value returned by
erlang:now/0. This freeze can continue for a long time. It can take years,
decades, and even longer until the freeze stops." And the summary line
(verbatim): "To sum up this section: Do not use erlang:now/0."

The New API Do/Don't table (condensed, from crawl 40):

- Retrieve system time: Don't `erlang:now/0`; Do `erlang:system_time/1` (or
  `erlang:timestamp/0`).
- Measure elapsed time: Don't `now/0` + `timer:now_diff/2`; Do
  `erlang:monotonic_time/0` + subtraction.
- Order events: Don't `now/0`; Do `erlang:unique_integer([monotonic])`.
- Order events + time: Don't `now/0`; Do
  `{erlang:monotonic_time(), erlang:unique_integer([monotonic])}` (monotonic
  time MUST be the first/most-significant element; add `erlang:time_offset/0`
  as a third element if you need the actual system time and the offset may
  change).
- Unique name: Don't `now/0`; Do `erlang:unique_integer/0` (`[positive]` if
  needed).
- Seed RNG: Don't `now/0`; Do a combination of `monotonic_time/0`,
  `time_offset/0`, `unique_integer/0`, and other functionality.

### Timer accuracy guarantees (crawl 40, verbatim)

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

Implications (from crawl 40):

- Timer resolution is capped at 1 ms (API + internal). Sub-millisecond timers
  are impossible via the standard timer API.
- Timers fire on whole milliseconds since runtime start (the scheduler
  time-wheel is millisecond-granular and driven by Erlang monotonic time).
- Never fire early: a timer with timeout T fires in `[T, T+1)` ms under normal
  load; under heavy load, later — never earlier.
- Because timers are relative to MONOTONIC time, they are immune to wall-clock
  warps (NTP adjustments, leap seconds, manual clock changes). A timer
  scheduled for T ms still fires ~T ms of monotonic time later regardless of
  what happens to system time. This is why time-warp-safe code is compatible
  with timers.
- Timer accuracy degrades if Erlang monotonic time itself is low-resolution or
  if the monotonic clock frequency is deliberately skewed (the ~1% error in
  `no_time_warp` mode affects timer pacing).

## Practical rules

- Time units are milliseconds unless stated (`now_diff/2` microseconds; `tc/*`
  default microsecond, configurable).
- Never mutate a `TRef` term.
- Always pair interval/one-shot timers you want to stop early with `cancel/1`.
- Do not rely on `self/0` inside an `apply_*` callback returning the caller's pid
  (it runs in a freshly-spawned process).
- `apply_interval/*` does NOT wait for the spawned process — guard against
  unbounded process growth when interval < execution time.
- `sleep/1` blocks the calling process; `infinity` suspends forever.
- Prefer BIF timers `erlang:send_after/3` / `erlang:start_timer/3` for
  high-frequency / many-timer workloads.
- `timer:tc/3` and `timer:sleep/1` do NOT use the timer server — safe at scale.
- Use `{abs, true}` BIF timers for deadline-based scheduling across requests.
- All timers (receive-after, BIF timers, `timer` module) fire relative to Erlang
  MONOTONIC time — immune to wall-clock warps.
- Timer resolution is capped at 1 ms; sub-millisecond timers are impossible via
  the standard API.
- A timer with timeout T fires in `[T, T+1)` ms (unloaded); later when loaded;
  NEVER before T.
- Default to multi-time-warp + time correction (OTP 26+ default); only fall back
  to `no_time_warp`/`single_time_warp` for legacy time-warp-unsafe code.
- All code must be time-warp-safe under multi-time-warp (hard precondition).
- Never use `erlang:now/0` — not for time, unique values, event ordering, or RNG
  seeding.
- Measure elapsed time with `erlang:monotonic_time/0,1` + subtraction; order
  events with `erlang:unique_integer([monotonic])`.
- Do not disable time correction (`+c false`) — causes monotonic time to
  warp/stop/freeze.

## Review checklist

- [ ] Are high-frequency timers using BIFs, not the `timer` module?
- [ ] Are interval timers guarded against unbounded process growth?
- [ ] Is `self/0` captured before scheduling `apply_*` callbacks that need it?
- [ ] Are `TRef`s / `TimerRef`s treated as opaque?
- [ ] Are deadlines expressed as `{abs, MonotonicMs}` where appropriate?
- [ ] Is `erlang:now/0` absent from all code (replaced by `monotonic_time/0`,
      `system_time/1`, `unique_integer/0`)?
- [ ] Is all code time-warp-safe (required by the OTP 26+ multi-time-warp
      default)?
- [ ] Are timer expectations aligned with the 1-ms resolution cap and
      `[T, T+1)` ms fire window?
- [ ] Is time correction enabled (`+c true`, the default)?

## Implementation checklist

- [ ] Use `erlang:send_after/3` for one-shot message timers.
- [ ] Use `erlang:start_timer/3` for `{timeout, Ref, Msg}` timers.
- [ ] Use `erlang:cancel_timer/1` to cancel; check the return.
- [ ] Use `timer:send_interval/3` only for low-frequency recurring messages.
- [ ] Use `timer:sleep/1` for blocking delays (no timer server involved).
- [ ] Use `timer:tc/4` with `erlang:monotonic_time/0` for measurement.
- [ ] Measure elapsed time with `erlang:monotonic_time/0` + subtraction (native
      unit); convert with `erlang:convert_time_unit/3`.
- [ ] Order events with `erlang:unique_integer([monotonic])`; store
      `{monotonic_time(), unique_integer([monotonic])}` if time is also needed.
- [ ] Retrieve wall-clock time with `erlang:system_time/1` (or
      `erlang:timestamp/0`), never `now/0`.
- [ ] Use `{abs, true}` BIF timers with
      `erlang:monotonic_time(millisecond)` for deadline scheduling.

## Runtime / debugging checklist

- [ ] `erlang:read_timer/1,2` to inspect remaining time on a BIF timer.
- [ ] `erlang:cancel_timer/2` with `{async, true}` for non-blocking cancel.
- [ ] `timer:cancel/1` to cancel `timer`-module timers.
- [ ] `process_info(Pid, messages)` to spot undelivered `{timeout, _, _}` msgs.
- [ ] Watch the timer-server process mailbox if using `timer` at high frequency.
- [ ] `erlang:system_info(time_warp_mode)` to check the active time-warp mode.
- [ ] `erlang:system_info(time_correction)` to verify time correction is enabled.
- [ ] `erlang:system_info(start_time)` / `end_time` to bound the valid
      absolute-timer interval.

## Validation hooks

- After `erlang:send_after/3`, assert a `reference()` is returned.
- After `erlang:cancel_timer/1`, assert `non_neg_integer()` (ms left) or `false`.
- `timer:tc/4` asserts a measured duration is within expected bounds.
- `erlang:read_timer/1` asserts remaining time is non-negative before expiry.
- Assert a timer with timeout T fires in `[T, T+1)` ms (unloaded); never before T.
- Assert `erlang:monotonic_time/0` is monotonically increasing (consecutive
  calls may return the same value, never decrease).
- Assert `erlang:system_info(time_warp_mode)` returns `multi_time_warp` (OTP 26+
  default).
- Assert no call to `erlang:now/0` exists in the codebase (grep).

## Examples

BIF one-shot message timer (preferred):
```erlang
Ref = erlang:send_after(5000, self(), tick),
receive tick -> erlang:cancel_timer(Ref) end.
```

Absolute deadline timer:
```erlang
Deadline = erlang:monotonic_time(millisecond) + 10_000,
erlang:start_timer(Deadline, self(), deadline, [{abs, true}]).
```

Measure a function (no timer server involved):
```erlang
{Time, Result} = timer:tc(fun() -> expensive_work() end, microsecond).
```

Example — measure elapsed time correctly (no `now/0`):
```erlang
T0 = erlang:monotonic_time(),
Result = do_work(),
Elapsed = erlang:convert_time_unit(erlang:monotonic_time() - T0, native, microsecond).
```

Example — order events with monotonic time + unique integer:
```erlang
Timestamp = {erlang:monotonic_time(), erlang:unique_integer([monotonic])}.
```

## Common mistakes

- Using `timer:send_after/3` at high frequency (single-server bottleneck).
- Expecting `self/0` inside `apply_after` to be the caller's pid.
- Letting `apply_interval` spawn unbounded processes (interval < exec time).
- Mutating or pattern-matching into a `TRef` term.
- Using `erlang:now/0` (deprecated) instead of `erlang:monotonic_time/0` for
  measurement.
- Assuming `timer:sleep/1` uses the timer server (it does not).
- Using `erlang:now/0` for time, unique values, event ordering, or RNG seeding
  (time-warp-unsafe; deprecated; suboptimal performance/scalability).
- Assuming timers fire relative to wall-clock/system time (they fire relative
  to MONOTONIC time).
- Expecting sub-millisecond timer resolution (capped at 1 ms).
- Expecting a timer to fire before its timeout T (fires in `[T, T+1)` ms or
  later, never earlier).
- Running time-warp-unsafe code under multi-time-warp mode (OTP 26+ default)
  without ensuring all code is time-warp-safe.
- Disabling time correction (`+c false`) — causes monotonic time to
  warp/stop/freeze.
- Using `no_time_warp` mode unnecessarily (introduces a ~1% frequency error in
  ALL time measurements).

## Strict vs contextual guidance

Strict:
- `timer:send_after` and `timer:send_interval` have only `/2,3` (no `/4`).
- `apply_interval/*` does not wait for spawned processes to finish.
- `sleep/1` blocks; `infinity` suspends forever.
- BIF timers are more efficient than `timer`-module timers.
- All timers fire relative to Erlang monotonic time (not wall clock).
- Timer resolution is capped at 1 ms; timers never fire before T.
- Multi-time-warp + time correction is the OTP 26+ default; all code must be
  time-warp-safe.
- Never use `erlang:now/0`.
- Do not disable time correction (`+c false`).

Contextual:
- `timer` module is acceptable for low-frequency scheduling (OTP 25+ improved).
- `apply_interval` vs `apply_repeatedly` — choose by whether overlap is wanted.
- `{abs, true}` vs relative time — choose by deadline semantics.
- `no_time_warp` / `single_time_warp` only for legacy time-warp-unsafe code that
  cannot be fixed.
- `erlang:monotonic_time/0` (native unit) + `convert_time_unit/3` vs
  `erlang:monotonic_time/1` (requested unit, may lose accuracy).

## Policy decisions for individual repos

- Threshold above which BIF timers are mandatory (e.g. > 100 timers/sec).
- Whether `timer:send_interval` is permitted in hot paths.
- Default time unit for `tc/*` measurements.
- Whether absolute-monotonic deadlines are the standard for request timeouts.
- Whether multi-time-warp mode is mandatory (OTP 26+ default) or `no_time_warp`
  is forced for legacy code.
- Whether `erlang:now/0` is banned via lint/grep in CI.
- Standard time unit for elapsed-time measurements (native + convert vs
  requested unit).

## Related docs

- `processes-and-messages.md` — `send_after` message delivery.
- `gen-server.md` — gen_server timeout handling.
- `runtime-debugging.md` — `monotonic_time`, `system_info(start_time)`.
- `validation.md` — timer validation hooks.

## Related skills

- `beam-observability-debugging`
- `beam-processes`
- `beam-errors-failures`
- `beam-applications-releases`
- `beam-logger-config`
