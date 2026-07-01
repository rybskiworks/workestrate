# Runtime Environment: `file` and `os` (kernel)

## Purpose

`file` (kernel) is the file system interface: open/read/write/close, directory
listing, file metadata (`#file_info{}`), copy/rename/delete, cwd management,
and open modes (`raw`/`binary`/`consult`/`delayed_write`/`read_ahead`/
`compressed`/`encoding`). `IoDevice` is a pid (except `raw` mode, which returns
an `fd()`). Several functions are version-gated (`read_file/2` OTP 27+,
`delete/2` OTP 24+).

`os` (kernel) is the OS-specific interface: `getenv`/`putenv`/`env`,
`cmd/1,2` (with caveats vs ports), `system_time`/`perf_counter`/`timestamp`,
`type`/`version`. Corrections applied: the `os` module does NOT have
`listenv/0`, `monotonic_time/0,1`, or `locale/0,1`. Monotonic time lives in
`erlang:monotonic_time/0,1`; `os` time BIFs reflect the OS wall clock / hardware
counter, which can jump — they are explicitly NOT monotonic.

## Sources used

- `.crawl/44-file.md` — https://www.erlang.org/doc/apps/kernel/file.html
  (PRIMARY — `file` module: `read_file`/`write_file`/`open`/`close`/`read`/
  `read_line`/`write`/`list_dir`/`make_dir`/`del_dir`/`delete`/`copy`/`rename`/
  `read_file_info`/`get_cwd`/`datasync`/`sync`, open modes, IoDevice-is-a-pid,
  version gates). Page documents OTP 29.0.2 (kernel 11.0.2).
- `.crawl/45-os.md` — https://www.erlang.org/doc/apps/kernel/os.html
  (PRIMARY — `os` module: `cmd/1,2`, `env/0`, `getenv/0,1,2`, `putenv/2`,
  `unsetenv/1`, `find_executable/1,2`, `getpid/0`, `perf_counter/0,1`,
  `system_time/0,1`, `timestamp/0`, `type/0`, `version/0`, `set_signal/2`).

Corrections: `listenv/0` does NOT exist (list-all is `env/0` since OTP 24.0 or
`getenv/0`); `monotonic_time/0,1` is NOT in `os` (use
`erlang:monotonic_time/0,1`); `locale/0,1` is NOT present (OTP 29.0.2 /
kernel 11.0.2). `os` time BIFs reflect the OS wall clock / hardware counter,
which can jump — they are explicitly NOT monotonic.

## Core guidance

### `file` module — key functions

- `read_file/1` -> `{ok, Binary} | {error, Reason}` — `Binary` is ALWAYS a binary (never a list).
- `read_file/2` (OTP 27+) -> same; `Opts :: [read_file_option()]` where `read_file_option() :: raw`.
- `write_file/2` -> `ok | {error, Reason}`; `Bytes :: iodata()`; overwrites existing.
- `write_file/3` -> same; `Modes :: [mode()]`; `binary` and `write` are implicit (do not pass).
- `consult/1` -> `{ok, Terms} | {error, Reason}`; reads Erlang terms separated by `.`. NOTE: `file:consult/1` is listed as Potentially Unsafe (see secure_coding / common-mistakes.md) — it constructs arbitrary terms/atoms from file data.
- `open/2` -> `{ok, IoDevice} | {error, Reason}`.
- `close/1` -> `ok | {error, Reason}` (can return old write error if `delayed_write`).
- `read/2` -> `{ok, Data} | eof | {error, Reason}`; binary if `binary` mode, else list.
- `read_line/1` -> `{ok, Data} | eof | {error, Reason}`; line includes trailing LF; CRLF collapsed to LF.
- `write/2` -> `ok | {error, Reason}`; `Bytes :: iodata()`; only way to write to `raw` files.
- `list_dir/1` -> `{ok, Filenames} | {error, Reason}`; excludes raw-filename entries; unsorted.
- `make_dir/1` — does NOT create missing parents (use `filelib:ensure_dir/1`).
- `del_dir/1` — directory must be empty.
- `delete/1` / `delete/2` (OTP 24+) — `Opts :: [delete_option()]` where `delete_option() :: raw`.
- `copy/2,3` -> `{ok, BytesCopied} | {error, Reason}`.
- `rename/2` — destination must be a full filename, not just a directory.
- `read_file_info/1,2` -> `{ok, #file_info{}}` — record from `kernel/include/file.hrl`.
- `get_cwd/0,1`, `datasync/1` (fdatasync), `sync/1` (fsync).

