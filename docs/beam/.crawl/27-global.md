# Crawl: kernel/global.html
- seed_url: https://www.erlang.org/doc/apps/kernel/global.html
- canonical_url: https://www.erlang.org/doc/apps/kernel/global.html
- family: Erlang/OTP kernel module docs
- fetch: 200
- otp_version: OTP 29.0.2 (kernel v11.0.2)
- feeds_docs: distribution.md
## Purpose
`global` is a global name registration facility for a network of Erlang nodes. It provides three services:
1. Registration of global names (cluster-wide aliases from names to pids).
2. Global locks (cluster-wide mutual exclusion on resource ids).
3. Maintenance of a fully connected network.

Services are controlled by the `global_name_server` process that exists on every node and starts automatically. Registered names are stored in replica global name tables on every node (no central storage point), so name→pid translation is local and fast. Any change to the global name table is automatically propagated to all nodes. The name server subscribes to `nodeup`/`nodedown` messages from `net_kernel` and globally unregisters names whose process or node goes down. It also ensures the network stays fully connected (if N1 connects to N2 which is connected to N3, global connects N1↔N3); if it cannot connect nodes it emits a warning event to the error logger.

As of OTP 25, `global` by default prevents overlapping partitions by actively disconnecting from nodes that report lost connections to other nodes, forming fully connected partitions instead. This is controlled by the `prevent_overlapping_partitions` kernel parameter. Disabling it is strongly discouraged: overlapping partitions can leave `global`'s internal state inconsistent (and affect `mnesia` etc.) in subtle, hard-to-detect ways, and the inconsistency can persist even after partitions rejoin. The fix must be enabled on ALL nodes to work. None of the services are reliably delivered unless both `connect_all` and `prevent_overlapping_partitions` kernel parameters are enabled; API calls do not fail when disabled, they just give unreliable results.

## Key functions (exact arities)
- `register_name(Name, Pid) -> yes | no` — equivalent to `register_name(Name, Pid, fun random_exit_name/3)`.
- `register_name(Name, Pid, Resolve) -> yes | no` — globally associates `Name` with `Pid`; fully synchronous; returns `yes` on success, `no` if name already in use / process already registered.
- `re_register_name(Name, Pid) -> yes` — equivalent to `re_register_name(Name, Pid, fun random_exit_name/3)`.
- `re_register_name(Name, Pid, Resolve) -> yes` — atomically changes the registered `Name` on all nodes to refer to `Pid`.
- `unregister_name(Name) -> _` — removes the globally registered `Name` from the network.
- `whereis_name(Name) -> pid() | undefined` — returns pid for globally registered `Name`, or `undefined` if not registered.
- `send(Name, Msg) -> Pid` — sends `Msg` to pid globally registered as `Name`; exits with `{badarg, {Name, Msg}}` if `Name` not registered.
- `registered_names() -> [Name]` — list of all globally registered names.
- `set_lock(Id) -> boolean()` — equivalent to `set_lock(Id, [node() | nodes()], infinity)`.
- `set_lock(Id, Nodes) -> boolean()` — equivalent to `set_lock(Id, Nodes, infinity)`.
- `set_lock(Id, Nodes, Retries) -> boolean()` — sets a lock on `Nodes` using `id()`; retries until success or `Retries` exhausted; `infinity` blocks forever.
- `del_lock(Id) -> true` — equivalent to `del_lock(Id, [node() | nodes()])`.
- `del_lock(Id, Nodes) -> true` — deletes lock `Id` synchronously.
- `trans(Id, Fun) -> Res | aborted` — equivalent to `trans(Id, Fun, [node() | nodes()], infinity)`.
- `trans(Id, Fun, Nodes) -> Res | aborted` — equivalent to `trans(Id, Fun, Nodes, infinity)`.
- `trans(Id, Fun, Nodes, Retries) -> Res | aborted` — sets lock on `Id` via `set_lock/3`, evaluates `Fun()`, returns result; `aborted` if lock fails; `infinity` default never aborts.
- `sync() -> ok | {error, Reason}` — synchronizes global name server with all nodes known to this node (those returned by `nodes()`); only error reason is `{"global_groups definition error", Error}`.
- `disconnect() -> [node()]` — (since OTP 25.1) disconnect from all other nodes known to `global`; returns list of disconnected nodes; preferred over manual disconnect to avoid overlapping partitions.

