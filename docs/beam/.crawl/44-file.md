# Crawl: kernel/file.html (focused)
- seed_url: https://www.erlang.org/doc/apps/kernel/file.html
- canonical_url: https://www.erlang.org/doc/apps/kernel/file.html
- family: Erlang/OTP kernel module docs
- fetch: 200
- otp_version: OTP 29.0.2 (kernel 11.0.2)
- feeds_docs: runtime-environment.md (or ports-io.md)

## Purpose
`file` is the kernel module providing an interface to the file system. It exposes
file open/read/write/close, directory listing, file metadata (file_info record),
copy/rename/delete, and cwd management. Operations are only guaranteed to appear
atomic when going through the same file server; a NIF or other OS process may
observe intermediate steps (e.g. renaming an existing file on Windows, or
`write_file_info/2` on any OS at time of writing).

Filename encoding: VM operates in `latin1` or `utf8` mode (query via
`native_name_encoding/0`). In `utf8` mode filenames may contain Unicode >255 and
are converted to native encoding (UTF-8 on Unix, UTF-16 on Windows). Default:
Windows/macOS/Android use `utf8`; Unix uses `utf8` if terminal supports UTF-8 else
`latin1`. Override with `+fnl` / `+fnu` at `erl` startup. Binaries passed as
filenames are treated as raw filenames ("as is") unless using `set_cwd/1`.

## Key functions (exact arities)
- `read_file/1`            -> `{ok, Binary} | {error, Reason}` (Binary always a binary)
- `read_file/2` (OTP 27+)   -> same; `Opts :: [read_file_option()]` where `read_file_option() :: raw`
- `write_file/2`            -> `ok | {error, Reason}`; `Bytes :: iodata()`; overwrites existing
- `write_file/3`            -> same; `Modes :: [mode()]`; `binary` and `write` are implicit (do not pass)
- `consult/1`               -> `{ok, Terms} | {error, Reason}`; reads Erlang terms separated by `.`
- `open/2`                  -> `{ok, IoDevice} | {error, Reason}`
- `close/1`                 -> `ok | {error, Reason}` (mostly ok; can return old write error if `delayed_write`)
- `read/2`                  -> `{ok, Data} | eof | {error, Reason}`; `Number` = chars/bytes
- `read_line/1`             -> `{ok, Data} | eof | {error, Reason}`; line incl trailing LF, CRLF->LF
- `write/2`                 -> `ok | {error, Reason}`; `Bytes :: iodata()`; only way to write to `raw` files
- `list_dir/1`              -> `{ok, Filenames} | {error, Reason}`; excludes raw-filename entries; unsorted
- `make_dir/1`              -> `ok | {error, Reason}`; does NOT create missing parents
- `del_dir/1`               -> `ok | {error, Reason}`; directory must be empty
- `delete/1`                -> `ok | {error, Reason}`; equiv `delete(Filename, [])`
- `delete/2` (OTP 24+)      -> same; `Opts :: [delete_option()]` where `delete_option() :: raw`
- `copy/2`                  -> `{ok, BytesCopied} | {error, Reason}`; equiv `copy(Src,Dst,infinity)`
- `copy/3`                  -> same; `ByteCount :: non_neg_integer() | infinity`
- `rename/2`                -> `ok | {error, Reason}`; destination filename must be specified (not just dir)
- `read_file_info/1`        -> `{ok, FileInfo} | {error, Reason}`; equiv `read_file_info(File, [])`
- `read_file_info/2` (OTP R15B+) -> same; `Opts :: [file_info_option()]`
- `get_cwd/0`               -> `{ok, Dir} | {error, Reason}`; cwd of file server
- `get_cwd/1`               -> `{ok, Dir} | {error, Reason}`; `Drive` like `"c:"`; `enotsup` on Unix
- `datasync/1` (OTP R14B+)  -> `ok | {error, Reason}`; flushes OS buffers, skips some metadata (fdatasync)
- `sync/1`                  -> `ok | {error, Reason}`; flushes OS buffers incl metadata (fsync)

`IoDevice` is really the pid of the process handling the file; that process
monitors the owner process and closes/terminates on owner exit. For `raw` mode
the returned `fd()` is a file descriptor, not a pid.