### IoDevice is a pid

> "IoDevice is really the pid of the process that handles the file. This process monitors the process that originally opened the file (the owner process). If the owner process terminates, the file is closed and the process itself terminates too."

For `raw` mode the returned `fd()` is a file descriptor, not a pid.

### `file:open/2` modes

- `read` — file must exist.
- `write` — created if absent; if exists and not combined with `read`, file is TRUNCATED.
- `append` — created if absent; every write goes to end.
- `exclusive` — "This option does not guarantee exclusiveness on file systems not supporting O_EXCL properly, such as NFS."
- `raw` — "Allows faster access to a file, as no Erlang process is needed to handle the file." Limitations: "The functions in the io module cannot be used... Only the Erlang process that opened the file can use it. A remote Erlang file server cannot be used." "Especially if read_line/1 is to be used on a raw file, it is recommended to combine this option with option {read_ahead, Size} as line-oriented I/O is inefficient without buffering."
- `binary` — "Read operations on the file return binaries rather than lists."
- `{delayed_write, Size, Delay}` — "the result of write/2 calls can prematurely be reported as successful, and if a write error occurs, the error is reported as the result of the next file operation... close/1 can return {error, enospc}... close/1 must probably be called again, as the file is still open."
- `{read_ahead, Size}` / `read_ahead` — read buffering; highly recommended for `raw` + `read_line/1`.
- `compressed` — gzip; must combine with `read` OR `write` (not both).
- `{encoding, Encoding}` — automatic Unicode translation; NOT allowed on `raw` files.
- `ram` — `File` must be `iodata()`; returns `fd()` operating on in-memory data.
- `sync` — enables POSIX `O_SYNC`.
- `directory` — allows `open` on directories.
- Backwards-compat: "In previous versions of file, modes were specified as one of the atoms read, write, or read_write instead of a list. This is still allowed for reasons of backwards compatibility, but is not to be used for new code. Also note that read_write is not allowed in a mode list."

### Atomicity

> "File operations are only guaranteed to appear atomic when going through the same file server. A NIF or other OS process may observe intermediate steps on certain operations on some operating systems, eg. renaming an existing file on Windows, or write_file_info/2 on any OS at the time of writing."

### `os` module — key functions

- `cmd/1,2` — "Executes Command in a command shell of the target OS, captures the standard output and standard error of the command, and returns this result as a string." Options: `max_size` (OTP 20.2.3+), `exception_on_failure` (OTP 28.0+). Caveats: buffers ALL output (not streaming); "in some cases, standard output of a command when called from another program can differ, compared with the standard output of the command when called directly from an OS command shell." Prefer ports for structured/streaming interaction.
- `env/0` (OTP 24.0) — list of all env vars as `{VarName,Value}` 2-tuples. "Consider using env/0 for a nicer 2-tuple format."
- `getenv/0` — all env vars as `"VarName=Value"` strings.
- `getenv/1` -> `Value | false`; `getenv/2` (OTP 18.0) -> `Value | DefaultValue`.
- `putenv/2` -> `true`; `unsetenv/1` (OTP R16B03) -> `true`.
- `find_executable/1,2` -> absolute filename or `false`.
- `getpid/0` — emulator process id as string.
- `system_time/0,1` (OTP 18.0) — "This time is not a monotonically increasing time." `system_time/1` ≡ `erlang:convert_time_unit(os:system_time(), native, Unit)`.
- `perf_counter/0,1` (OTP 19.0) — "two consecutive calls to the function are not guaranteed to be monotonic, though it most likely will be."
- `timestamp/0` — "Returns the current OS system time in the same format as erlang:timestamp/0."
- `type/0` — `{Osfamily, Osname}`; "Think twice before using this function. Use module filename if you want to inspect or build filenames in a portable way. Avoid matching on atom Osname."
- `version/0` — "Think twice before using this function. If you still need to use it, always call os:type() first."
- `set_signal/2` (OTP 20.0).

### Corrections (os module)

