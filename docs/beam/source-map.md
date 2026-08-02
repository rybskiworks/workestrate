# BEAM / OTP Source Map

## Purpose

This document maps every crawled source URL to the BEAM/OTP documentation files that use it. It is the authoritative provenance index behind the `docs/beam/` topic files. Future agents can trace any claim back to its primary source, identify coverage gaps, and assess which sources remain available for future expansion. All 47 sources were live-fetched via curl (HTTP 200 or verified redirect) from erlang.org at OTP 29.0.2; the extracted content is persisted in `docs/beam/.crawl/01–47`.

The intended audience is any future maintainer, reviewer, or agent who needs to verify a claim, add a topic, or audit coverage. It is a provenance layer between the generated guidance and the official sources, not a replacement for them. Every source is an official erlang.org page maintained by the Erlang/OTP team (authority: official primary).

## Source coverage summary

The corpus comprises 47 crawled source URLs across 8 documentation families, all official primary, all fetched at OTP 29.0.2.

| Family | Count | Authority |
|---|---|---|
| Erlang/OTP system docs (`erlang.org/doc/system/`) | 19 | Official primary |
| Erlang/OTP STDLIB module docs (`erlang.org/doc/apps/stdlib/`) | 9 | Official primary |
| Erlang/OTP Kernel module docs (`erlang.org/doc/apps/kernel/`) | 11 | Official primary |
| Erlang/OTP ERTS docs (`erlang.org/doc/apps/erts/`) | 4 | Official primary |
| Erlang/OTP runtime_tools module docs (`erlang.org/doc/apps/runtime_tools/`) | 1 | Official primary |
| Erlang/OTP observer module docs (`erlang.org/doc/apps/observer/`) | 1 | Official primary |
| Erlang/OTP mnesia docs (`erlang.org/doc/apps/mnesia/`) | 1 | Official primary |
| Erlang/OTP efficiency/system guide (`efficiency_guide/` → `system/`) | 1 | Official primary |
| **Total** | **47** | |

The system docs family is the largest (19 pages) and forms the conceptual backbone; the STDLIB (9) and Kernel (11) module families supply the concrete callback contracts and function inventories; the ERTS family (4) supplies the BIF reference (`erlang.html`), the NIF C API (`erl_nif.html`), alternative distribution, and time correction; the runtime_tools (1) and observer (1) families supply the interactive/scripted tracing tools (`dbg`, `ttb`); the mnesia family (1) supplies the transactional DBMS overview; and the efficiency/system guide (1) supplies binary-handling internals. All 47 pages were crawled to completion and used to write the 22 canonical topic docs.

> **Skill-surface re-scope note:** BEAM's skill surface has been re-scoped to `constraint-beam-*` (4 cross-language invariants) + `workflow-beam-runtime-diagnosis` (the one cross-language workflow). The historical `validation-beam-compile/-test/-dialyzer` and `workflow-beam-{implementation,code-review,refactoring,debugging,validation}` packages were removed; the "Workflows influenced" column below reflects the pre-re-scope provenance and is retained for audit history.

## Link expansion coverage

- **Depth-0 seed pass:** The crawl began with the Erlang/OTP system docs that anchor the corpus — `design_principles.html`, `gen_server_concepts.html`, `sup_princ.html`, `statem.html`, `applications.html`, `ref_man_processes.html`, `errors.html`, `release_handling.html`, `events.html`, and `spec_proc.html` (crawl files 01–10). These conceptual/narrative pages are the depth-0 seeds.
- **Depth-1/2 discovery:** From each seed page, discovered links were followed to the module-level references they cite. The system docs cross-reference STDLIB modules (`gen_server.html`, `gen_statem.html`, `supervisor.html`, `proc_lib.html`, `sys.html`, `ets.html`, `gen_event.html`, `timer.html`), Kernel modules (`application.html`, `code.html`, `config.html`, `logger.html`, `net_kernel.html`, `global.html`, `erpc.html`, `trace.html`, `file.html`, `os.html`), and ERTS pages (`erlang.html`, `erl_nif.html`, `alt_dist.html`, `time_correction.html`). Additional system docs were discovered via link-following (`distributed.html`, `ports.html`, `code_loading.html`, `nif.html`, `commoncaveats.html`, `logger_chapter.html`, `appup_cookbook.html`, `secure_coding.html`).
- **Gap-fill pass (crawls 35–47):** A dedicated expansion pass closed the gaps left by depth-0/1/2. It crawled the STDLIB `gen_event` module ref (35), the bit-syntax / binary-handling chapters (36, 43), the `dbg` and `ttb` tracing tools (37, 38), alternative distribution and time-correction ERTS chapters (39, 40), DETS and the Mnesia overview (41, 42), the `file` and `os` Kernel modules (44, 45), the Appup Cookbook (46), and the Secure Coding guidelines (47). This pass raised the corpus from 34 to 47 sources and from 20 to 22 topic docs.
- **Families with rich discovered links:** The system docs family had the richest cross-referencing — `design_principles.html` links to `sup_princ`, `gen_server_concepts`, `spec_proc`, `applications`, `release_handling`; `ref_man_processes.html` (crawl 06) alone discovered 70+ links into `erlang.html` BIF anchors, `erl_nif.html`, `erl_dist_protocol.html`, and `erpc.html`. The ERTS `erlang.html` page (crawl 17) is the single largest reference and was a hub for timer, process, and distribution BIFs. The `logger_chapter.html` page (crawl 20) discovered the entire logger handler/formatter/filter module family.
- Note: the crawl ledger headers do not carry an explicit `origin` field; origin (seed vs discovered) below is inferred from crawl order and link structure — pages 01–10 (the conceptual system docs) are classified as seed; pages 11–34 (module references and additional system docs reached by following discovered links) are classified as discovered; pages 35–47 (the gap-fill pass) are classified as discovered (gap-fill).