Types (not exported):
- `id() :: {ResourceId :: term(), LockRequesterId :: term()}` — lock id.
- `method() :: fun((Name :: term(), Pid :: pid(), Pid2 :: pid()) -> pid() | none)` — resolver function signature.
- `retries() :: non_neg_integer() | infinity`.
- `trans_fun() :: function() | {module(), atom()}`.

## register_name + resolver/conflict-resolution policies
`register_name/3` globally associates `Name` with `Pid` and notifies all nodes. When new nodes join, they are informed of existing names and vice versa; if a name clash is discovered, the `Resolve` function is called once per clash to decide which pid is correct. If `Resolve` crashes or returns anything other than one of the pids, the name is unregistered. The function is completely synchronous: on return the name is registered on all nodes or none. Returns `yes` on success, `no` on failure (e.g. name already in use or process already registered).

Warning: if you plan to change code without restarting the system, you MUST use an external fun (`fun Module:Function/Arity`) as `Resolve`; a local fun can never have its module code replaced.

Three predefined resolver policies (all arity 3, usable for `register_name/3` and `re_register_name/3`):
- `random_exit_name(Name, Pid1, Pid2) -> pid()` — randomly selects one pid for registration and KILLS the other. (Default resolver.)
- `random_notify_name(Name, Pid1, Pid2) -> pid()` — randomly selects one pid for registration and sends `{global_name_conflict, Name}` to the other pid (no kill).
- `notify_all_name(Name, Pid1, Pid2) -> none` — unregisters BOTH pids and sends `{global_name_conflict, Name, OtherPid}` to both processes.

Note: Erlang/OTP R10 and earlier did not check if a process was already registered, so the global name table could become inconsistent. The old (buggy) behavior can be re-enabled by setting kernel variable `global_multi_name_action = allow`.

If a process with a registered name dies, or its node goes down, the name is unregistered on all nodes.

## trans / locks
`trans/4` sets a lock on `Id` (using `set_lock/3`); if it succeeds, `Fun()` is evaluated and the result `Res` is returned. Returns `aborted` if the lock attempt fails. `Retries = infinity` (default) means the transaction never aborts.

`set_lock/3` sets a lock on the specified `Nodes` using `id()`. If a lock already exists on `ResourceId` for a different `LockRequesterId` and `Retries /= 0`, the process sleeps and retries; after `Retries` attempts returns `false`, otherwise `true`. `infinity` eventually returns `true` (unless the lock is never released). Fully synchronous. If a process holding a lock dies or its node goes down, the locks it held are deleted. The global name server tracks all processes sharing the same lock: if two processes set the same lock, both must delete it.

Deadlock: cannot occur if processes only lock one resource at a time. Can occur if processes lock two or more resources; detection/resolution is up to the application.

Reserved `ResourceId` values that MUST be avoided (else Erlang/OTP misbehaves): `dist_ac`, `global`, `mnesia_adjust_log_writes`, `mnesia_table_lock`.

## Strict rules
- Both `connect_all` and `prevent_overlapping_partitions` kernel parameters must be enabled for reliable service; API never fails when disabled, just returns unreliable results.
- `prevent_overlapping_partitions` must be enabled on ALL nodes to work properly.
- Use external funs (`fun M:F/A`) for `Resolve` if you hot-load code; local funs prevent code replacement of their module.
- Avoid reserved `ResourceId` values: `dist_ac`, `global`, `mnesia_adjust_log_writes`, `mnesia_table_lock`.
- `register_name` is fully synchronous: name is registered on all nodes or none when it returns.
- `send/2` exits with `{badarg, {Name, Msg}}` if `Name` is not globally registered (unlike `whereis_name/1` which returns `undefined`).
- Prefer `global:disconnect/0` over manual disconnect to avoid creating overlapping partitions.
- No need to call `disconnect/0` before halting a node; removal is automatic on halt regardless of `prevent_overlapping_partitions`.
- If the fully connected network is not set up properly, first try increasing `net_setuptime`.
- Relevant kernel params: `net_setuptime`, `net_ticktime`, `dist_auto_connect`, `connect_all`, `prevent_overlapping_partitions`.