## open/2 modes
`Modes :: [mode() | ram | directory]`. `mode()` type:
```
read | write | append | exclusive | raw | binary |
{delayed_write, Size, Delay} | delayed_write |
{read_ahead, Size} | read_ahead | compressed | compressed_one |
{zstd, zstd:compress_parameters() | zstd:decompress_parameters()} |
{encoding, unicode:encoding()} | sync
```
- `read`     - file must exist, opened for reading.
- `write`    - created if absent; if exists and not combined with `read`, file is TRUNCATED.
- `append`   - created if absent; every write goes to end of file.
- `exclusive`- opened for writing; created if absent; `{error, eexist}` if exists. NOT guaranteed on NFS.
- `raw`      - faster, no Erlang process; limitations: `io` module unusable (use `read/2`,`read_line/1`,`write/2`); only opening process can use it; no remote file server; combine with `{read_ahead, Size}` for line I/O.
- `binary`   - read ops return binaries rather than lists.
- `{delayed_write, Size, Delay}` - buffers writes until Size bytes or Delay ms; flushes before non-write ops. Write errors may be reported late (e.g. `close/1` returns `{error, enospc}`); close may need re-calling.
- `delayed_write` - defaults (~64KB, 2s).
- `{read_ahead, Size}` / `read_ahead` - read buffering; highly recommended for `raw` + `read_line/1`.
- `compressed` - gzip; must combine with `read` OR `write` (not both). File size from `read_file_info` won't match readable bytes.
- `{zstd, Opts}` - zstd compression; same read/write constraint.
- `compressed_one` - read one gzip member; only with `read`.
- `{encoding, Encoding}` - automatic Unicode translation of disk data; `write/2`/`read/2` stay byte-oriented. Values: `latin1` (default), `unicode`/`utf8`, `utf16`/`{utf16,big}`, `{utf16,little}`, `utf32`/`{utf32,big}`, `{utf32,little}`. NOT allowed on `raw` files. Changeable on the fly via `io:setopts/2`.
- `ram`      - `File` must be `iodata()`; returns `fd()` operating on in-memory data.
- `sync`     - enables POSIX `O_SYNC` (or `FILE_FLAG_WRITE_THROUGH` on Windows); `{error, enotsup}` if unsupported.
- `directory`- allows `open` on directories.

Backwards-compat: atoms `read`/`write`/`read_write` as a single mode (not a list)
still allowed but deprecated for new code; `read_write` is NOT allowed in a mode list.

Typical `open/2` errors: `enoent`, `eacces`, `eisdir`, `enotdir`, `enospc`.

## Return conventions (iodata/binary)
- `read_file/1,2` -> ALWAYS returns `{ok, Binary}` (a binary, never a list).
- `write_file/2,3` -> accepts `iodata()` for `Bytes`.
- `read/2` -> `Data :: string() | binary()`: binary if file opened with `binary` mode, else list. Shorter than requested if EOF reached. `Number` denotes number of CHARACTERS (not bytes) for non-latin1 encodings; fails with `{no_translation, unicode, latin1}` if data >255 and encoding != latin1 (use `io:get_chars/3` instead).
- `read_line/1` -> `Data :: string() | binary()`: binary if `binary` mode else list. Line includes trailing LF; CRLF collapsed to LF; CR before LF silently ignored. Same `{no_translation,...}` failure mode as `read/2` (use `io:get_line/2` for non-latin1).
- `write/2` -> accepts `iodata()`; for non-latin1 encoding each byte may expand to 1-4 bytes; use `io:put_chars/2` to write `unicode:chardata()`.
- `list_dir/1` -> `Filenames :: [filename()]` (strings); excludes raw-filename entries. `list_dir_all/1` includes raw filenames and returns `filename_all()` (string | binary).
- `copy/2,3` -> `{ok, BytesCopied}`; `BytesCopied` may be < `ByteCount` if source EOF. When both args are filenames, opens with `[read, binary]` and `[write, binary]` prepended respectively.
- `read_file_info/1,2` -> `{ok, #file_info{}}` record from `kernel/include/file.hrl` (`-include_lib("kernel/include/file.hrl").`).

## Strict rules
- File operations are only atomic through the same file server; NIFs/other OS processes may see intermediate steps. `write_file_info/2` is non-atomic on any OS at time of writing.
- `raw` mode: `io` module functions CANNOT be used; only `read/2`, `read_line/1`, `write/2`. Only the opening process can use the fd. No remote file server access.
- `raw` + `read_line/1` is inefficient without `{read_ahead, Size}` - combine them for line-oriented raw reading.
- `delayed_write`: write success can be reported prematurely; errors surface on the NEXT file op; `close/1` may return `{error, enospc}` and the file stays open - call `close/1` again.
- `exclusive` does NOT guarantee exclusiveness on filesystems without proper `O_EXCL` (e.g. NFS). Don't depend on it unless FS is known-safe.
- `make_dir/1` does NOT create missing parent directories (use `filelib:ensure_dir/1` for that).
- `del_dir/1` requires the directory to be empty (`eexist` otherwise).
- `rename/2`: destination must be a full filename, not just a directory; renaming open files is not allowed on most platforms (`eacces`); `exdev` if cross-filesystem.
- `read_file_info` with `raw` option breaks atomicity guarantees (can race with concurrent `write_file_info/1,2`); has no effect when given an IoDevice (use `open/2` with `raw` to get an fd first).
- `{encoding, ...}` is NOT allowed on `raw` files.
- `position/2` offsets are in BYTES not characters; for non-latin1 encodings positioning only valid at known character boundaries.
- `read_file/2` and `delete/2` `raw` option bypasses the file server - useful during early boot before file server is registered.
- Avoid `open/2` for NFS-mounted files, FIFOs, devices - can hang IO threads forever; break such access out to a port program.
- `set_cwd/1` expects binaries encoded per `native_name_encoding/0` (unlike other functions treating binaries as raw filenames); `no_translation` if latin1 binary in unicode mode.
- Null characters (integer 0) are NOT allowed in filenames (not even at end).