## Sources by topic

Legend:

- **Origin:** seed = depth-0 conceptual system doc; discovered = reached by following links at depth 1–2; discovered (gap-fill) = crawled in the dedicated 35–47 expansion pass.
- **Authority:** official primary = erlang.org documentation maintained by the Erlang/OTP team.
- **Coverage status:** fully explored = the page was crawled to completion and substantially used to write the topic docs. All 47 pages are fully explored.

### 1. Erlang/OTP system docs (19 pages)

| # | URL | Canonical URL if redirected | Origin | Pages followed | Topics extracted | Generated docs that use it | Skills referencing it | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 01 | https://www.erlang.org/doc/system/design_principles.html | — | seed | sup_princ, gen_server_concepts, statem, events, spec_proc, applications, release_handling, release_structure, create_target | OTP design principles, supervision trees, workers vs supervisors, behaviours, applications, releases | overview, otp-behaviours, supervision, applications, releases | beam-supervision, beam-gen-server, beam-gen-statem, beam-processes, beam-errors-failures, beam-applications-releases, beam-observability-debugging | impl, review, refactor, debug, val | official primary | fully explored |
| 02 | https://www.erlang.org/doc/system/gen_server_concepts.html | — | seed | design_principles, sup_princ, statem, gen_server (module), global | gen_server client-server model, message dispatch, sys support, code change, callback contract | gen-server, otp-behaviours | beam-gen-server, beam-gen-statem, beam-supervision, beam-processes, beam-errors-failures, beam-observability-debugging | impl, review, refactor, val | official primary | fully explored |
| 03 | https://www.erlang.org/doc/system/sup_princ.html | — | seed | supervisor (module), gen_server, gen_statem, gen_event, erlang, release_handling, events, spec_proc, applications | Supervision theory, supervisor flags, child specs, restart strategies, intensity/period, auto_shutdown | supervision | beam-supervision, beam-gen-server, beam-errors-failures, beam-observability-debugging | impl, review, debug, refactor, val | official primary | fully explored |
| 04 | https://www.erlang.org/doc/system/statem.html | — | seed | gen_server, gen_event, gen_server_concepts, events, sup_princ, gen_statem (module), gen_server (module), gen_fsm, sys, proc_lib, maps, global, erlang | gen_statem state machine model, callback modes, state enter, event types, timeouts, actions | gen-statem | beam-gen-statem, beam-supervision, beam-errors-failures, beam-observability-debugging | impl, review, refactor | official primary | fully explored |
| 05 | https://www.erlang.org/doc/system/applications.html | — | seed | app, application, config, code, systools, distributed_applications, included_applications, release_structure, release_handling, sup_princ, spec_proc | Applications, .app resource, application callback, env, start phases, included applications, application controller | applications | beam-applications-releases | impl, val | official primary | fully explored |
| 06 | https://www.erlang.org/doc/system/ref_man_processes.html | — | seed | data_types, expressions, errors, features, design_principles, distributed, ets, erpc, erl_dist_protocol, erl_nif, erlang (70+ BIF anchors) | Process states, signals, links, monitors, message passing, exit signal reception rules, directly visible resources | processes-and-messages, links-monitors-and-exits | beam-processes, beam-errors-failures, beam-supervision, beam-observability-debugging | review, debug, rtdiag | official primary | fully explored |
| 07 | https://www.erlang.org/doc/system/errors.html | — | seed | erlang (error/exit/throw/raise/link/monitor/stacktrace), erl_error, ref_man_processes, expressions, system_limits, features | Exception classes (error/exit/throw), exit reasons, raise/3, stacktrace | links-monitors-and-exits, common-mistakes | beam-processes, beam-errors-failures, beam-supervision, beam-observability-debugging | review, debug | official primary | fully explored |
| 08 | https://www.erlang.org/doc/system/release_handling.html | — | seed | code_loading, release_structure, create_target, appup_cookbook, appup, relup, release_handler, systools, sasl_app, gen_server, erlang, erl_cmd, init, heart, application | Release handling, appup/relup, release instructions, hot code upgrade, release_handler, systools, target systems, heart | releases | beam-applications-releases | val | official primary | fully explored |
| 09 | https://www.erlang.org/doc/system/events.html | — | seed | gen_event (module), sup_princ, statem, global | gen_event behaviour, event manager vs handler, add_handler/add_sup_handler, notify, callback contract | gen-event, otp-behaviours | beam-gen-server, beam-gen-statem, beam-supervision, beam-processes, beam-errors-failures, beam-observability-debugging | impl, review, refactor | official primary | fully explored |
| 10 | https://www.erlang.org/doc/system/spec_proc.html | — | seed | sys, proc_lib, logger_chapter, sup_princ, applications, design_principles, statem, erlang, typespec | Special processes, proc_lib/sys integration, system messages/events/callbacks | proc-lib-and-sys, otp-behaviours | beam-gen-server, beam-gen-statem, beam-supervision, beam-processes, beam-observability-debugging | impl, review, refactor, debug, rtdiag, val | official primary | fully explored |
| 20 | https://www.erlang.org/doc/system/logger_chapter.html | https://www.erlang.org/doc/apps/kernel/logger_chapter.html | discovered | logger, logger_formatter, logger_filters, logger_std_h, logger_disk_log_h, logger_handler, logger_cookbook, kernel_app, disk_log, error_logger, error_logging | Logger architecture, levels, handlers, filters, formatters, macros, structured logging, report callbacks | logger-and-config | beam-logger-config | rtdiag | official primary | fully explored |
| 26 | https://www.erlang.org/doc/system/distributed.html | — | discovered | ssl_distribution, alt_dist, net_kernel, global, global_group, net_adm, peer, erl_cmd, epmd_cmd, ei_users_guide, tutorial, code_loading, ref_man_processes | Distributed Erlang, nodes, cookies, EPMD, net_kernel, distributed message passing | distribution | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | impl, review, debug, rtdiag, val | official primary | fully explored |
| 29 | https://www.erlang.org/doc/system/ports.html | — | discovered | tutorial, erlang, erl_driver, driver_entry, erl_ddll, erlang port BIFs | Ports, port communication, port drivers, linked-in drivers, port signals | ports-io | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | impl, debug | official primary | fully explored |
| 32 | https://www.erlang.org/doc/system/code_loading.html | — | discovered | code, system_principles, compile, make, erl_cmd, erlc_cmd, erlang, distributed, ports | Code loading, embedded vs interactive mode, code replacement, on_load | releases | beam-applications-releases | val | official primary | fully explored |
| 33 | https://www.erlang.org/doc/system/nif.html | — | discovered | example, debugging, erl_nif, erlang#load_nif | NIF tutorial, ERL_NIF_INIT, lifecycle, dirty schedulers, resource objects | nifs | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | review, debug, rtdiag, val | official primary | fully explored |
| 34 | https://www.erlang.org/doc/system/commoncaveats.html | — | discovered | timer, gen_server, erl_nif, binaries, introduction | Common caveats: timer bottleneck, ++, copying, atoms, length, setelement, size, NIFs | common-mistakes, timers, processes-and-messages | beam-processes, beam-observability-debugging, beam-errors-failures, beam-applications-releases, beam-logger-config | impl, review, debug, rtdiag | official primary | fully explored |
| 36 | https://www.erlang.org/doc/system/binaries.html | https://www.erlang.org/doc/system/bit_syntax.html | discovered (gap-fill) | efficiency_guide, efficiency_guide/binaryhandling, expressions#bit-syntax-expressions, expressions#guard-expressions, erlang#binary_to_list, reference_manual, list_comprehensions | Bit syntax construction & matching, segment Value:Size/TypeSpecifierList, type/signedness/endianness/unit, defaults, construction rules (badarg), matching rules, size as guard expr (OTP 23+), appending | binaries | beam-processes, beam-observability-debugging, beam-errors-failures | impl, review, rtdiag | official primary | fully explored |
| 46 | https://www.erlang.org/doc/system/appup_cookbook.html | — | discovered (gap-fill) | release_handling, gen_server_concepts, sup_princ, spec_proc, secure_coding, system/index, stdlib/gen_server, stdlib/supervisor, stdlib/gen_statem, stdlib/gen_event, stdlib/gen_fsm, stdlib/sys | Appup cookbook, functional vs residence module, load_module/add_module/delete_module, DepMods, update supervisor, changing gen_server (code_change state migration), changing code a process is running, appup instruction recipes | releases | beam-applications-releases | val | official primary | fully explored |
| 47 | https://www.erlang.org/doc/system/secure_coding.html | — | discovered (gap-fill) | design_principles, otp_versions_tree, vex, source markdown, OWASP/CWE external | Secure coding, threat model (trusted/untrusted, what is/isn't protected), binary_to_term safe option, atom exhaustion, code injection (eval/apply), distribution/cookie security, ports/NIF input validation, secure coding rules, CWE/OWASP mapping | common-mistakes, validation | beam-processes, beam-observability-debugging, beam-errors-failures, beam-applications-releases, beam-logger-config | review, debug, rtdiag, val | official primary | fully explored |

