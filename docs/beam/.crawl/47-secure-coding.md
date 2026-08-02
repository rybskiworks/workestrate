# Crawl: system/secure_coding.html
- seed_url: https://www.erlang.org/doc/system/secure_coding.html
- canonical_url: https://www.erlang.org/doc/system/secure_coding.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: Erlang System Documentation v29.0.2 (source: OTP-29.0.2)
- feeds_docs: common-mistakes.md, validation.md

## Purpose

This section provides a guideline for writing secure Erlang code, describing
common pitfalls and weaknesses best avoided. From time to time, it will reference
the Common Weakness Enumeration (CWE) maintained by MITRE, as well as the
application security risks maintained by OWASP.

There are also sections addressing the Top 40 CWEs, the OWASP Top 10 security
risks, and the OWASP API Security Top 10. Finally, there is a section with
concrete Secure Coding Rules.

This is a living document and is updated as new best practices are discovered. It
is by no means complete, and contributions to improve it are warmly welcomed.
Those following these guidelines will hopefully avoid the listed issues, but
undiscovered issues remain.

### Threat model (what is / is not protected against)

Throughout this document we will use the terms trusted and untrusted to mean
fully trusted and not fully trusted, respectively. No distinction is made between
partially trusted and untrusted and it is important to remember that anything
that is not fully and completely trusted must be considered as untrusted as
something given to you by a malicious actor.

What is NOT protected against:
- All loaded code is assumed to be trusted. There is no built-in sand-boxing
  mechanism for running untrusted Erlang code. Any code loaded and executed
  within the environment has unrestricted access to the system. A malicious BEAM
  module can do anything, including breaking the memory safety protections of
  the runtime system and crashing the virtual machine.
- Excessive resource usage is not prevented by default. Unless safeguards are
  put in place (e.g. heap size limitations), a process is free to consume enough
  resources to crash the whole program.
- All nodes connected through Erlang distribution are assumed to be trusted,
  and have unrestricted access to all other connected systems. There is no
  authentication built into the default distribution protocol, merely a "cookie"
  mechanism that prevents the unintentional mixing of Erlang clusters on the
  same network. For secure communication over an unsecured network, the
  distribution should be configured to use TLS with client certificate
  verification.
- Inappropriate usage of functionality is not guaranteed to crash or produce
  sensible results. The arguments given to many Erlang functions are assumed to
  be trusted, so providing valid-appearing but nonsensical input to a function
  can have it produce valid-appearing garbage instead of crashing or returning
  an error.
- Using undocumented functionality, whether it is a function, option, etc., can
  result in almost anything happening.

What IS protected against:
- Erlang is a memory-safe language. CWE categories related to memory safety
  (spatial and temporal), such as CWE-416, CWE-465 or CWE-1218, cannot occur.
  Race conditions can only affect application logic (message-passing only; a
  classical data race CWE-362 cannot occur unless the programmer explicitly
  built a shared resource). Uninitialized-variable errors cannot occur.
- Importing memory-unsafe code through FFI (drivers, NIFs) may violate this.
- Classical memory leaks cannot occur; memory that is no longer referenced will
  eventually be reclaimed.
- Erlang processes are isolated and can only affect other entities through
  signals (messages, links, monitors). A crashing process does not impact other
  processes other than as defined by the programmer. Execution of one process
  cannot block another (other than as defined by the programmer, e.g. waiting
  for a never-sent reply). Effects of CWE-835 are limited to the offending
  process and recoverable through termination.
- Behavior is defined for all possible inputs; partial operations throw
  exceptions that leave the program in a well-defined state.
- Arbitrary precision integer arithmetic; overflow only at several-megabits size
  and throws an exception instead of wrapping (CWE-190).

## Pitfalls (each: name, problem, recommended practice — verbatim where possible)

### binary_to_term on untrusted input (safe option)

- Rule: DSG-011 - Only Deserialize Trusted Data (priority: High; CWE-74, A05:2025)
- Problem: Erlang/OTP provides various functionality that serializes and
  deserializes general Erlang terms. Such functionality is intended to be used
  in a trusted environment and is not suitable for communication with untrusted
  entities. For example, you do not want to load a mnesia backup from an
  untrusted entity. One issue with this being the potential for atom
  exhaustion, but more importantly you could potentially end up with a mnesia
  table containing harmful data (CWE-502). Other examples are dets and disk_log.
