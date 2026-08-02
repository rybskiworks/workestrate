# Crawl: stdlib/timer.html
- seed_url: https://www.erlang.org/doc/apps/stdlib/timer.html
- canonical_url: https://www.erlang.org/doc/apps/stdlib/timer.html
- family: Erlang/OTP stdlib module docs
- fetch: 200
- otp_version: OTP 29.0.2 (stdlib v8.0.1)
- feeds_docs: timers.md (or processes-and-messages.md)

## Purpose
`timer` provides time-related utility functions: scheduling one-shot and
repeating sends/applies, sending exit/kill signals after a delay, sleeping,
and measuring execution time. Unless otherwise stated, time is always measured
in **milliseconds**. All timer functions return immediately, regardless of work
done by another process. Time-outs are not exact but are *at least* as long as
requested.

## Key functions (exact arities)
One-shot timers (not linked to any process; removed at time-out or via cancel/1):
- `apply_after/2`            — `apply_after(Time, Function)` (since OTP 27.0)
- `apply_after/3`            — `apply_after(Time, Function, Arguments)` (since OTP 27.0)
- `apply_after/4`            — `apply_after(Time, Module, Function, Arguments)`
- `send_after/2`            — `send_after(Time, Message)` ≡ `send_after(Time, self(), Message)`
- `send_after/3`            — `send_after(Time, Destination, Message)` (no /4 exists)
- `exit_after/2`            — `exit_after(Time, Reason1)` ≡ `exit_after(Time, self(), Reason)`
- `exit_after/3`            — `exit_after(Time, Target, Reason1)`
- `kill_after/1`            — `kill_after(Time)` ≡ `exit_after(Time, self(), kill)`
- `kill_after/2`            — `kill_after(Time, Target)` ≡ `exit_after(Time, Target, kill)`

Interval timers (linked to the process the timer performs its task for):
- `apply_interval/2`        — `apply_interval(Time, Function)` (since OTP 27.0)
- `apply_interval/3`        — `apply_interval(Time, Function, Arguments)` (since OTP 27.0)
- `apply_interval/4`        — `apply_interval(Time, Module, Function, Arguments)`
- `apply_repeatedly/2`      — (since OTP 27.0) waits for spawned process to finish before next
- `apply_repeatedly/3`      — (since OTP 27.0)
- `apply_repeatedly/4`      — (since OTP 26.0)
- `send_interval/2`         — `send_interval(Time, Message)` ≡ `send_interval(Time, self(), Message)`
- `send_interval/3`         — `send_interval(Time, Destination, Message)` (no /4 exists)

Note on apply_interval vs apply_repeatedly:
- `apply_interval/*` spawns a *new* process at each interval **irrespective of
  whether a previously spawned process has finished** — can run many concurrent
  processes; extreme example `timer:apply_interval(1, timer, sleep, [1000])`
  x1000 → up to 1,000,000 processes (exceeds default system limit).
- `apply_repeatedly/*` waits for the spawned process to finish before starting
  the next; if execution time > Time, next is spawned immediately after the
  current finishes (system tries to catch up).

Cancellation / control:
- `cancel/1`                — `cancel(TRef) -> {ok, cancel} | {error, Reason}`
- `start/0`                 — starts the timer server (normally started dynamically)

Time helpers (return milliseconds):
- `seconds/1`, `minutes/1`, `hours/1`, `hms/3`

Measurement / misc:
- `sleep/1`                 — `sleep(Time) -> ok`; `Time` may be `infinity`; does NOT return immediately
- `tc/1`                    — `tc(Fun)` ≡ `tc(Fun, microsecond)` (since OTP R14B03)
- `tc/2`                    — `tc(Fun, Arguments)` OR `tc(Fun, TimeUnit)` (TimeUnit form since OTP 26.0)
- `tc/3`                    — `tc(Module, Function, Arguments)` OR `tc(Fun, Arguments, TimeUnit)`
- `tc/4`                    — `tc(Module, Function, Arguments, TimeUnit)` (since OTP 26.0); uses `erlang:monotonic_time/0`
- `now_diff/2`              — `now_diff(T2, T1)` microseconds, T1/T2 from `erlang:timestamp/0` or `os:timestamp/0`