### 2. Erlang/OTP STDLIB module docs (9 pages)

| # | URL | Canonical URL if redirected | Origin | Pages followed | Topics extracted | Generated docs that use it | Skills referencing it | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 11 | https://www.erlang.org/doc/apps/stdlib/gen_server.html | — | discovered | gen_server_concepts, ref_man_processes, release_handling, gen_event, gen_statem, proc_lib, supervisor, sys | gen_server module, callback typespecs, exit trapping, throw handling, start/start_link, call/cast/reply | gen-server | beam-gen-server, beam-supervision, beam-errors-failures, beam-observability-debugging | impl, review, refactor, val | official primary | fully explored |
| 12 | https://www.erlang.org/doc/apps/stdlib/gen_statem.html | — | discovered | gen_fsm, gen_server, supervisor, sys, proc_lib, logger, statem, sup_princ, release_handling, ref_man_processes | gen_statem module, callback modes, state enter, event types, timeouts, actions, code_change/4 | gen-statem | beam-gen-statem, beam-supervision, beam-errors-failures, beam-observability-debugging | impl, review, refactor | official primary | fully explored |
| 13 | https://www.erlang.org/doc/apps/stdlib/supervisor.html | — | discovered | sup_princ, release_handling, appup_cookbook, gen_server, gen_statem, gen_event, sys, global, logger, erlang, proplists | supervisor module, supervisor flags, child specs, start_link, start_child, terminate_child, which_children, auto_shutdown | supervision | beam-supervision, beam-gen-server, beam-errors-failures, beam-observability-debugging | impl, review, debug, refactor, val | official primary | fully explored |
| 14 | https://www.erlang.org/doc/apps/stdlib/proc_lib.html | — | discovered | sys, logger, error_logging, erlang, c | proc_lib module, spawn/start/start_link/start_monitor, init_ack, init_fail, stop, hibernate, initial_call, crash reports | proc-lib-and-sys | beam-gen-server, beam-supervision, beam-errors-failures, beam-observability-debugging | debug, refactor, rtdiag, val | official primary | fully explored |
| 15 | https://www.erlang.org/doc/apps/stdlib/sys.html | — | discovered | gen_server, gen_statem, gen_event, io, erlang, file | sys module, get_state, get_status, replace_state, suspend, resume, change_code, terminate, log, trace, statistics, system messages | proc-lib-and-sys, runtime-debugging | beam-gen-server, beam-supervision, beam-errors-failures, beam-observability-debugging, beam-processes, beam-applications-releases, beam-logger-config | debug, refactor, rtdiag, val | official primary | fully explored |
| 16 | https://www.erlang.org/doc/apps/stdlib/timer.html | — | discovered | erlang (send_after, start_timer, monotonic_time, timestamp), time_correction, commoncaveats, system_limits, os, erlang#spawn | timer module, send_after/apply_after/send_interval/apply_interval, cancel, exit_after/kill_after, sleep, tc | timers | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | impl, review, rtdiag | official primary | fully explored |
| 24 | https://www.erlang.org/doc/apps/stdlib/ets.html | — | discovered | dets, match_spec, ms_transform, qlc, dbg, lists, erlang, time_correction, erl_cmd | ETS table types, protection, keypos, heir, insert/lookup/match/select, foldl/foldr, give_away, safe_fixed | ets-data | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | impl, review, rtdiag | official primary | fully explored |
| 35 | https://www.erlang.org/doc/apps/stdlib/gen_event.html | — | discovered (gap-fill) | gen_server#call/3, supervisor, sys#get_status, proc_lib#hibernate, events, ref_man_processes#blocking-signaling, sasl/appup, kernel/logger | gen_event module, callback contract (init/handle_event/handle_call/handle_info/terminate/code_change/format_status), start/start_link/start_monitor, add_handler/add_sup_handler, notify/sync_notify, call, delete_handler, swap_handler, supervised-handler semantics, no get_state for managers | gen-event | beam-gen-server, beam-gen-statem, beam-supervision, beam-processes, beam-errors-failures, beam-observability-debugging | impl, review, refactor | official primary | fully explored |
| 41 | https://www.erlang.org/doc/apps/kernel/dets.html | https://www.erlang.org/doc/apps/stdlib/dets.html | discovered (gap-fill) | stdlib/ets, mnesia/mnesia, erts/time_correction, kernel/file | dets disk-based term storage, open_file/1,2, close, insert/insert_new, lookup, delete, match/match_object/select (+continuations), safe_fixtable, foldl/foldr, first/next, info, sync, table types (set/bag/duplicate_bag; NO ordered_set), 2 GB limit, repair on open, DETS vs ETS | ets-data | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | impl, review, rtdiag | official primary | fully explored |