- Recommended practice: JSON is an example of a better format to use when
  communicating with untrusted entities. Erlang/OTP provides the json module for
  JSON encoding/decoding. XML is another example; the xmerl application provides
  functionality for encoding/decoding. The decoded data, of course, needs to be
  validated and sanitized if it does not originate from a trusted entity. Using
  the xmerl_sax_parser when decoding XML you can do this validation during
  decoding, and, for example, prevent issues like memory exhaustion. The json
  module also provides a SAX style decoding with user defined callbacks in which
  you can do the same.
- `safe` option specifics (from DSG-003): There are also a number APIs that
  create general Erlang terms from data of some serialized format. You should
  not use such APIs if the data is not trusted (see DSG-011) unless the API also
  provides some way of preventing creation of atoms. For example,
  `binary_to_term/2` with the `safe` option will prevent new atoms from being
  created. However, note that even if the `safe` option is used and the data
  originates from an untrusted source, it still has to be validated and
  sanitized, since it can still be harmful to the Erlang application in other
  ways. In general, it is best to avoid using such functions altogether on
  untrusted data, even with the `safe` option.
- Unsafe-functionality table entry: `binary_to_term/1` -> alternative
  `binary_to_term/2` with `safe` option (see DSG-003 and DSG-011).
  `binary_to_term/2` without `safe` option -> `binary_to_term/2` with `safe`
  option.
- Also: `file:consult/1` and `file:path_consult/2` are listed as Potentially
  Unsafe (see DSG-003 and DSG-011) — they construct arbitrary terms/atoms from
  file data.

### Atom exhaustion (list_to_atom)

- Rule: DSG-003 - Do Not Abuse Atoms (priority: High; CWE-770, API10:2023)
- Problem: Atoms are designed to provide an easy way to create named constants
  in code. Defining them programmatically is not something that we have
  optimized for, and may also lead to Atom Exhaustion. Specifically, the
  `binary_to_atom/1,2` and `list_to_atom/1` functions will create a new atom if
  it does not already exist. Doing so without care might cause the system to
  crash.
- Background (Atom Exhaustion section): Data of the type atom is very space
  efficient and performant to use, but it must be used with care as it is not
  intended to be used dynamically: the intended use case is to provide named
  constants in code. The amount of atoms in the system is limited (see system
  limits and CWE-770) and cannot be reclaimed once created. That is, if you
  dynamically create a large amount of atoms, the system might crash.
- Recommended practice: If you know that the atom already should exist in the
  system, you can use the `binary_to_existing_atom/1,2` and
  `list_to_existing_atom/1` instead. These functions will throw an exception
  instead of creating a new atom if the atom does not already exist. Note that
  there are valid use-cases for using `binary_to_atom/1,2` and `list_to_atom/1`,
  so you cannot blindly replace these calls.
  However, before using the `*_to_existing_atom()` functions, consider whether
  an explicit conversion is more appropriate. Quite often there are only a few
  atoms that are valid in the context the conversion is done, and in those cases
  it is better to translate them yourself and reject invalid inputs, as
  `*_to_existing_atom()` can return any atom that exists in the system, not just
  those expected in the context.
- Verbatim DO/DO NOT:
  ```erlang
  %% DO, AND PREFER (see STL-001)
  input_to_atom(<<"foo">>) -> foo;
  input_to_atom(<<"bar">>) -> bar;
  input_to_atom(<<"quux">>) -> quux.

  %% DO
  input_to_atom(Text) -> binary_to_existing_atom(Text).

  %% DO NOT
  input_to_atom(Text) -> binary_to_atom(Text).
  ```
- Unsafe-functionality table entries: `binary_to_atom/1` ->
  `binary_to_existing_atom/1`; `binary_to_atom/2` ->
  `binary_to_existing_atom/2`; `list_to_atom/1` -> `list_to_existing_atom/1`
  (all see DSG-003).
- Related: `xmerl_scan` module dynamically produces new atoms and is therefore
  not suitable for decoding XML data originating from untrusted sources (use
  `xmerl_sax_parser` instead).

### Code injection (eval/apply on untrusted MFA)