## Verbatim quotes
- "A global name registration facility."
- "The registered names are stored in replica global name tables on every node. There is no central storage point. Thus, the translation of a name to a pid is fast, as it is always done locally."
- "Both the registration and lock services are atomic. All nodes involved in these actions have the same view of the information."
- "As of OTP 25, `global` will by default prevent overlapping partitions due to network issues by actively disconnecting from nodes that reports that they have lost connections to other nodes."
- "A network of overlapping partitions might cause the internal state of `global` to become inconsistent. Such an inconsistency can remain even after such partitions have been brought together to form a fully connected network again."
- "None of the above services will be reliably delivered unless both of the kernel parameters `connect_all` and `prevent_overlapping_partitions` are enabled. Calls to the `global` API will, however, not fail even though one or both of them are disabled. You will just get unreliable results."
- "If you plan to change code without restarting your system, you must use an external fun (`fun Module:Function/Arity`) as function `Resolve`. If you use a local fun, you can never replace the code for the module that the fun belongs to."
- "This function is completely synchronous, that is, when this function returns, the name is either registered on all nodes or none."
- "If `Name` is not a globally registered name, the calling function exits with reason `{badarg, {Name, Msg}}`." (send/2)
- "A deadlock can never occur as long as processes only lock one resource at a time. A deadlock can occur if some processes try to lock two or more resources. It is up to the application to detect and rectify a deadlock."
- "If `Retries` is `infinity`, `true` is eventually returned (unless the lock is never released)."
- "The global name server keeps track of all processes sharing the same lock, that is, if two processes set the same lock, both processes must delete the lock."
- `random_exit_name/3`: "The function randomly selects one of the pids for registration and kills the other one."
- `random_notify_name/3`: "The function randomly selects one of the pids for registration, and sends the message `{global_name_conflict, Name}` to the other pid."
- `notify_all_name/3`: "The function unregisters both pids and sends the message `{global_name_conflict, Name, OtherPid}` to both processes."

## Version notes
- Page documents OTP 29.0.2, kernel v11.0.2 (major version 29).
- `prevent_overlapping_partitions` default behavior introduced in OTP 25.
- `disconnect/0` introduced in OTP 25.1.
- `global_multi_name_action = allow` restores pre-R10 (buggy) behavior that did not check if a process was already registered.
- Built with ExDoc v0.40.3. Source: `lib/kernel/src/global.erl` (OTP-29.0.2 tag).
- Copyright © 1996-2026 Ericsson AB.

## Discovered links
### Relevant (crawl later)
- https://www.erlang.org/doc/apps/kernel/global_group.html — `global_group` module (global groups, referenced in See Also and disconnect docs)
- https://www.erlang.org/doc/apps/kernel/net_kernel.html — `net_kernel` module (nodeup/nodedown, networking; referenced in See Also)
- https://www.erlang.org/doc/apps/kernel/kernel_app.html — kernel application config (connect_all, prevent_overlapping_partitions, net_setuptime, net_ticktime, dist_auto_connect, global_multi_name_action)

### Skipped
- https://www.erlang.org/doc/apps/erts/erlang.html — erts BIFs (register/2, whereis/1, nodes/0, types); covered broadly elsewhere
- https://github.com/erlang/otp/blob/OTP-29.0.2/lib/kernel/src/global.erl — source mirror, not docs
- global.md (copy-markdown link), llms.txt, kernel.epub, search.html — site chrome
- https://erlang.org, https://www.ericsson.com, https://github.com/elixir-lang/ex_doc — external/non-doc