### 3. Erlang/OTP Kernel module docs (11 pages)

| # | URL | Canonical URL if redirected | Origin | Pages followed | Topics extracted | Generated docs that use it | Skills referencing it | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 18 | https://www.erlang.org/doc/apps/kernel/application.html | — | discovered | app | application module, start/stop, get_env/set_env, load/unload, ensure_started, spec/info, restart_type | applications | beam-applications-releases | impl, val | official primary | fully explored |
| 19 | https://www.erlang.org/doc/apps/kernel/config.html | — | discovered | app, erl_cmd, design_principles, script, typespec | config module, read/parse, sys.config, config file reading | logger-and-config, applications | beam-logger-config, beam-applications-releases | impl, val, rtdiag | official primary | fully explored |
| 21 | https://www.erlang.org/doc/apps/kernel/logger.html | — | discovered | logger_handler, logger_std_h, logger_disk_log_h, logger_formatter, logger_filters, logger_chapter, config | logger module, levels, primary/module level, handlers, filters, formatters, macros, report callbacks, compare_levels | logger-and-config | beam-logger-config | rtdiag | official primary | fully explored |
| 22 | https://www.erlang.org/doc/apps/kernel/code.html | — | discovered | code_loading, init, erl_prim_loader, erlang#check_process_code, compile, erl_cmd, escript, filename, script, cover, eep48_chapter, eep-0048 | code module, load_file, ensure_loaded, purge, soft_purge, delete, path management, two module versions | releases | beam-applications-releases | val | official primary | fully explored |
| 23 | https://www.erlang.org/doc/apps/kernel/app.html | — | discovered | application, config, applications, versions, systools, erl_cmd | .app resource file format, term structure, required/optional keys (description, vsn, modules, registered, applications) | applications | beam-applications-releases | impl, val | official primary | fully explored |
| 27 | https://www.erlang.org/doc/apps/kernel/global.html | — | discovered | global_group, net_kernel, kernel_app | global module, cluster-wide name registration, register_name/whereis_name/trans | distribution | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | impl, review, debug, rtdiag, val | official primary | fully explored |
| 28 | https://www.erlang.org/doc/apps/kernel/net_kernel.html | — | discovered | distributed, ssl_distribution, kernel, erl_cmd, erlang#nodes, erlang#spawn | net_kernel module, start/stop, monitor_nodes, node names, distribution layer | distribution | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | impl, review, debug, rtdiag, val | official primary | fully explored |
| 30 | https://www.erlang.org/doc/apps/kernel/erpc.html | — | discovered | rpc, reference_manual/processes | erpc module, call/cast/send_request, error semantics, call_options; erpc as modern replacement for rpc | distribution | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | impl, review, debug, rtdiag, val | official primary | fully explored |
| 31 | https://www.erlang.org/doc/apps/kernel/trace.html | (canonical_url absent in ledger; seed_url is canonical) | discovered | dbg, ttb, tprof, match_spec, erl_tracer, erlang (trace/trace_pattern/trace_info/trace_delivered), time_correction, erl_cmd | trace module (OTP 27.0), session lifecycle, process/port/function/send/recv tracing, trace flags, system monitoring | runtime-debugging | beam-observability-debugging, beam-errors-failures, beam-applications-releases, beam-logger-config | debug, rtdiag, val | official primary | fully explored |
| 44 | https://www.erlang.org/doc/apps/kernel/file.html | — | discovered (gap-fill) | stdlib/io, stdlib/filename, stdlib/unicode, stdlib/epp, stdlib/unicode_usage, stdlib/zstd, erts/erlang | file module, read_file/write_file, open/2 modes (read/write/append/exclusive/raw/binary/delayed_write/read_ahead/compressed/zstd/encoding/sync/ram/directory), read/read_line/write, list_dir, make_dir/del_dir, delete, copy, rename, read_file_info, get_cwd, datasync/sync, atomicity caveats, filename encoding | runtime-environment | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases | impl, review, debug, rtdiag | official primary | fully explored |
| 45 | https://www.erlang.org/doc/apps/kernel/os.html | — | discovered (gap-fill) | erts/erlang#timestamp/convert_time_unit/time_unit, erts/time_correction, erts/erl_cmd#file_name_encoding, file#native_name_encoding, stdlib/filename, stdlib/calendar, kernel_app#os_cmd_shell/erl_signal_server | os module, cmd/1,2, env/getenv/putenv/unsetenv, find_executable, getpid, perf_counter, system_time, timestamp, type, version, set_signal, native_name_encoding, cmd vs ports, os time vs monotonic | runtime-environment | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases | impl, review, debug, rtdiag | official primary | fully explored |