## Verbatim quotes
- "File operations are only guaranteed to appear atomic when going through the same file server. A NIF or other OS process may observe intermediate steps on certain operations on some operating systems, eg. renaming an existing file on Windows, or write_file_info/2 on any OS at the time of writing."
- "raw - Allows faster access to a file, as no Erlang process is needed to handle the file. However, a file opened in this way has the following limitations: The functions in the io module cannot be used... Only the Erlang process that opened the file can use it. A remote Erlang file server cannot be used."
- "Especially if read_line/1 is to be used on a raw file, it is recommended to combine this option with option {read_ahead, Size} as line-oriented I/O is inefficient without buffering."
- "binary - Read operations on the file return binaries rather than lists."
- "The functions read/2, pread/3, and read_line/1 are the only ways to read from a file opened in raw mode (although they work for normally opened files, too)."
- "This function [write/2] is the only way to write to a file opened in raw mode (although it works for normally opened files too)."
- "When this option [delayed_write] is used, the result of write/2 calls can prematurely be reported as successful, and if a write error occurs, the error is reported as the result of the next file operation, which is not executed... close/1 can return {error, enospc}... close/1 must probably be called again, as the file is still open."
- "This option [exclusive] does not guarantee exclusiveness on file systems not supporting O_EXCL properly, such as NFS."
- "If the option raw is set, the file server is not called. This can be useful in particular during the early boot stage when the file server is not yet registered, to still be able to delete local files." (delete/2)
- "If the option raw is set, the file server is not called and only information about local files is returned. Note that this will break this module's atomicity guarantees as it can race with a concurrent call to write_file_info/1,2." (read_file_info/2)
- "While this function [open/2] can be used to open any file, we recommend against using it for NFS-mounted files, FIFOs, devices, or similar since they can cause IO threads to hang forever."
- "IoDevice is really the pid of the process that handles the file. This process monitors the process that originally opened the file (the owner process). If the owner process terminates, the file is closed and the process itself terminates too."
- "In previous versions of file, modes were specified as one of the atoms read, write, or read_write instead of a list. This is still allowed for reasons of backwards compatibility, but is not to be used for new code. Also note that read_write is not allowed in a mode list."
- "datasync ... resembles fsync but it does not update some of the metadata of the file, such as the access time. On some platforms this function has no effect."
- "Lines are defined to be delimited by the linefeed (LF, \n) character, but any carriage return (CR, \r) followed by a newline is also treated as a single LF character (the carriage return is silently ignored). The line is returned including the LF, but excluding any CR immediately followed by an LF."

## Version notes
- Page documents OTP 29.0.2 (kernel 11.0.2).
- `read_file/2` introduced OTP 27.0.
- `delete/2` introduced OTP 24.0.
- `datasync/1` since OTP R14B.
- `read_file_info/2`, `read_link_info/2`, `write_file_info/2` since OTP R15B.
- `list_dir_all/1`, `read_link_all/1` since OTP R16B.
- `del_dir_r/1` since OTP 23.0.
- `native_name_encoding/0` since OTP R14B01.
- `{zstd, Opts}` mode and `compressed_one` mode present in OTP 29.
- Source: https://github.com/erlang/otp/blob/OTP-29.0.2/lib/kernel/src/file.erl

## Discovered links

### Relevant (crawl later)
- https://www.erlang.org/doc/apps/stdlib/io.html (io module - get_chars/get_line/put_chars/setopts; raw-mode alternative for unicode)
- https://www.erlang.org/doc/apps/stdlib/filename.html (filename manipulation; ensure_dir for make_dir parents)
- https://www.erlang.org/doc/apps/stdlib/unicode.html (encoding/chardata types; latin1_binary)
- https://www.erlang.org/doc/apps/stdlib/epp.html#encoding (file encoding comment directive for consult/eval/script)
- https://www.erlang.org/doc/apps/stdlib/unicode_usage.html#notes-about-raw-filenames (raw filename semantics)
- https://www.erlang.org/doc/apps/stdlib/zstd.html (zstd compress/decompress parameters)
- https://www.erlang.org/doc/apps/erts/erlang.html (iodata/iolist/binary/char/string base types)

### Skipped
- GitHub source links (https://github.com/erlang/otp/blob/OTP-29.0.2/lib/kernel/src/file.erl#L...) - source, not docs
- CSS/asset links (dist/html-erlang-*.css)
- https://erlang.org (site root)
- In-page anchors (#functions, #position/2, #t:*/0 type anchors)