- Rule: CWE-94 - Improper Control of Generation of Code ('Code Injection')
  (covered in Top 40 CWEs section)
- Problem: This is not Erlang-specific. As with other injection attacks,
  following DSG-007 (Know Your Data) may also help.
- Related rule: MSC-005 - Treat Match Specifications As Code (priority: High;
  CWE-74). Match specifications, such as those used with ETS, are vulnerable to
  injection attacks if they are constructed based on untrusted input.
  Recommended practice: When untrusted data is matched verbatim (such as a
  key), it is important to wrap it in `{const, UntrustedData}` expressions.
  Building general queries based on untrusted data should be avoided, but if
  that cannot be done, the query should be the result of parsing the untrusted
  data into a match specification (where the final shape is controlled by the
  programmer), rather than attempting to validate the data before passing it in
  unaltered.
  ```erlang
  %% DO
  find(Table, Needle) ->
      ets:match(Table, {'_', {const, Needle}, '$1'}).

  %% DO NOT
  find(Table, Needle) ->
      ets:match(Table, {'_', Needle, '$1'}).
  ```
- Related rule: DSG-004 - Do Not Use Undocumented Functionality (priority:
  Critical; CWE-242, CWE-477, CWE-676). Undocumented functions or functionality
  must never be used. This includes undocumented arguments to documented
  functions and undocumented system services. Merely passing the wrong arguments
  to these functions can cause the system to behave in unexpected ways.
- Related rule: DSG-006 - Do Not Use Unsafe Functionality (priority: Critical;
  CWE-242). Unsafe Functions must not be used, and Potentially Unsafe Functions
  should only be used in a safe manner, and are best avoided if possible.
- Related rule: MSC-007 - Avoid Using Debug Functionality in Production
  (priority: Critical; CWE-242, CWE-489, A06:2025). Functionality explicitly
  marked for debugging only, such as `erlang:list_to_pid/1` or the
  `keep_secrets` ssl option, should not be used in production. In production,
  debug functionality should be considered unsafe as per DSG-006.
- Note: The page does not call out `erlang:eval`/`apply/3` on untrusted MFA by
  name as a dedicated rule; the general guidance is the threat-model statement
  that all loaded code is trusted and there is no built-in sandbox for running
  untrusted Erlang code, plus DSG-007 (Know Your Data) and the injection rules
  above. (No verbatim `erlang:eval`/`apply` rule exists on this page.)

### Distribution/cookie security

- Rule: DEP-001 - Do Not Expose Default Erlang Distribution on Untrusted
  Networks (priority: Critical; CWE-200, CWE-668, A01:2025, A02:2025,
  API2:2023, API6:2023, API8:2023)
- Problem: The builtin Erlang distribution makes it possible to easily and
  transparently communicate between Erlang nodes. By default, communication is
  performed over an unencrypted TCP connection with a rudimentary cookie based
  authentication only present in order to prevent mistakes. This configuration
  should only be used in a trusted network.
- EPMD leak: The Erlang Port Mapper Daemon (EPMD) service will respond to
  unauthenticated requests, and can by this leak information about what Erlang
  nodes exist and what ports they are listening on (CWE-668, CWE-200). You are
  therefore advised to disable the default EPMD and implement your own EPMD
  module using another port lookup scheme and enable that EPMD module. The
  simplest solution, assuming only one Erlang node per IP address, would be to
  use a statically assigned port, skip registration of nodes, and just assume
  that a node will be listening on that port.
- Node trust: All nodes admitted into an Erlang cluster must be trusted. Once a
  node is connected to the cluster, it gains complete access to the resources
  and operations of all other nodes, making node trustworthiness a critical
  security consideration.
- Recommended practice: In order to communicate between nodes over an untrusted
  network, the Erlang distribution protocol should be configured to communicate
  between nodes using TLS. If the distribution protocol is not required per se,
  Erlang/OTP includes the ssh and ssl applications, enabling secure
  communication over untrusted networks. Depending on the nature of the
  communication and the remote parties involved, you may need to explicitly
  validate and sanitize incoming data to ensure safety and correctness.
- Background (threat model): There is no authentication built into the default
  distribution protocol, merely a "cookie" mechanism that prevents the
  unintentional mixing of Erlang clusters on the same network. For secure
  communication over an unsecured network, the distribution should be configured
  to use TLS with client certificate verification.