### 4. Erlang/OTP ERTS docs (4 pages)

| # | URL | Canonical URL if redirected | Origin | Pages followed | Topics extracted | Generated docs that use it | Skills referencing it | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 17 | https://www.erlang.org/doc/apps/erts/erlang.html | — | discovered | time_correction, net_kernel, trace, seq_trace, logger, os, application, error_handler, msacc, scheduler, instrument, calendar | erlang module BIFs: spawn_opt, process_info, system_info, statistics, process_flag, garbage_collect, hibernate, monitor, demonitor, link, unlink, exit, exit_signal, raise, alias, unalias | processes-and-messages, links-monitors-and-exits, runtime-debugging, timers | beam-processes, beam-errors-failures, beam-supervision, beam-observability-debugging, beam-applications-releases, beam-logger-config | impl, review, debug, rtdiag, val | official primary | fully explored |
| 25 | https://www.erlang.org/doc/apps/erts/erl_nif.html | — | discovered | nif (system), modules, erl_driver, erlang#load_nif, erlang#nif_error, erts_alloc | erl_nif C API, ERL_NIF_INIT, resource objects, ErlNifEnv, term construction/inspection, binary handling, dirty schedulers, monitoring | nifs | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | review, debug, rtdiag, val | official primary | fully explored |
| 39 | https://www.erlang.org/doc/apps/erts/alt_dist.html | — | discovered (gap-fill) | erts/erl_dist_protocol, kernel/net_kernel, erts/alt_disco, erts/erl_driver, erts/driver_entry, kernel/kernel_app, erts/erlang | Alternative distribution carrier protocol, driver/distribution-controller, interface module, distribution module callbacks, dist_util #hs_data, data delivery ordering, -proto_dist/-no_epmd, blocking-signaling caveat (non-blocking ticks) | distribution | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | impl, review, debug, rtdiag, val | official primary | fully explored |
| 40 | https://www.erlang.org/doc/apps/erts/time_correction.html | — | discovered (gap-fill) | erts/erl_cmd, erts/erlang, stdlib/timer, kernel/os, erts/communication, erts/match_spec | Time correction, monotonic vs system time, time offset, time warp modes (no/single/multi), multi-time-warp default (OTP 26), time-warp-safe code requirement, +C/+c flags, now/0 deprecation, timer accuracy | timers, runtime-debugging | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | impl, review, rtdiag, val | official primary | fully explored |