- `listenv/0` does NOT exist. The list-all function is `env/0` (2-tuple form, OTP 24.0) or `getenv/0` (`"VarName=Value"` string form).
- `monotonic_time/0,1` is NOT in the `os` module. Monotonic time lives in `erlang:monotonic_time/0,1`. `os` only exposes OS system time (`system_time`, `timestamp`), which is explicitly NOT monotonic.
- `locale/0,1` is NOT present in this module (OTP 29.0.2 / kernel 11.0.2).

## Practical rules

- Use `read_file/1` for whole-file reads (returns binary); use `open/2` + `read/2`/`read_line/1` for streaming.
- For `raw` + `read_line/1`, always combine with `{read_ahead, Size}`.
- `raw` mode: only `read/2`, `read_line/1`, `write/2` work; `io` module functions CANNOT be used; only the opening process can use the fd.
- `delayed_write`: write success can be reported prematurely; errors surface on the NEXT file op; `close/1` may return `{error, enospc}` and must be called again.
- `make_dir/1` does NOT create missing parents — use `filelib:ensure_dir/1`.
- `del_dir/1` requires the directory to be empty.
- `rename/2`: destination must be a full filename; renaming open files is not allowed on most platforms.
- `read_file/2` and `delete/2` `raw` option bypasses the file server — useful during early boot.
- Avoid `open/2` for NFS-mounted files, FIFOs, devices — can hang IO threads forever.
- Null characters (integer 0) are NOT allowed in filenames.
- `os:system_time/0,1` is NOT monotonic — do not use for measuring intervals; use `erlang:monotonic_time` or `os:perf_counter`.
- `os:cmd/1,2` buffers ALL output — prefer ports for streaming/structured interaction.
- Use `os:env/0` (OTP 24.0) for the nicer 2-tuple format over `os:getenv/0`.
- Do NOT use `os:type/0` for filename logic — use the `filename` module.
- All `os` args must be valid per `file:native_name_encoding()`; null chars disallowed; `badarg` raised otherwise.

## Review checklist

- [ ] Is `read_file/1` used for whole-file reads (not `open`+`read`+`close`)?
- [ ] Is `{read_ahead, Size}` combined with `raw` + `read_line/1`?
- [ ] Are `io` module functions avoided on `raw` files?
- [ ] Is `delayed_write` error handling correct (check `close/1` return, re-call if needed)?
- [ ] Is `filelib:ensure_dir/1` used before `make_dir/1` for nested paths?
- [ ] Is `os:system_time` NOT used for interval measurement (use `erlang:monotonic_time` or `os:perf_counter`)?
- [ ] Is `os:cmd/1,2` avoided for streaming/structured external interaction (use ports)?
- [ ] Is `os:env/0` (not `os:getenv/0`) used for the 2-tuple format where available?
- [ ] Are `os:type/0`/`os:version/0` avoided for filename/portability logic?

## Implementation checklist

- [ ] Use `read_file/1` for whole-file reads; `open/2` + `read/2` for streaming.
- [ ] Pair `file:open/2` with `file:close/1` in a `try/after`.
- [ ] Use `[read, binary]` or `[write, binary]` for binary I/O.
- [ ] Use `{delayed_write, Size, Delay}` only where late error reporting is acceptable.
- [ ] Use `file:read_file_info/1` for metadata; `-include_lib("kernel/include/file.hrl").` for the record.
- [ ] Use `os:putenv/2` / `os:getenv/1,2` for env var access; `os:env/0` for listing.
- [ ] Use `erlang:monotonic_time/0,1` for interval measurement; `os:perf_counter/0,1` for high-res OS counter.

## Runtime / debugging checklist

- [ ] `file:get_cwd/0` to check the file server's cwd.
- [ ] `file:read_file_info/1` to inspect file type, size, access, mtime.
- [ ] `file:native_name_encoding/0` to check filename encoding mode (latin1 vs utf8).
- [ ] `os:type/0` / `os:version/0` only for diagnostics, not for program logic.
- [ ] `os:getpid/0` for the emulator OS process id.
- [ ] Watch for `close/1` returning `{error, enospc}` with `delayed_write` — file is still open, call again.

## Validation hooks