### Ports/NIF input validation

- Rule: MSC-003 - Use open_port/2 With the spawn_executable Option To Start
  External Executables (priority: High; CWE-78)
- Problem: Calling `open_port/2` with the `{spawn, _}` argument will either
  start an instance of a driver, or spawn an external program using the passed
  command line. When spawning an external program, some platforms will start a
  shell, which will search in the PATH in order to find the program. Environment
  variable expansion may also be performed. If user input is to be part of the
  command line for an external program, this makes it hard to verify and
  sanitize this input. Even if user input is not part of the command, mistakes
  can easily be made in non-trivial scenarios. Since loaded drivers and programs
  compete in the same name-space, one can also unintentionally start an instance
  of a driver instead of spawning an external program.
- Recommended practice (external program): use `open_port/2` with the
  `{spawn_executable, _}` argument. No shell is used and the arguments to the
  program needs to be explicitly listed using the `{args, Args}` option where no
  environment variable expansion is made. You may also want to overwrite certain
  environment variables using the `{env, Env}` option in order not to expose
  those to the external program.
- Recommended practice (driver): use `open_port/2` with the
  `{spawn_driver, _}` argument where no mix-up with external programs may occur.
- `os:cmd/1,2`: suffers from most of the same issues as `open_port/2` with
  `{spawn, _}`. A shell will be started, PATH searched, env vars expanded. A
  safer approach is `open_port/2` with `{spawn_executable, _}`.
- Unsafe-functionality table: `open_port/2` with `{spawn, _}` argument ->
  `open_port/2` with `{spawn_executable|spawn_driver, _}` argument (see
  MSC-003). `os:cmd/1` and `os:cmd/2` -> `open_port/2` with
  `{spawn_executable, _}` argument (see MSC-003).
- NIFs (Native Code section): Drivers and NIF libraries are typically written
  in C/C++ and have, if not full access, access to a large part of the runtime
  system internals. They are not necessarily unsafe and sometimes their use is
  unavoidable, but it is much easier to avoid introducing vulnerabilities by
  writing Erlang code whenever possible. Poorly written native code can both
  introduce vulnerabilities as well as stability problems. A NIF library is
  loaded using `erlang:load_nif/2`; a driver is loaded using one of the
  `erl_ddll` load functions.
- Unsafe-functionality table (Potentially Unsafe, CWE-676):
  `erlang:load_nif/2` (see Native Code); `erl_ddll:load/2`,
  `erl_ddll:load_driver/2`, `erl_ddll:try_load/3`, `erl_ddll:reload/2`,
  `erl_ddll:reload_driver/2` (see Native Code).
- Input validation general rule: DSG-007 - Know Your Data (priority: Medium;
  CWE-20, CWE-22, CWE-74, CWE-78, CWE-89, CWE-502, CWE-532, CWE-843, CWE-918,
  CWE-1287). Always know what data your code is operating on, and make sure it
  is documented. Something as simple as marking data as trusted or untrusted
  can be a great help since all code implicitly trusts the data it is given to
  different degrees. Documenting what data your code expects to be operating on
  helps protect against, for example, injection attacks in match specifications
  (see MSC-005), or the leakage of sensitive data (see MSC-004).

### Other listed secure-coding rules

- STL-001 - Be Restrictive (priority: High; CWE-252, CWE-253, CWE-391,
  CWE-392, CWE-394, CWE-396, A10:2025). All unexpected errors should be left to
  the supervision structure, and all unexpected conditions should be considered
  errors. Erlang code should be written as restrictively as possible, to
  provoke errors whenever anything unexpected happens. Avoid catch-all clauses
  (`_ ->`), avoid ignoring return values (`_ = file:write(...)`), prefer
  explicit pattern matching over `binary_to_existing_atom` when the set of
  possible atoms is known, avoid `catch error:_` in favor of specific errors,
  prefer `<:-` (match) over `<-` (filter) in comprehensions to avoid silently
  dropping non-matching entries.
- STL-002 - Avoid Boolean Blindness (priority: Recommendation; CWE-628). Prefer
  descriptive atoms (initialized/uninitialized, changed/unchanged) over bare
  booleans when the boolean has context.
