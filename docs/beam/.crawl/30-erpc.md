# Crawl: kernel/erpc.html
- seed_url: https://www.erlang.org/doc/apps/kernel/erpc.html
- canonical_url: https://www.erlang.org/doc/apps/kernel/erpc.html
- family: Erlang/OTP kernel module docs
- fetch: 200
- otp_version: OTP 29.0.2 (kernel 11.0.2)
- feeds_docs: distribution.md

## Purpose
`erpc` (Enhanced Remote Procedure Call) provides services similar to Remote
Procedure Calls: call a function on a remote node and collect the answer. Used
for collecting information on a remote node, or for running a function with
specific side effects on a remote node.

It is an "enhanced subset" of the operations provided by the `rpc` module.
Enhanced in the sense that it makes it possible to distinguish between returned
value, raised exceptions, and other errors. `erpc` also has better performance
and scalability than the original `rpc` implementation. The current `rpc`
module will utilize `erpc` in order to also provide these properties when
possible.

For an `erpc` operation to succeed, the remote node also needs to support
`erpc`. Typically only ordinary Erlang nodes as of OTP 23 have `erpc` support.

It is up to the user to ensure that correct code to execute via `erpc` is
available on the involved nodes.

Note: blocking signaling over distribution (see Processes chapter of Erlang
Reference Manual) can cause timeouts in `erpc` to be significantly delayed.

## Key functions (exact arities)
- `call(Node, Fun)` — since OTP 23. Equivalent to `call(Node, Fun, #{timeout => infinity})`.
- `call(Node, Fun, TimeoutOrOptions)` — since OTP 23. `TimeoutOrOptions :: timeout_time() | call_options()`. Equivalent to `erpc:call(Node, erlang, apply, [Fun,[]], #{timeout => Timeout})`. May raise all same exceptions as `call/5` plus `{erpc, badarg}` if `Fun` is not a fun of zero arity.
- `call(Node, Module, Function, Args)` — since OTP 23. Equivalent to `call(Node, Module, Function, Args, #{timeout => infinity})`.
- `call(Node, Module, Function, Args, TimeoutOrOptions)` — since OTP 23. Evaluates `apply(Module, Function, Args)` on `Node` and returns `Result`. `TimeoutOrOptions` can be a timeout time or a call options map (options map since OTP 28.0).
- `cast(Node, Fun)` — since OTP 23. Equivalent to `erpc:cast(Node,erlang,apply,[Fun,[]])`.
- `cast(Node, Module, Function, Args)` — since OTP 23. Evaluates `apply(Module, Function, Args)` on `Node`. No response delivered. Returns immediately after cast request sent. Failures beside bad arguments silently ignored.
- `check_response(Message, RequestId)` — since OTP 23. Returns `{response, Result} | no_response`. Check if a message is a response to a call request previously made via `send_request/4`.
- `check_response(Message, RequestIdCollection, Delete)` — since OTP 25. Returns `{{response, Result}, Label, NewRequestIdCollection} | no_response | no_request`.
- `multicall(Nodes, Fun)` — since OTP 23.
- `multicall(Nodes, Fun, TimeoutOrOptions)` — since OTP 23.
- `multicall(Nodes, Module, Function, Args)` — since OTP 23.
- `multicall(Nodes, Module, Function, Args, TimeoutOrOptions)` — since OTP 23. Performs multiple call operations in parallel on multiple nodes. Result is a list aligned with `Nodes`; each item `{ok, Result}` or `{Class, ExceptionReason}`.
- `multicast(Nodes, Fun)` — since OTP 23. Equivalent to `erpc:multicast(Nodes,erlang,apply,[Fun,[]])`.
- `multicast(Nodes, Module, Function, Args)` — since OTP 23. No response delivered. Returns immediately after cast requests sent. Failures beside bad arguments silently ignored.
- `receive_response(RequestId)` — since OTP 23. Equivalent to `receive_response(RequestId, infinity)`.
- `receive_response(RequestId, Timeout)` — since OTP 23. Receive a response to a call request previously made via `send_request/4`. On timeout, request is abandoned and `{erpc, timeout}` raised.
- `receive_response(RequestIdCollection, Timeout, Delete)` — since OTP 25. Returns `{Result, Label, NewRequestIdCollection} | no_request`. Abandons requests at timeout (future responses ignored).
- `wait_response(RequestId)` — since OTP 23. Equivalent to `wait_response(RequestId, 0)` (poll). Returns `{response, Result} | no_response`.
- `wait_response(RequestId, WaitTime)` — since OTP 23. Wait or poll. Returns `{response, Result} | no_response`. Does NOT raise `{erpc, timeout}`; returns `no_response` instead. Valid to continue waiting.
- `wait_response(RequestIdCollection, WaitTime, Delete)` — since OTP 25. Returns `{{response, Result}, Label, NewRequestIdCollection} | no_response | no_request`. Does NOT abandon requests at timeout (unlike `receive_response/3`).
- `send_request(Node, Fun)` — since OTP 23. Equivalent to `erpc:send_request(Node, erlang, apply, [Fun, []])`.
- `send_request(Node, Module, Function, Args)` (`send_request/4`) — since OTP 23. Send an asynchronous call request; returns `RequestId` to pass to `receive_response/2`, `wait_response/2`, or `check_response/2`.
- `send_request(Node, Module, Function, Args, Label, RequestIdCollection)` (`send_request/6`) — since OTP 25. Associates `Label` with request id and adds to collection.
- `reqids_new()` — since OTP 25. Returns new empty request identifier collection.
- `reqids_add(RequestId, Label, RequestIdCollection)` — since OTP 25.
- `reqids_size(RequestIdCollection)` — since OTP 25. Returns count.
- `reqids_to_list(RequestIdCollection)` — since OTP 25. Returns `[{RequestId, Label}]`.