### 5. Erlang/OTP runtime_tools module docs (1 page)

| # | URL | Canonical URL if redirected | Origin | Pages followed | Topics extracted | Generated docs that use it | Skills referencing it | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 37 | https://www.erlang.org/doc/apps/runtime_tools/dbg.html | — | discovered (gap-fill) | runtime_tools/dbg_guide, erts/match_spec, erts/erl_tracer, stdlib/ms_transform, kernel/trace#process/port/function, kernel/rpc | dbg text trace facility, tracer/0,2,3, p/1,2, tp/tpl/tpe/ctp/ctpg/ctpl/ctpe, match specs, built-in aliases (x/c/cx), trace_port/trace_client, n/cn/ln, ltp/dtp/wtp/rtp | runtime-debugging | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | debug, rtdiag, val | official primary | fully explored |

### 6. Erlang/OTP observer module docs (1 page)

| # | URL | Canonical URL if redirected | Origin | Pages followed | Topics extracted | Generated docs that use it | Skills referencing it | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 38 | https://www.erlang.org/doc/apps/observer/ttb.html | — | discovered (gap-fill) | runtime_tools/dbg, kernel/seq_trace, erts/match_spec, ttb_ug#format | ttb Trace Tool Builder, tracer/0,1,2, p/2, tp/tpl/tpe/ctp, start_trace/4, stop/0,1, format/1,2, get_et_handler, write_trace_info, seq_trigger_ms, history/config, dbg-parallel-use warning | runtime-debugging | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | debug, rtdiag, val | official primary | fully explored |

### 7. Erlang/OTP mnesia docs (1 page)