- STL-003 - Use Uppercase Names for Macros (priority: Recommendation). All
  macros should be upper-case so a missing `?` prefix leads to an unbound
  variable error rather than silently valid code.
- DEP-002 - Build and Install Erlang/OTP Yourself (priority: High; A03:2025).
  Build Erlang/OTP yourself instead of relying on external build tools or
  prebuilt installations. Be careful with CFLAGS/LDFLAGS; do not pass `pie`
  options there (use `--enable-pie` configure arg). As of OTP 28.0 the configure
  script tries to enable most OpenSSF-recommended C/C++ hardening flags on
  Unix-like systems (inspect with `V=1`). Build against an up-to-date OpenSSL
  via `--with-ssl=PATH`; consider `--disable-dynamic-ssl-lib` to statically link
  libcrypto so the selected version is actually used.
- DEP-003 - Use Actively Maintained Versions of Erlang/OTP (priority:
  Critical; A03:2025). See the OTP Versions Tree page; VEX documents are
  regularly updated for maintained releases; patches announced on
  erlang-announce and erlang forums.
- DEP-004 - Protect the Code Path (priority: Medium; CWE-427, A08:2025). Ensure
  folders in the code paths are only writable by a dedicated user, separate
  from the user running the VM.
- DEP-005 - Avoid interactive Mode in Production (priority: Critical;
  A08:2025). interactive mode loads code on demand; use embedded mode instead.
- DEP-006 - Minimize VM Privileges (priority: Medium). Run with fewest possible
  privileges; dedicated user; preferably further restricted via
  SELinux/AppArmor, containerization, etc.
- DSG-001 - Encode the Problem Domain in the Supervision Tree (priority:
  Medium; CWE-389, CWE-544, CWE-653, A10:2025). The supervision structure must
  reflect the problem the program solves; parts isolated in the problem domain
  should be isolated as separate Erlang processes, limiting "blast radius".
- DSG-002 - Prefer Letting the User Decide What Warrants an Exception
  (priority: Recommendation; CWE-389, A10:2025). Follow the
  `{ok, Result} | {error, Reason}` convention; prefer pattern matching over
  `try ... catch error:_`.
- DSG-005 - Do Not Use Deprecated Functionality (priority: Medium; CWE-242,
  CWE-477, CWE-676). Heed compiler deprecation warnings.
- DSG-007 - Know Your Data (priority: Medium; see Ports/NIF section above for
  full CWE list). Mark data trusted/untrusted; consider `-nominal` types.
- DSG-008 - Avoid Designing Unrealistic Interfaces (priority: Recommendation;
  CWE-362). Do not promise absolute consistency/availability (PACELC); document
  trade-offs and recovery strategies; Erlang acts as a distributed system
  internally.
- DSG-009 - Consider Providing Methods to Control Resource Consumption
  (priority: Medium; CWE-770, API4:2023). E.g. configurable upper bound on
  concurrent sessions.
- DSG-010 - Avoid Name Races (priority: Recommendation; CWE-386). Registered
  process/table names may terminate at any time; look up the identifier once
  (`whereis`/`ets:whereis`) and use the identifier for subsequent operations.
- LNG-001 - Prefer Tuples Over Exporting Variables (priority: Recommendation).
  Erlang does not employ lexical scoping; return a tuple instead of relying on
  variables defined in "inner" expressions being available in "outer"
  expressions.
- LNG-002 - Do Not Use catch (priority: Recommendation; CWE-253, CWE-480,
  A10:2025). The legacy `catch` cannot distinguish between `throw/1` and a
  normal return; `gen_server` will unintentionally accept any documented return
  value when thrown. Use `try ... catch ... end`. From OTP 29 the compiler
  raises warnings for legacy `catch` by default.
- LNG-003 - Do Not Use the Legacy and and or Operators (priority:
  Recommendation; CWE-783). Use `andalso`/`orelse` (or `,`/`;`). Legacy `and`
  /`or` have higher precedence than in most languages (e.g.
  `X and Y =:= 3` parses as `(X and Y) =:= 3`), silently failing in guards.
  From OTP 29 the compiler can warn via `warn_obsolete_bool_op`.
- MSC-001 - Do Not Abuse persistent_term (priority: Medium). Modifying a
  persistent_term comes at extreme performance cost (every process scanned for
  GC); use only for values that rarely, if ever, change.