## TimerRef / cancellation semantics
- Successful timer evaluations return `{ok, TRef}` (some return `{ok, TRef}` or
  `{error, Reason}`); `TRef` is a unique timer reference.
- `TRef` is an Erlang term whose contents must not be changed.
- `cancel/1` cancels any previously requested time-out; returns `{ok, cancel}`
  or `{error, Reason}` when `TRef` is not a timer reference.
- A timer can always be removed by `cancel/1`.
- Interval timers (apply_interval/*, apply_repeatedly/*, send_interval/2,3) are
  **linked to the process** the timer performs its task for.
- One-shot timers (apply_after/*, send_after/2,3, exit_after/2,3, kill_after/1,2)
  are **not linked to any process** — removed only at time-out or via `cancel/1`.
- Functions passed to apply_after/2,3, apply_interval/2,3, apply_repeatedly/2,3
  (or M:F:A forms) execute in a **freshly-spawned process**, so `self/0` inside
  them returns that spawned process's pid, NOT the caller's. Capture the target
  pid in a variable before scheduling if you need to message the caller.

## timer module vs erlang:* BIF timers (bottleneck note)
- "Creating timers using `erlang:send_after/3` and `erlang:start_timer/3` is
  more efficient than using the timers provided by this module."
- Historically the `timer` module ran all timers through a **single timer
  server process**, which was a scalability bottleneck at high timer frequency
  (the classic "timer module bottleneck").
- "However, the timer module has been improved in OTP 25, making it more
  efficient and less susceptible to being overloaded." (See the Timer Module
  section in the Efficiency Guide.)
- Guidance: prefer BIF timers `erlang:send_after/3` and `erlang:start_timer/3`
  for high-frequency / many-timer workloads; the `timer` module is acceptable
  for low-frequency scheduling and is now (OTP 25+) far less of a bottleneck.
- `send_after/3` Destination can be local/remote pid, registered-name atom, or
  `{RegName, Node}` tuple.
- Accuracy model: time-outs are not exact but are **at least** as long as
  requested (timer-wheel / scheduler-driven; see Time and Time Correction in
  Erlang ERTS User's Guide, Timers section).

## Strict rules
- Time units are milliseconds unless stated otherwise (`now_diff/2` is
  microseconds; `tc/*` default microsecond, configurable via `erlang:time_unit()`).
- Never mutate a `TRef` term.
- Always pair interval/one-shot timers you want to stop early with `cancel/1`.
- Do not rely on `self/0` inside an apply_* callback returning the caller's pid.
- `apply_interval/*` does NOT wait for the spawned process — guard against
  unbounded process growth when interval < execution time.
- `sleep/1` blocks the calling process (does not return immediately); `infinity`
  suspends forever.
- Before OTP 25, `timer:sleep/1` rejected integers > 16#ffffffff (2^32-1);
  since OTP 25 arbitrarily high integers are accepted.

## Verbatim quotes
- "This module provides useful functions related to time. Unless otherwise
  stated, time is always measured in milliseconds. All timer functions return
  immediately, regardless of work done by another process."
- "Successful evaluations of the timer functions give return values containing a
  timer reference, denoted TRef. By using cancel/1, the returned reference can be
  used to cancel any requested action. A TRef is an Erlang term, which contents
  must not be changed."
- "The time-outs are not exact, but are at least as long as requested."
- "Creating timers using erlang:send_after/3 and erlang:start_timer/3 is more
  efficient than using the timers provided by this module. However, the timer
  module has been improved in OTP 25, making it more efficient and less
  susceptible to being overloaded."
- "An interval timer ... is linked to the process to which the timer performs its
  task."
- "A one-shot timer ... is not linked to any process. Hence, such a timer is
  removed only when it reaches its time-out, or if it is explicitly removed by a
  call to cancel/1."
- "The functions given to apply_after/2, apply_after/3, apply_interval/2,
  apply_interval/3, apply_repeatedly/2, and apply_repeatedly/3, or denoted by
  Module, Function and Arguments ... are executed in a freshly-spawned process,
  and therefore calls to self/0 in those functions will return the Pid of this
  process, which is different from the process that called timer:apply_*."
- (apply_interval warning) "If the execution time of the spawned process is, on
  average, greater than the given Time, multiple such processes will run at the
  same time ... this may even lead to exceeding the number of allowed processes."
- (sleep note) "Before OTP 25, timer:sleep/1 did not accept integer timeout
  values greater than 16#ffffffff, that is, 2^32-1. Since OTP 25, arbitrarily
  high integer values are accepted."

## Version notes
- Page version: OTP 29.0.2, stdlib v8.0.1.
- `time()` type: since OTP 28.0 (`-nominal time() :: non_neg_integer()`).
- `tref()`: opaque type (no public `timer()` record is exposed by this module;
  the reference type is `tref()`).
- `apply_after/2,3`, `apply_interval/2,3`, `apply_repeatedly/2,3`: since OTP 27.0.
- `apply_repeatedly/4`: since OTP 26.0.
- `tc/1`: since OTP R14B03; `tc/2` since OTP R14B; `tc/4` and `tc/2,3` TimeUnit
  forms since OTP 26.0.
- `timer` module efficiency/overload-resistance improvement: OTP 25.
- `timer:sleep/1` accepts arbitrarily large integers since OTP 25.
- NOTE on requested arities: the page exposes `send_after/2,3` and
  `send_interval/2,3` only (no /4 variants exist); `apply_interval/2,3,4` and
  `apply_after/2,3,4` exist (the brief's "apply_interval/3,4" omits the /2
  fun-arg form added in OTP 27.0).

## Discovered links

### Relevant (crawl later)
- https://www.erlang.org/doc/apps/erts/erlang.html#send_after/3   (BIF timer — preferred for high freq)
- https://www.erlang.org/doc/apps/erts/erlang.html#start_timer/3  (BIF timer — preferred for high freq)
- https://www.erlang.org/doc/apps/erts/erlang.html#monotonic_time/0 (basis for tc/4)
- https://www.erlang.org/doc/apps/erts/erlang.html#timestamp/0    (basis for now_diff/2)
- https://www.erlang.org/doc/apps/erts/time_correction.html#timers (Timers section — accuracy/timer-wheel model)
- https://www.erlang.org/doc/system/commoncaveats.html#timer-module (Timer Module bottleneck / Efficiency Guide)
- https://www.erlang.org/doc/system/system_limits.html            (process limit referenced by apply_interval warning)
- https://www.erlang.org/doc/apps/kernel/os.html#timestamp/0      (os:timestamp/0 alt for now_diff)
- https://www.erlang.org/doc/apps/erts/erlang.html#spawn/3         (underlying spawn for apply_*)

### Skipped
- https://www.erlang.org/doc/apps/erts/erlang.html#apply/3
- https://www.erlang.org/doc/apps/erts/erlang.html#self/0
- https://www.erlang.org/doc/apps/erts/erlang.html#t:atom/0
- https://www.erlang.org/doc/apps/erts/erlang.html#t:integer/0
- https://www.erlang.org/doc/apps/erts/erlang.html#t:module/0
- https://www.erlang.org/doc/apps/erts/erlang.html#t:node/0
- https://www.erlang.org/doc/apps/erts/erlang.html#t:non_neg_integer/0
- https://www.erlang.org/doc/apps/erts/erlang.html#t:pid/0
- https://www.erlang.org/doc/apps/erts/erlang.html#t:term/0
- https://www.erlang.org/doc/apps/erts/erlang.html#t:time_unit/0
- https://www.erlang.org/doc/apps/erts/erlang.html#t:timestamp/0
- https://www.erlang.org/doc/apps/kernel/index.html
- https://www.erlang.org/doc/index.html
- https://www.erlang.org/doc/llms.txt
- https://www.erlang.org/doc/stdlib.epub
- https://www.erlang.org/doc/timer.md
- https://www.erlang.org/doc/apps/stdlib/timer.html (self)
- CSS/asset links