- `read_file/1` returns `{ok, Binary}` or `{error, Reason}` — always pattern-match both.
- `open/2` returns `{ok, IoDevice}` or `{error, Reason}` — always pattern-match both.
- `close/1` return value must be checked (especially with `delayed_write`).
- `os:cmd/2` with `exception_on_failure => true` (OTP 28.0+) throws on non-zero exit.
- `os:getenv/1` returns `false` if the variable is unset — handle explicitly.

## Examples

Whole-file read:

```erlang
{ok, Bin} = file:read_file("config.json").
```

Whole-file write:

```erlang
ok = file:write_file("out.bin", <<"data">>).
```

Streaming read with raw + read_ahead:

```erlang
{ok, Fd} = file:open("log.txt", [read, raw, binary, {read_ahead, 65536}]),
{ok, Line} = file:read_line(Fd),
ok = file:close(Fd).
```

File info:

```erlang
{ok, #file_info{size = Sz, type = Type}} = file:read_file_info("path").
```

Env var access:

```erlang
case os:getenv("HOME") of
    false -> {error, no_home};
    Home -> {ok, Home}
end.
```

Interval measurement (DO NOT use `os:system_time`):

```erlang
%% DO: use erlang:monotonic_time for intervals
T0 = erlang:monotonic_time(),
%% ... work ...
Elapsed = erlang:convert_time_unit(erlang:monotonic_time() - T0, native, millisecond).

%% DO NOT: os:system_time is NOT monotonic
%% T0 = os:system_time(),
%% ... work ...
%% Elapsed = os:system_time() - T0.  %% can go backwards!
```

## Common mistakes

- Using `os:system_time/0,1` for interval measurement — it is NOT monotonic and can jump.
- Looking for `os:listenv/0` — it does NOT exist; use `os:env/0` or `os:getenv/0`.
- Looking for `os:monotonic_time/0,1` — it is NOT in the `os` module; use `erlang:monotonic_time/0,1`.
- Using `io` module functions on a `raw` file — they cannot be used.
- Forgetting `{read_ahead, Size}` with `raw` + `read_line/1` — inefficient line I/O.
- Not checking `close/1` return with `delayed_write` — write errors surface late.
- Using `make_dir/1` for nested paths without `filelib:ensure_dir/1`.
- Using `os:cmd/1,2` for streaming output — it buffers everything.
- Using `os:type/0` for filename logic instead of the `filename` module.
- Opening NFS-mounted files, FIFOs, or devices with `file:open/2` — can hang IO threads.

## Strict vs contextual guidance

Strict:

- `os:system_time/0,1` is NOT monotonic — never use for interval measurement.
- `raw` mode: `io` module functions CANNOT be used; only `read/2`, `read_line/1`, `write/2`.
- `delayed_write`: `close/1` return must be checked and re-called if `{error, _}`.
- `make_dir/1` does NOT create missing parents.
- `del_dir/1` requires an empty directory.
- Null characters are NOT allowed in filenames.
- `{encoding, ...}` is NOT allowed on `raw` files.

Contextual:

- `delayed_write` vs synchronous writes (performance vs error-timing).
- `compressed`/`{zstd, ...}` for compressed file I/O.
- `os:cmd/1,2` vs ports for one-shot commands (convenience vs control).
- `os:perf_counter/0,1` vs `erlang:monotonic_time/0,1` for high-res timing.

## Policy decisions for individual repos

- Whether `delayed_write` is permitted (late error reporting trade-off).
- Whether `raw` mode is the default for performance-critical file I/O.
- Whether `os:cmd/1,2` is banned in favor of `open_port/2` with `{spawn_executable, _}`.
- Default time source for interval measurement (`erlang:monotonic_time` vs `os:perf_counter`).
- Whether `file:consult/1` is permitted on untrusted files (security: Potentially Unsafe).

## Related docs

- `common-mistakes.md` — `os:cmd/1,2` caveats, `file:consult/1` security, binary handling.
- `binaries.md` — `iodata()`/`binary()` types used by `file:write/2`, `file:read_file/1`.
- `ports-io.md` — `open_port/2` with `{spawn_executable, _}` as the safer alternative to `os:cmd/1,2`.
- `logger-and-config.md` — `sys.config` file format, config composition.
- `releases.md` — embedded mode, boot scripts, `sys.config` location.
- `ets-data.md` — DETS uses `file:name()` type.

## Related skills

- `beam-applications-releases`
- `beam-logger-config`