- MSC-002 - Consider Path Traversal Attacks (priority: Medium; CWE-22, CWE-59).
  Erlang file routines do not protect against path traversal by default. Use
  `filelib:safe_relative_path/2` for paths from untrusted sources.
- MSC-004 - Protect Sensitive Data (priority: Medium; CWE-209, CWE-532). Erlang
  logs many things by default. Protect sensitive data via: `private` option for
  ETS tables; `process_flag(sensitive, true)` (disables introspection, tracing,
  crash-dump inclusion); `format_status/2` callback for gen* behaviors; wrap
  secrets in a zero-arity fun(); catch errors in sensitive sections and discard
  stack-frame arguments before re-raising. Note neither approach prevents
  leakage through a crash/core dump.
- MSC-006 - Consider "Link Following" Attacks (priority: Medium; CWE-22,
  CWE-59, CWE-61). Use `filelib:safe_relative_path/2`; beware TOCTOU races
  across filesystem operations; on shared folder structures ensure only one
  entity has access.
- MSC-007 - Avoid Using Debug Functionality in Production (priority: Critical;
  CWE-242, CWE-489, A06:2025). See Code injection section above.

### Application-specific guidelines

- crypto Application: Initialize via `application:start(crypto)` only (do not
  call any crypto function, including `crypto:start/0`, beforehand). Use the
  `fips_mode` application parameter instead of `crypto:enable_fips_mode/1`. Do
  not use `crypto:rand_uniform/2` (CWE-338); use `rand:uniform_s/2` with a
  cryptographically strong generator (`crypto:rand_seed_s/0`). Do not use
  `rsa_pkcs1_padding` with `crypto:private_encrypt/4`,
  `crypto:private_decrypt/4`, `crypto:public_encrypt/4`, `crypto:public_decrypt/4`;
  for signatures use `crypto:sign/4`/`crypto:verify/5`.
- ssl Application: secure by default; be careful enabling legacy functionality
  (it weakens security). Consider disabling TLS 1.2 in favor of TLS 1.3 when
  you control both ends. See TLS Hardening Guide.
- ssh Application: see the ssh documentation's hardening chapter.
- public_key Application: do not use `rsa_pkcs1_padding` with
  `public_key:encrypt_private/2,3`, `public_key:decrypt_private/2,3`,
  `public_key:encrypt_public/2,3`, `public_key:decrypt_public/2,3`; for
  signatures use `public_key:sign/4`/`public_key:verify/5`.
- xmerl Application: `xmerl_scan` dynamically produces new atoms and is not
  suitable for decoding XML from untrusted sources. When parsing untrusted XML,
  prevent XML entity attacks by disabling parsing of entities via the
  `disallow_entities` option in `xmerl_sax_parser`.

## Verbatim quotes

> "All loaded code is assumed to be trusted. There is no built-in sand-boxing
> mechanism for running untrusted Erlang code. Any code loaded and executed
> within the environment has unrestricted access to the system."

> "This means that a malicious BEAM module can do anything, including breaking
> the memory safety protections of the runtime system and crashing the virtual
> machine."

> "All nodes connected through Erlang distribution are assumed to be trusted,
> and have unrestricted access to all other connected systems."

> "Crucially, there is no authentication built into the default distribution
> protocol, merely a 'cookie' mechanism that prevents the unintentional mixing
> of Erlang clusters on the same network. For secure communication over an
> unsecured network, the distribution should be configured to use TLS with
> client certificate verification."

> "The arguments given to many Erlang functions are assumed to be trusted, so
> providing valid-appearing but nonsensical input to a function can have it
> produce valid-appearing garbage instead of crashing or returning an error."

> "Erlang is a memory-safe language." (CWE-416, CWE-465, CWE-1218 cannot occur;
> classical data race CWE-362 cannot occur unless the programmer explicitly
> built a shared resource; uninitialized-variable errors cannot occur.)

> "The amount of atoms in the system is limited (see system limits and CWE-770)
> and cannot be reclaimed once created. That is, if you dynamically create a
> large amount of atoms, the system might crash."

> (DSG-003) "the `binary_to_atom/1,2` and `list_to_atom/1` functions will
> create a new atom if it does not already exist. Doing so without care might
> cause the system to crash."