| # | URL | Canonical URL if redirected | Origin | Pages followed | Topics extracted | Generated docs that use it | Skills referencing it | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 42 | https://www.erlang.org/doc/apps/mnesia/mnesia_chapter.html | https://www.erlang.org/doc/apps/mnesia/mnesia_overview.html | discovered (gap-fill) | mnesia/mnesia_chap1, mnesia/mnesia_chap2, stdlib/dets, kernel/disk_log, stdlib/qlc | Mnesia overview, transactional distributed DBMS, ETS/DETS-backed, create_table storage types (ram_copies/disc_copies/disc_only_copies), schema reconfiguration, transactions vs dirty ops, replication/fragmentation, when to use Mnesia vs ETS vs DETS | ets-data | beam-observability-debugging, beam-processes, beam-errors-failures, beam-applications-releases, beam-logger-config | impl, review, rtdiag | official primary | fully explored |

### 8. Erlang/OTP efficiency/system guide (1 page)

| # | URL | Canonical URL if redirected | Origin | Pages followed | Topics extracted | Generated docs that use it | Skills referencing it | Workflows influenced | Authority | Coverage status |
|---|---|---|---|---|---|---|---|---|---|---|
| 43 | https://www.erlang.org/doc/efficiency_guide/binaryhandling.html | https://www.erlang.org/doc/system/binaryhandling.html | discovered (gap-fill) | commoncaveats, maps, erts/erl_nif#enif_inspect_binary | Binary handling efficiency, binary representations (refc/heap binaries, sub-binaries, match contexts), append optimization, iolist/iodata building, circumstances that force copying, binary module matching, perf rules | binaries (or common-mistakes) | beam-processes, beam-observability-debugging, beam-errors-failures | impl, review, rtdiag | official primary | fully explored |

## Redirects

Five seed URLs were found to 404 or redirect during the crawl; the canonical destination was recorded in each case and the crawl succeeded against the canonical URL:

- `https://www.erlang.org/doc/system/logger_chapter.html` returns **404** (the system-docs path was restructured). The canonical URL is `https://www.erlang.org/doc/apps/kernel/logger_chapter.html` (crawl file 20). The crawl succeeded against the canonical URL.
- `https://www.erlang.org/doc/system/binaries.html` returns **404**. The "Constructing and Matching Binaries" content lives at `https://www.erlang.org/doc/system/bit_syntax.html` (page title "Bit Syntax") (crawl file 36). The crawl succeeded against the canonical URL.
- `https://www.erlang.org/doc/efficiency_guide/binaryhandling.html` **redirects** to `https://www.erlang.org/doc/system/binaryhandling.html` (the Efficiency Guide "Binary Handling" chapter moved under system docs) (crawl file 43). The crawl succeeded against the canonical URL.
- `https://www.erlang.org/doc/apps/kernel/dets.html` returns **404**. DETS is in STDLIB, not Kernel; the canonical URL is `https://www.erlang.org/doc/apps/stdlib/dets.html` (crawl file 41). The crawl succeeded against the canonical URL.
- `https://www.erlang.org/doc/apps/mnesia/mnesia_chapter.html` returns **404** (no redirect). The nearest live chapter is `https://www.erlang.org/doc/apps/mnesia/mnesia_overview.html` (crawl file 42). The crawl extracted the overview chapter.
- Crawl file 31 (`trace.html`) has no `canonical_url` field in its ledger header; the seed URL `https://www.erlang.org/doc/apps/kernel/trace.html` is treated as canonical (fetch 200).
- All other 41 crawl files: seed URL == canonical URL, fetch 200, no redirects.

## Discovered but not crawled (revisit later)

The following pages were discovered as cross-references during the crawl but were not themselves crawled. They are the remaining genuinely-uncrawled candidates for a future expansion pass, grouped by family with the reason each was deferred.

### ERTS docs

- `apps/erts/erl_dist_protocol.html` — Erlang distribution protocol (dflags, signal format, atom cache, fragments); the signal-level counterpart to the alt_dist carrier page. Discovered by 06, 39.
- `apps/erts/alt_disco.html` — How to Implement an Alternative Node Discovery; companion to the alt_dist carrier page. Discovered by 39.
- `apps/erts/erl_driver.html` — erl_driver API (driver writer reference). Discovered by 29, 39.
- `apps/erts/driver_entry.html` — driver_entry (ErlDrvEntry struct). Discovered by 29, 39.
- `apps/erts/match_spec.html` — Match Specifications in Erlang (the match-spec language used by `tp`/`tpe`/`select`). Discovered by 24, 37, 38, 40.
- `apps/erts/erl_cmd.html` — `erl` command-line flags reference (`+C`, `+c`, `+fnl`/`+fnu`, `+t`); high-priority revisit (discovered by 10+ files). Discovered by 08, 32, 40, 45.
- `apps/erts/erl_tracer.html` — `erl_tracer` callback module (used by `dbg:tracer/2` `Type=module`). Discovered by 31, 37.