## erpc vs rpc differences (why prefer erpc)
- `erpc` is an "enhanced subset" of `rpc` operations.
- Enhanced: makes it possible to distinguish between returned value, raised exceptions, and other errors. (`rpc:call` conflates these — a thrown/raised exception on the remote node is re-raised locally, making it hard to tell a remote error from a local one.)
- `erpc` has better performance and scalability than the original `rpc` implementation.
- The current `rpc` module will utilize `erpc` in order to also provide these properties when possible (i.e., `rpc` is now layered on `erpc`).
- `erpc` requires the remote node to support `erpc` (ordinary Erlang nodes as of OTP 23). `rpc` works against older nodes too.
- `erpc` error semantics are explicit: failures of the `erpc` operation itself are reported as `{error, {erpc, Reason}}` (caught form) or raised as `error` exception `{erpc, Reason}`, distinct from exceptions raised by the applied function (which are reported as `{throw, ...}`, `{exit, {exception, ...}}`, `{error, {exception, Reason, StackTrace}}`).
- `multicall` returns per-node results as `{ok, Value} | {Class, ExceptionReason}` rather than crashing the whole call.
- `call()` can utilize a selective receive optimization removing the need to scan the message queue from the beginning; `send_request()/receive_response()` combination cannot.

## Error semantics + options
`call()` only returns if the applied function successfully returned without
raising any uncaught exceptions, the operation did not time out, and no failures
occurred. In all other cases an exception is raised. Exceptions by class:

- `throw` — applied function called `throw(Value)` and did not catch. Reason = `Value`.
- `exit`:
  - `{exception, ExitReason}` — applied function called `exit(ExitReason)` and did not catch.
  - `{signal, ExitReason}` — process that applied the function received an exit signal and terminated with `ExitReason`.
- `error`:
  - `{exception, ErrorReason, StackTrace}` — runtime error while applying; `StackTrace` limited to applied function and functions called by it.
  - `{erpc, ERpcErrorReason}` — the erpc operation itself failed. Common `ERpcErrorReason`:
    - `badarg` — `Node`/`Module`/`Function` not atom, `Args` not a list (properness not verified client-side), invalid `Timeout`.
    - `noconnection` — connection to `Node` lost or could not be established. Function may or may not be applied.
    - `system_limit` — e.g. failure to create a process on remote node.
    - `timeout` — operation timed out. Function may or may not be applied.
    - `notsup` — remote node does not support this `erpc` operation.

If the erpc operation fails but it is unknown whether the function is/will be
applied (timeout or connection loss), the caller will not receive any further
information about the result if/when the applied function completes.

`caught_call_exception()` type (the caught form used by `multicall`):
```
{throw, Throw} |
{exit, {exception, Reason}} |
{error, {exception, Reason, StackTrace}} |
{exit, {signal, Reason}} |
{error, {erpc, Reason}}
```

### Options (`call_options()`, since OTP 28.0)
```
#{timeout => Timeout :: timeout_time(),
  always_spawn => AlwaysSpawn :: boolean()}
```
- `timeout` — upper time limit for call operations. Default: `infinity`.
- `always_spawn` — if `true`, the `apply()` will always be performed in a freshly spawned process. If `false` (default), the calling process may be used instead if possible. When `false`, you cannot make assumptions about which process performs the `apply()`.

### `timeout_time()` (since OTP 23.0)
```
0..4294967295 | infinity | {abs, integer()}
```
- `0..4294967295` — relative timeout in ms.
- `infinity` — never times out.
- `{abs, Timeout}` — absolute Erlang monotonic time in ms; times out when `erlang:monotonic_time(millisecond) >= Timeout`. Not allowed to identify a time further than 4294967295 ms into the future. Handy for deadlines over a collection of requests.

### `receive_response` vs `wait_response` at timeout
- `receive_response/2,3` abandons the request(s) at timeout → no future response will ever be received; raises `{erpc, timeout}`.
- `wait_response/2,3` does NOT abandon; returns `no_response` and it is valid to keep waiting. Does not raise `{erpc, timeout}`.