> (DSG-003) "`binary_to_term/2` with the `safe` option will prevent new atoms
> from being created. However, note that even if the `safe` option is used and
> the data originates from an untrusted source, it still has to be validated
> and sanitized, since it can still be harmful to the Erlang application in
> other ways. In general, it is best to avoid using such functions altogether on
> untrusted data, even with the `safe` option."

> (DSG-011) "Erlang/OTP provides various functionality that serializes and
> deserializes general Erlang terms. Such functionality is intended to be used
> in a trusted environment and is not suitable for communication with
> untrusted entities."

> (DEP-001) "By default, communication is performed over an unencrypted TCP
> connection with a rudimentary cookie based authentication only present in
> order to prevent mistakes. This configuration should only be used in a
> trusted network."

> (DEP-001) "Once a node is connected to the cluster, it gains complete access
> to the resources and operations of all other nodes, making node
> trustworthiness a critical security consideration."

> (MSC-003) "Calling `open_port/2` with the `{spawn, _}` argument will either
> start an instance of a driver, or spawn an external program using the passed
> command line. When spawning an external program, some platforms will start a
> shell, which will search in the PATH in order to find the program."

> (MSC-005) "Match specifications, such as those used with ETS, are vulnerable
> to injection attacks if they are constructed based on untrusted input."

> (DSG-004) "Undocumented functions or functionality must never be used. This
> includes undocumented arguments to documented functions and undocumented
> system services."

> (DSG-006) "Unsafe Functions must not be used, and Potentially Unsafe
> Functions should only be used in a safe manner, and are best avoided if
> possible."

> (Native Code) "Drivers and NIF libraries are typically written in C or C++
> and have, if not full access, access to a large part of the runtime system
> internals."

## Version notes

- Page meta: project = "Erlang System Documentation v29.0.2".
- Source markdown on GitHub: branch/tag `OTP-29.0.2`,
  `system/doc/design_principles/secure_coding.md`.
- Built using ExDoc v0.40.3. Copyright © 1996-2026 Ericsson AB.
- OTP 28.0+: configure script tries to enable most OpenSSF-recommended C/C++
  hardening flags on Unix-like systems (DEP-002).
- OTP 29: compiler raises warnings for legacy `catch` by default (LNG-002); can
  warn for legacy `and`/`or` via `warn_obsolete_bool_op` option (LNG-003).
- OWASP references use the 2025 Top 10 and 2023 API Security Top 10 editions.
- CWE references use the 2025 CWE Top 25 and On The Cusp list.

## Discovered links

### Relevant (crawl later)

- https://www.erlang.org/doc/system/design_principles.html — Design Principles
  (referenced for supervision/error-handling background and A06:2025).
- https://erlang.org/download/otp_versions_tree.html — OTP Versions Tree
  (maintained releases + CVEs; DEP-003).
- https://erlang.org/download/vex/ — VEX documents for maintained OTP releases
  (DEP-003).
- https://github.com/erlang/otp/blob/OTP-29.0.2/system/doc/design_principles/secure_coding.md
  — source markdown for this chapter (verbatim rule text).
- (External, not erlang.org but high-signal) OWASP Top 10 2025 introduction:
  https://owasp.org/Top10/2025/0x00_2025-Introduction/
- (External) CWE Top 25 2025: https://cwe.mitre.org/top25/archive/2025/2025_cwe_top25.html

### Skipped

- All individual CWE definition pages (https://cwe.mitre.org/data/definitions/*.html)
  — reference material, not BEAM-specific crawl targets.
- All individual OWASP Top 10 / API Security 2023/2025 detail pages — external,
  not BEAM-specific.
- https://www.erlang.org/doc/system/secure_coding.html — self (canonical).
- https://erlang.org, https://www.ericsson.com, https://mitre.org,
  https://owasp.org/, https://erlangforums.com/,
  https://erlang.org/pipermail/erlang-announce, https://github.com/elixir-lang/ex_doc,
  https://en.wikipedia.org/wiki/PACELC_design_principle,
  https://en.wikipedia.org/wiki/Two_Generals%27_Problem — organizational /
  encyclopedic, not crawl targets.
- /assets/css/algolia-typeahead.css — site asset.