### Kernel module docs

- `apps/kernel/ssl_distribution.html` — TLS distribution setup. Discovered by 26, 28. (Note: the SSL distribution chapter is documented under the `ssl` application; see "ssl/crypto modules" below.)
- `apps/kernel/global_group.html` — global_group module. Discovered by 26, 27.
- `apps/kernel/peer.html` — peer module (peer nodes). Discovered by 26.
- `apps/kernel/seq_trace.html` — sequential tracing. Discovered by 38.
- `apps/kernel/disk_log.html` — disk_log module (disk-based logging; comparison target for DETS/Mnesia). Discovered by 20, 41, 42.

### STDLIB module docs

- `apps/stdlib/gen_fsm.html` — legacy `gen_fsm` module (replaced by `gen_statem`). Discovered by 12, 46.
- `apps/stdlib/ms_transform.html` — `ms_transform` parse transform (backs `fun2ms/1`). Discovered by 24, 37.
- `apps/stdlib/qlc.html` — QLC (Query List Comprehension), Mnesia's query language. Discovered by 24, 42.

### System docs

- `system/system_principles.html` — system principles. Discovered by 32.
- `system/create_target.html` — target system creation. Discovered by 01, 08.
- `system/release_structure.html` — release file structure. Discovered by 01, 05, 08.

### runtime_tools / tools docs

- `apps/runtime_tools/dbg_guide.html` — "Tracing in Erlang with dbg" users guide (quick-start for function-call tracing). Discovered by 37.
- `apps/tools/tprof.html` — `tprof` profiler. Discovered by 31.

### SASL docs

- `apps/sasl/appup.html` — appup file format. Discovered by 08, 46.
- `apps/sasl/relup.html` — relup file format. Discovered by 08.
- `apps/sasl/release_handler.html` — release_handler module. Discovered by 08.
- `apps/sasl/systools.html` — systools (release packaging). Discovered by 05, 08, 23.

### mnesia docs

- `apps/mnesia/mnesia_chap1.html` — "Getting Started": `create_table`, storage types, schema, transactions. HIGH priority; fills the API-detail gaps left by the overview (crawl 42). Discovered by 42.
- `apps/mnesia/mnesia_chap2.html` — deeper Mnesia chapter (transactions, dirty ops, fragmentation, replication details). Discovered by 42.

### ssl / crypto modules

- `apps/ssl/*` and `apps/crypto/*` — TLS/crypto module references (TLS distribution, client certificate verification, cryptographic primitives). Discovered by 26, 28. Deferred as out of current scope.

Note: additional adjacent/reference pages also remain available but are lower priority — language-reference chapters (`data_types`, `expressions`, `features`, `system_limits`, `typespec`, `tutorial`, `introduction`, `modules`), the logger submodule family (`logger_formatter`, `logger_filters`, `logger_std_h`, `logger_disk_log_h`, `logger_handler`, `logger_cookbook`), and adjacent modules (`rpc`, `heart`, `kernel_app`, `error_logger`, `erl_prim_loader`, `erts_alloc`, `erlc_cmd`, `init`, `epmd_cmd`, `erl_interface/ei_users_guide`, `compiler/compile`, `tools/make`, `tools/cover`, `runtime_tools/msacc|scheduler|instrument`). These are out of the current corpus scope.

## Skipped

The following categories of source were intentionally excluded from the crawl: markdown mirrors of the same erlang.org pages (e.g. `*.md` source files linked from ExDoc pages, and the `mnesia_overview.md` mirror); search and index pages (`doc/index.html`, `system/index.html`); vendor/tooling homepages (`erlang.org`, `ericsson.com`, `github.com/elixir-lang/ex_doc`); ePub/download bundles (`*.epub`, `llms.txt`); and GitHub source mirrors (`github.com/erlang/otp/blob/...`). These are not primary documentation and would duplicate content already captured from the canonical HTML pages.

## Verification method

All 47 pages were fetched live via curl and returned HTTP 200 against OTP 29.0.2 (ERTS v17.0.2; stdlib v8.0.1; kernel v11.0.2; runtime_tools v2.4; observer v2.19; mnesia 4.26.1). The extracted content (purpose, key concepts, strict rules, verbatim quotes, and discovered links) is persisted in `docs/beam/.crawl/01–47`. The 22 canonical topic docs were written strictly from those extractions; no claim in the topic docs relies on memory or non-crawled sources. Five seed URLs were found to 404 or redirect (`logger_chapter.html`, `binaries.html`, `binaryhandling.html`, `dets.html`, `mnesia_chapter.html`) and their canonical replacements are recorded in the Redirects section above. Origin (seed vs discovered vs gap-fill) is inferred from crawl order and link structure because the ledger headers do not carry an explicit origin field.