## Strict rules
- Remote node must support `erpc` (OTP 23+ ordinary nodes).
- User must ensure correct code to execute is available on involved nodes.
- `Args` list is NOT verified to be a proper list at the client side.
- When `always_spawn` is `false` (default), you cannot make assumptions about which process performs the `apply()` — may be the calling process itself or a freshly spawned process.
- A response might be consumed upon an `{erpc, badarg}` exception and if so will be lost forever.
- All request identifiers in a `RequestIdCollection` must correspond to requests made by the calling process via `send_request/4` or `send_request/6`.
- `check_response/2` / `wait_response/2` do not raise `{erpc, timeout}` (only `receive_response` does).
- Blocking signaling over distribution can cause `erpc` timeouts to be significantly delayed.
- `multicall` may have already sent some requests when a `badarg` failure occurs (function may or may not be applied on some nodes).
- Deleting handled associations from a collection is "not for free"; a collection containing already-handled requests can still be used, but without deletion the calls cannot detect "no more outstanding requests" via `no_request`.

## Verbatim quotes
- "This is an enhanced subset of the operations provided by the rpc module. Enhanced in the sense that it makes it possible to distinguish between returned value, raised exceptions, and other errors. erpc also has better performance and scalability than the original rpc implementation."
- "In order for an erpc operation to succeed, the remote node also needs to support erpc . Typically only ordinary Erlang nodes as of OTP 23 have erpc support."
- "Note that it is up to the user to ensure that correct code to execute via erpc is available on the involved nodes."
- "The call() function only returns if the applied function successfully returned without raising any uncaught exceptions, the operation did not time out, and no failures occurred. In all other cases an exception is raised."
- "If the erpc operation fails, but it is unknown if the function is/will be applied (that is, a timeout or a connection loss), the caller will not receive any further information about the result if/when the applied function completes."
- "If the always_spawn option is false (which is the default), you cannot make any assumptions about the process that will perform the apply() . It may be the calling process itself, or a freshly spawned process."
- "call() can utilize a selective receive optimization which removes the need to scan the message queue from the beginning in order to find a matching message. The send_request()/receive_response() combination can, however, not utilize this optimization."
- "The difference between receive_response/3 and wait_response/3 is that receive_response/3 abandons the requests at timeout so that any potential future responses are ignored, while wait_response/3 does not."
- "Note that a response might have been consumed uppon an {erpc, badarg} exception and if so, will be lost for ever."
- "Blocking signaling can, for example, cause timeouts in erpc to be significantly delayed."

## Version notes
- Module introduced in OTP 23.0 (all core functions `call`, `cast`, `multicall`, `multicast`, `send_request/4`, `receive_response/1,2`, `wait_response/1,2`, `check_response/2`, types `request_id/0`, `request_id_collection/0`, `stack_item/0`, `timeout_time/0`).
- `check_response/3`, `receive_response/3`, `wait_response/3`, `send_request/6`, `reqids_add/3`, `reqids_new/0`, `reqids_size/1`, `reqids_to_list/1` added in OTP 25.0 (request identifier collection API).
- `call_options()` type and options-map form of `TimeoutOrOptions` for `call/3,5` and `multicall/3,5` (including `always_spawn`) added in OTP 28.0.
- Page reflects OTP 29.0.2 / kernel 11.0.2.
- `{abs, Timeout}` absolute monotonic-time form of `timeout_time()` available since OTP 23.0.

## Discovered links

### Relevant (crawl later)
- (none beyond this page) — page is a leaf module reference. Related modules referenced only by name in prose: `rpc` (https://www.erlang.org/doc/apps/kernel/rpc.html), Erlang Reference Manual Processes chapter (https://www.erlang.org/doc/reference_manual/processes.html) for "Blocking Signaling Over Distribution".

### Skipped
- In-page anchors: `#call/2`, `#call/3`, `#call/4`, `#call/5`, `#cast/2`, `#cast/4`, `#check_response/2`, `#check_response/3`, `#multicall/2`, `#multicall/3`, `#multicall/4`, `#multicall/5`, `#multicast/2`, `#multicast/4`, `#receive_response/1`, `#receive_response/2`, `#receive_response/3`, `#reqids_add/3`, `#reqids_new/0`, `#reqids_size/1`, `#reqids_to_list/1`, `#send_request/2`, `#send_request/4`, `#send_request/6`, `#wait_response/1`, `#wait_response/2`, `#wait_response/3`, `#summary`, `#functions`, `#types`, `#t:call_options/0`, `#t:caught_call_exception/0`, `#t:request_id/0`, `#t:request_id_collection/0`, `#t:stack_item/0`, `#t:timeout_time/0`.
- GitHub source links to `erlang/otp` `erpc.erl` (OTP-29.0.2 tag) — source view, not docs.
- `https://erlang.org`, `https://www.ericsson.com`, `https://github.com/elixir-lang/ex_doc`, `/assets/css/algolia-typeahead.css`, "View llms.txt", "Download ePub version".
