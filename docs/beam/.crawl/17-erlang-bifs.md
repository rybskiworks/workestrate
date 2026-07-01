# Crawl: erts/erlang.html (focused)
- seed_url: https://www.erlang.org/doc/apps/erts/erlang.html
- canonical_url: https://www.erlang.org/doc/apps/erts/erlang.html
- family: Erlang/OTP ERTS BIF docs
- fetch: HTTP 200
- otp_version: OTP 29 (ERTS v17.0.2; major-vsn 29)
- feeds_docs: processes-and-messages.md, links-monitors-and-exits.md, runtime-debugging.md, timers.md

## Purpose
The `erlang` module contains the Built-In Functions (BIFs) of the Erlang runtime system (ERTS). This crawl captures ONLY the BIFs relevant to BEAM process lifecycle, concurrency, failure/signals, links/monitors/aliases, process flags, registration, runtime introspection/observability, and time/timers. Math, binary-conversion, term-comparison, code loading, and port-program BIFs are deliberately skipped.

## Process creation (signatures + spawn_opt options)

### spawn/1
spawn(Fun) (auto-imported) -spec spawn(Fun) -> pid () when Fun :: function (). Returns the process identifier of a new process started by the application of Fun to the empty list [] . Otherwise works like spawn/3 .
spawn(Fun) (auto-imported)
spawn(Fun) (auto-imported)

### spawn/2
spawn(Node, Fun) (auto-imported) -spec spawn(Node, Fun) -> pid () when Node :: node (), Fun :: function (). Returns the process identifier of a new process started by the application of Fun to the empty list [] on Node . If Node does not exist, a useless pid
is returned. Otherwise works like spawn/3 .
spawn(Node, Fun) (auto-imported)
spawn(Node, Fun) (auto-imported)

### spawn/3
spawn(Module, Function, Args) (auto-imported) -spec spawn(Module, Function, Args) -> pid ()
               when Module :: module (), Function :: atom (), Args :: [ term ()]. Returns the process identifier of a new process started by the application of Module:Function to Args . error_handler:undefined_function(Module, Function, Args) is
 evaluated by the new process if Module:Function/Arity does not exist
(where Arity is the length of Args ). The error handler can be redefined
(see process_flag/2 ). If error_handler is undefined, or the user has redefined the default error_handler and its replacement is undefined, a failure with reason undef occurs. Example: > spawn ( speed , regulator , [ high_speed , thin_cut ] ) . < 0.13 . 1 >
spawn(Module, Function, Args) (auto-imported)
spawn(Module, Function, Args) (auto-imported)

### spawn/4
spawn(Node, Module, Function, Args) (auto-imported) -spec spawn(Node, Module, Function, Args) -> pid ()
               when Node :: node (), Module :: module (), Function :: atom (), Args :: [ term ()]. Returns the process identifier (pid) of a new process started by the application
of Module:Function to Args on Node . If Node does not exist, a useless
pid is returned. Otherwise works like spawn/3 .
spawn(Node, Module, Function, Args) (auto-imported)
spawn(Node, Module, Function, Args) (auto-imported)

### spawn_link/1
spawn_link(Fun) (auto-imported) -spec spawn_link(Fun) -> pid () when Fun :: fun(() -> term ()). Returns the process identifier of a new process started by the application of Fun to the empty list [] . A link is created between the calling process and
the new process, atomically. Otherwise works like spawn/3 .
spawn_link(Fun) (auto-imported)
spawn_link(Fun) (auto-imported)

### spawn_link/2
spawn_link(Node, Fun) (auto-imported) -spec spawn_link(Node, Fun) -> pid () when Node :: node (), Fun :: fun(() -> term ()). Returns the process identifier (pid) of a new process started by the application
of Fun to the empty list [] on Node . A link is created between the calling
process and the new process, atomically. If Node does not exist, a useless pid
is returned and an exit signal with reason noconnection is sent to the calling
process. Otherwise works like spawn/3 .
spawn_link(Node, Fun) (auto-imported)
spawn_link(Node, Fun) (auto-imported)

### spawn_link/3
spawn_link(Module, Function, Args) (auto-imported) -spec spawn_link(Module, Function, Args) -> pid ()
                    when Module :: module (), Function :: atom (), Args :: [ term ()]. Returns the process identifier of a new process started by the application of Module:Function to Args . A link is created between the calling process and
the new process, atomically. Otherwise works like spawn/3 .
spawn_link(Module, Function, Args) (auto-imported)
spawn_link(Module, Function, Args) (auto-imported)

### spawn_link/4
spawn_link(Node, Module, Function, Args) (auto-imported) -spec spawn_link(Node, Module, Function, Args) -> pid ()
                    when Node :: node (), Module :: module (), Function :: atom (), Args :: [ term ()]. Returns the process identifier (pid) of a new process started by the application
of Module:Function to Args on Node . A link is created between the calling
process and the new process, atomically. If Node does not exist, a useless pid
is returned and an exit signal with reason noconnection is sent to the calling
process. Otherwise works like spawn/3 .
spawn_link(Node, Module, Function, Args) (auto-imported)
spawn_link(Node, Module, Function, Args) (auto-imported)

### spawn_monitor/1
spawn_monitor(Fun) (auto-imported) -spec spawn_monitor(Fun) -> { pid (), reference ()} when Fun :: function (). Returns the process identifier of a new process, started by the application of Fun to the empty list [] , and a reference for a monitor created to the new
process. Otherwise works like spawn/3 .
spawn_monitor(Fun) (auto-imported)
spawn_monitor(Fun) (auto-imported)

### spawn_monitor/2
spawn_monitor(Node, Fun) (auto-imported) (since OTP 23.0) -spec spawn_monitor(Node, Fun) -> { pid (), reference ()} when Node :: node (), Fun :: function (). Returns the process identifier of a new process, started by the application of Fun to the empty list [] on the node Node , and a reference for a monitor
created to the new process. Otherwise works like spawn/3 . If the node identified by Node does not support distributed spawn_monitor() ,
the call will fail with a notsup exception.
spawn_monitor(Node, Fun) (auto-imported) (since OTP 23.0)
spawn_monitor(Node, Fun) (auto-imported) (since OTP 23.0)

### spawn_monitor/3
spawn_monitor(Module, Function, Args) (auto-imported) -spec spawn_monitor(Module, Function, Args) -> { pid (), reference ()}
                       when Module :: module (), Function :: atom (), Args :: [ term ()]. A new process is started by the application of Module:Function to Args . The
process is monitored at the same time. Returns the process identifier and a
reference for the monitor. Otherwise works like spawn/3 .
spawn_monitor(Module, Function, Args) (auto-imported)
spawn_monitor(Module, Function, Args) (auto-imported)

### spawn_monitor/4
spawn_monitor(Node, Module, Function, Args) (auto-imported) (since OTP 23.0) -spec spawn_monitor(Node, Module, Function, Args) -> { pid (), reference ()}
                       when Node :: node (), Module :: module (), Function :: atom (), Args :: [ term ()]. A new process is started by the application of Module:Function to Args on
the node Node . The process is monitored at the same time. Returns the process
identifier and a reference for the monitor. Otherwise works like spawn/3 . If the node identified by Node does not support distributed spawn_monitor() ,
the call will fail with a notsup exception.
spawn_monitor(Node, Module, Function, Args) (auto-imported) (since OTP 23.0)
spawn_monitor(Node, Module, Function, Args) (auto-imported) (since OTP 23.0)

### spawn_opt/2
spawn_opt(Fun, Options) (auto-imported) -spec spawn_opt(Fun, Options) -> pid () | { pid (), reference ()}
                   when Fun :: function (), Options :: [ spawn_opt_option ()]. Returns the process identifier (pid) of a new process started by the application
of Fun to the empty list [] . Otherwise works like spawn_opt/4 . If option monitor is specified, the newly created process is monitored, and
both the pid and reference for the monitor are returned.
spawn_opt(Fun, Options) (auto-imported)
spawn_opt(Fun, Options) (auto-imported)

### spawn_opt/3
spawn_opt(Node, Fun, Options) (auto-imported) -spec spawn_opt(Node, Fun, Options) -> pid () | { pid (), reference ()}
                   when
                       Node :: node (),
                       Fun :: function (),
                       Options :: [monitor | {monitor, [ monitor_option ()]} | link | OtherOption],
                       OtherOption :: term (). Returns the process identifier (pid) of a new process started by the application
of Fun to the empty list [] on Node . If Node does not exist, a useless
pid is returned. Otherwise works like spawn_opt/4 . Valid options depends on what options are supported by the node identified by Node . A description of valid Option s for the local node of current OTP
version can be found in the documentation of spawn_opt/4 .
spawn_opt(Node, Fun, Options) (auto-imported)
spawn_opt(Node, Fun, Options) (auto-imported)

### spawn_opt/4
spawn_opt(Module, Function, Args, Options) (auto-imported) -spec spawn_opt(Module, Function, Args, Options) -> Pid | {Pid, MonitorRef}
                   when
                       Module :: module (),
                       Function :: atom (),
                       Args :: [ term ()],
                       Options :: [ spawn_opt_option ()],
                       Pid :: pid (),
                       MonitorRef :: reference (). Works as spawn/3 , except that an extra option list is specified when creating
the process. If option monitor is specified, the newly created process is monitored, and
both the pid and reference for the monitor are returned. Options: link - Sets a link to the parent process (like spawn_link/3 does). monitor - Monitors the new process (like monitor(process, Pid) does). A {Pid, MonitorRef} tuple will
be returned instead of just a Pid . {monitor, MonitorOpts} - Monitors the new process with options (like monitor(process, Pid, MonitorOpts) does). A {Pid, MonitorRef} tuple will be returned instead of just a Pid . {priority, Level} - Sets the priority of the new process. Equivalent to
executing process_flag(priority, Level) in the start function of the new process, except that the priority is set
before the process is selected for execution for the first time. For more
information on priorities, see process_flag(priority, Level) . {fullsweep_after, Number} - Useful only for performance tuning. Do not
use this option unless you know that there is problem with execution times or
memory consumption, and ensure that the option improves matters. The Erlang runtime system uses a generational garbage collection scheme, using
an "old heap" for data that has survived at least one garbage collection. When
there is no more room on the old heap, a fullsweep garbage collection is done. Option fullsweep_after makes it possible to specify the maximum number of
generational collections before forcing a fullsweep, even if there is room on
the old heap. Setting the number to zero disables the general collection
algorithm, that is, all live data is copied at every garbage collection. A few cases when it can be useful to change fullsweep_after : If binaries that are no longer used are to be thrown away as soon as
possible. (Set Number to zero.) A process that mostly have short-lived data is fullsweeped seldom or never,
that is, the old heap contains mostly garbage. To ensure a fullsweep
occasionally, set Number to a suitable value, such as 10 or 20. In 
...[truncated]

### spawn_opt/5
spawn_opt(Node, Module, Function, Args, Options) (auto-imported) -spec spawn_opt(Node, Module, Function, Args, Options) -> pid () | { pid (), reference ()}
                   when
                       Node :: node (),
                       Module :: module (),
                       Function :: atom (),
                       Args :: [ term ()],
                       Options :: [monitor | {monitor, [ monitor_option ()]} | link | OtherOption],
                       OtherOption :: term (). Returns the process identifier (pid) of a new process started by the application
of Module:Function to Args on Node . If Node does not exist, a useless
pid is returned. Otherwise works like spawn_opt/4 . Valid options depends on what options are supported by the node identified by Node . A description of valid Option s for the local node of current OTP
version can be found in the documentation of spawn_opt/4 .
spawn_opt(Node, Module, Function, Args, Options) (auto-imported)
spawn_opt(Node, Module, Function, Args, Options) (auto-imported)

### t:spawn_opt_option/0
spawn_opt_option() -type spawn_opt_option() ::
          link |
          {link, LinkOpts :: [ link_option ()]} |
          monitor |
          {monitor, MonitorOpts :: [ monitor_option ()]} |
          {priority, Level :: priority_level ()} |
          {fullsweep_after, Number :: non_neg_integer ()} |
          {min_heap_size, Size :: non_neg_integer ()} |
          {min_bin_vheap_size, VSize :: non_neg_integer ()} |
          {max_heap_size, Size :: max_heap_size ()} |
          {message_queue_data, MQD :: message_queue_data ()} |
          {async_dist, Enabled :: boolean ()}. Options for spawn_opt() .
spawn_opt_option()
spawn_opt_option()

**spawn_opt options captured:** `link`, `monitor`, `{priority, low|normal|high|max}`, `{fullsweep_after, Number}`, `{min_heap_size, Size}`, `{max_heap_size, Size}`, `{message_queue_data, on_heap | off_heap}`, `{async_dist, boolean()}`. Note: `monitor` option is disallowed together with `link` (raises badarg).

## Messaging / exit / raise

### send/2
send(Dest, Msg) -spec send(Dest, Msg) -> Msg when Dest :: send_destination (), Msg :: term (). Sends a message and returns Msg . This is the same as using the send operator : Dest ! Msg . Dest can be a remote or local process identifier, an alias, a (local) port, a
locally registered name, or a tuple {RegName, Node} for a registered name at
another node. The function fails with a badarg run-time error if Dest is an atom name, but
this name is not registered. This is the only case when send fails for an
unreachable destination Dest (of correct type). Note For some important information about distributed signals, see the Blocking Signaling Over Distribution section in the Processes chapter of the Erlang Reference Manual .
send(Dest, Msg)
send(Dest, Msg)

### send/3
send(Dest, Msg, Options) -spec send(Dest, Msg, Options) -> Res
              when
                  Dest :: send_destination (),
                  Msg :: term (),
                  Options :: [nosuspend | noconnect | priority],
                  Res :: ok | nosuspend | noconnect. Either sends a message and returns ok , or does not send the message but
returns something else (see below). Otherwise the same as erlang:send/2 . For more detailed explanation and warnings, see erlang:send_nosuspend/2,3 . Options: nosuspend - If the sender would have to be suspended to do the send, nosuspend is returned instead. noconnect - If the destination node would have to be auto-connected to
do the send, noconnect is returned instead. priority - Since OTP 28.0 Send this message as a priority message. In order for the message to be
handled as a priority message by the
receiver, this option must be passed, and Dest must be an active priority alias . If Dest is an active priority alias, but this option is not passed, the
message will be handled as on ordinary message. The same is true, if this
option is passed, but Dest is not an active priority alias. Warning You very seldom need to resort to using priority messages and you may cause issues instead of solving issues if not used with care. For more information see, the Adding Messages to the Message Queue and the Enabling Priority Message Reception sections of the Erlang Reference Manual . Note For some important information about distributed signals, see the Blocking Signaling Over Distribution section in the Processes chapter of the Erlang Reference Manual . Warning As with erlang:send_nosuspend/2,3 : use with extreme care.
send(Dest, Msg, Options)
send(Dest, Msg, Options)

### send_after/3
send_after(Time, Dest, Msg) -spec send_after(Time, Dest, Msg) -> TimerRef
                    when
                        Time :: non_neg_integer (),
                        Dest :: pid () | atom (),
                        Msg :: term (),
                        TimerRef :: reference (). Equivalent to erlang:send_after(Time, Dest, Msg, []) .
send_after(Time, Dest, Msg)
send_after(Time, Dest, Msg)

### send_after/4
send_after(Time, Dest, Msg, Options) (since OTP 18.0) -spec send_after(Time, Dest, Msg, Options) -> TimerRef
                    when
                        Time :: integer (),
                        Dest :: pid () | atom (),
                        Msg :: term (),
                        Options :: [Option],
                        Abs :: boolean (),
                        Option :: {abs, Abs},
                        TimerRef :: reference (). Starts a timer. When the timer expires, the message Msg is sent to the process
identified by Dest . Apart from the format of the time-out message, this
function works exactly as erlang:start_timer/4 .
send_after(Time, Dest, Msg, Options) (since OTP 18.0)
send_after(Time, Dest, Msg, Options) (since OTP 18.0)

### send_nosuspend/2
send_nosuspend(Dest, Msg) -spec send_nosuspend(Dest, Msg) -> boolean () when Dest :: send_destination (), Msg :: term (). Send a message without suspending the caller. Equivalent to erlang:send(Dest, Msg, [nosuspend]) , but returns true if the message was sent and false if the message was not sent because
the sender would have had to be suspended. This function is intended for send operations to an unreliable remote node
without ever blocking the sending (Erlang) process. If the connection to the
remote node (usually not a real Erlang node, but a node written in C or Java) is
overloaded, this function does not send the message and returns false . The same occurs if Dest refers to a local port that is busy. For all other
destinations (allowed for the ordinary send operator '!' ), this function sends
the message and returns true . This function is only to be used in rare circumstances where a process
communicates with Erlang nodes that can disappear without any trace, causing the
TCP buffers and the drivers queue to be over-full before the node is shut down
(because of tick time-outs) by net_kernel . The normal reaction to take when
this occurs is some kind of premature shutdown of the other node. Notice that ignoring the return value from this function would result in an unreliable message passing, which is contradictory to the Erlang programming
model. The message is not sent if this function returns false . In many systems, transient states of overloaded queues are normal. Although this
function returns false does not mean that the other node is guaranteed to be
non-responsive, it could be a temporary overload. Also, a return value of true does only mean that the message can be sent on the (TCP) channel without
blocking; the message is not guaranteed to arrive at the remote node. For a
disconnected non-responsive node, the return value is true (mimics the
behavior of operator ! ). The expected behavior and the actions to take when
the function returns false are application- and hardware-specific. Warning Use with extreme care.
send_nosuspend(Dest, Msg)
send_nosuspend(Dest, Msg)

### send_nosuspend/3
send_nosuspend(Dest, Msg, Options) -spec send_nosuspend(Dest, Msg, Options) -> boolean ()
                        when Dest :: send_destination (), Msg :: term (), Options :: [noconnect]. Equivalent to erlang:send(Dest, Msg, [nosuspend | Options]) , but
with a Boolean return value. This function behaves like erlang:send_nosuspend/2 , but
takes a third parameter, a list of options. The only option is noconnect ,
which makes the function return false if the remote node is not currently
reachable by the local node. The normal behavior is to try to connect to the
node, which can stall the process during a short period. The use of option noconnect makes it possible to be sure not to get the slightest delay when
sending to a remote process. This is especially useful when communicating with
nodes that expect to always be the connecting part (that is, nodes written in C
or Java). Whenever the function returns false (either when a suspend would occur or when noconnect was specified and the node was not already connected), the message
is guaranteed not to have been sent. Warning Use with extreme care.
send_nosuspend(Dest, Msg, Options)
send_nosuspend(Dest, Msg, Options)

### exit/1
exit(Reason) (auto-imported) -spec exit(Reason) -> no_return () when Reason :: term (). Raises an exception of class exit with exit reason Reason . As evaluating this function causes an exception to be raised, it has no return value. The intent of the exception class exit is that the current process should be
stopped (for example when a message telling a process to stop is received). This function differ from error/1,2,3 by causing an exception of
a different class and by having a reason that does not include the list of
functions from the call stack. See the guide about errors and error handling for
additional information. Example: 1> exit ( foobar ) . ** exception exit: foobar 2> catch exit ( foobar ) . { 'EXIT' , foobar } Note If a process calls exit(kill) and does not catch the exception,
it will terminate with exit reason kill and also emit exit signals with exit
reason kill (not killed ) to all linked processes. Such exit signals with
exit reason kill can be trapped by the linked processes. Note that this
means that signals with exit reason kill behave differently depending on how
they are sent because the signal will be untrappable if a process sends such a
signal to another process with erlang:exit_signal/2 .
exit(Reason) (auto-imported)
exit(Reason) (auto-imported)

### exit/2
exit(Dest, Reason) (auto-imported) -spec exit(Dest, Reason) -> true when Dest :: pid () | port () | reference (), Reason :: term (). Old form of exit_signal/2 , with a quirk when sender and receiver are the same. Note The function erlang:exit/2 is named similarly to erlang:exit/1 but provides very different functionality. The erlang:exit/1 function should be used
when the intent is to stop the current process by raising an exception of class exit . The erlang:exit_signal/2 function, or the old form erlang:exit/2 , should be used
when the intent is to send an exit signal to another process. Note also that erlang:exit/1 raises an exception that can be caught, while erlang:exit_signal/2 does not cause any exception to be raised. Warning This function has a quirk: When a process P sends an exit signal with reason normal to itself using this function, that is, erlang:exit(self(), normal) , the behavior is
as follows: P exits with reason normal if P is not trapping exits. If P is trapping exits , the exit signal is transformed
into a message {'EXIT', From, normal} , where From is P 's process
identifier, and delivered to P 's message queue. Note that this differs from when a process sends an exit signal with reason normal to another process than itself (see exit_signal/2 for details). This behavior is kept
for backward compatibility reasons. Use exit_signal/2 for new code.
exit(Dest, Reason) (auto-imported)
exit(Dest, Reason) (auto-imported)

### exit_signal/2
exit_signal(Pid, Reason) (auto-imported) (since OTP 29.0) -spec exit_signal(Pid, Reason) -> true when Pid :: pid () | port () | reference (), Reason :: term (). Sends an exit signal with exit reason Reason to the process or port identified
by Dest . If Dest is a reference, the exit signal will only affect the
identified process if the reference is an active process alias of a process
executing on an OTP 28.0 node or newer. Let P be the process or port identified by Dest . The following behavior
applies if Reason is any term except normal or kill : If P is not trapping exits , P exits with exit reason Reason . If P is trapping exits , the exit signal is transformed
into a message {'EXIT', From, Reason} , where From is the process
identifier of the process that sent the exit signal, and delivered to the
message queue of P . The following behavior applies if Reason is the term normal : The signal has no effect if P is not trapping exits. If P is trapping exits , the exit signal is transformed
into a message {'EXIT', From, normal} , where From is the process
identifier of the process that sent the exit signal, and delivered to P 's
message queue. If Reason is the atom kill , that is, if exit(Dest, kill) is
called, an untrappable exit signal is sent to the process that is identified by Dest , which unconditionally exits with exit reason killed . The exit reason is
changed from kill to killed to hint to linked processes that the killed
process got killed by a call to exit(Dest, kill) . Note For some important information about distributed signals, see the Blocking Signaling Over Distribution section in the Processes chapter of the Erlang Reference Manual .
exit_signal(Pid, Reason) (auto-imported) (since OTP 29.0)
exit_signal(Pid, Reason) (auto-imported) (since OTP 29.0)

### exit_signal/3
exit_signal(Dest, Reason, OptList) (auto-imported) (since OTP 29.0) -spec exit_signal(Dest, Reason, OptList) -> true
                     when Dest :: pid () | port () | reference (), Reason :: term (), OptList :: [priority]. Provides an option list for modification of the functionality provided by the exit_signal/2 BIF. The Dest and Reason arguments has the same meaning as when
passed to exit_signal/2 . Currently available options: priority -- Since OTP 28.0 Send this exit signal as a priority exit signal. In order for
the signal to be handled as a priority EXIT message by the receiver, this option must be passed, Dest must be an active priority alias and the receiver must be trapping exits . If Dest is an active priority alias, but this option is not passed, the exit
signal will be handled as on ordinary exit signal. The same is true, if this
option is passed, but Dest is not an active priority alias. Warning You very seldom need to resort to using priority messages and you may cause issues instead of solving issues if not used with care. For more information see, the Adding Messages to the Message Queue and the Enabling Priority Message Reception sections of the Erlang Reference Manual .
exit_signal(Dest, Reason, OptList) (auto-imported) (since OTP 29.0)
exit_signal(Dest, Reason, OptList) (auto-imported) (since OTP 29.0)

### raise/3
raise(Class, Reason, Stacktrace) -spec raise(Class, Reason, Stacktrace) -> badarg
               when Class :: error | exit | throw, Reason :: term (), Stacktrace :: raise_stacktrace (). Raises an exception of the specified class, reason, and call stack backtrace
( stacktrace ). Class is error , exit , or throw . So, if it were not for the stacktrace, erlang:raise(Class, Reason, Stacktrace) is equivalent to erlang:Class(Reason) (given that Class is a valid class). Reason can be any term. Stacktrace is a list as provided in a try-catch clause. try . . . catch Class : Reason : Stacktrace -> . . . end That is, a list of four-tuples {Module, Function, Arity | Args, ExtraInfo} ,
where Module and Function are atoms, and the third element is an integer
arity or an argument list. The stacktrace can also contain {Fun, Args, ExtraInfo} tuples, where Fun is a local fun and Args is an
argument list. Element ExtraInfo at the end is optional. Omitting it is equivalent to
specifying an empty list. The stacktrace is used as the exception stacktrace for the calling process; it
is truncated to the current maximum stacktrace depth. As evaluating this function causes the process to terminate, it has no return
value unless the arguments are invalid, in which case the function returns the
error reason badarg . If you want to be sure not to return, you can call error(erlang:raise(Class, Reason, Stacktrace)) and hope to
distinguish exceptions later. See the reference manual about errors and error handling for more information about exception classes and how to catch exceptions.
raise(Class, Reason, Stacktrace)
raise(Class, Reason, Stacktrace)

### t:send_destination/0
send_destination() -type send_destination() :: pid () | reference () | port () | (RegName :: atom ()) | {RegName :: atom (), Node :: node ()}. The destination for a send operation. This can be a remote or local process identifier, a (local) port, a reference
denoting a process alias, a locally registered name, or a tuple {RegName, Node} for a registered name at another node.
send_destination()
send_destination()

### t:raise_stacktrace/0
raise_stacktrace() -type raise_stacktrace() ::
          [{ module (), atom (), arity () | [ term ()]} | { function (), arity () | [ term ()]}] | stacktrace (). A extended stacktrace/0 that can be passed to raise/3 .
raise_stacktrace()
raise_stacktrace()

## Links / monitors / aliases

### link/1
link(PidOrPort) (auto-imported) -spec link(PidOrPort) -> true when PidOrPort :: pid () | port (). Sets up and activates a link between the calling process and another process or
a port identified by PidOrPort . We will from here on call the identified process or port linkee. If the linkee
is a port, it must reside on the same node as the caller. If one of the participants of a link terminates, it will send an exit signal to
the other participant. The exit signal will contain the exit reason of the
terminated participant. Other cases when exit signals are triggered due to a
link are when no linkee exist ( noproc exit reason) and when the connection
between linked processes on different nodes is lost or cannot be established
( noconnection exit reason). An existing link can be removed by calling unlink/1 . For more information on
links and exit signals due to links, see the Processes chapter in the Erlang
Reference Manual : Links Sending Exit Signals Receiving Exit Signals For historical reasons, link/1 has a strange semi-synchronous
behavior when it is "cheap" to check if the linkee exists or not, and the caller
does not trap exits . If the above is true
and the linkee does not exist, link/1 will raise a noproc error exception . The expected behavior would instead have been that link/1 returned true , and the caller later was sent an exit
signal with noproc exit reason, but this is unfortunately not the case. The noproc exception is not to be confused with
an exit signal with exit
reason noproc . Currently it is "cheap" to check if the linkee exists when it
is supposed to reside on the same node as the calling process. The link setup and activation is performed asynchronously. If the link already
exists, or if the caller attempts to create a link to itself, nothing is done. A
detailed description of the link protocol can be found in the Distribution Protocol chapter of the ERTS User's Guide . Note For some important information about distributed signals, see the Blocking Signaling Over Distribution section in the Processes chapter of the Erlang Reference Manual . Failure: badarg if PidOrPort does not identify a process or a node local port. noproc linkee does not exist and it is "cheap" to check if it exists as
described above.
link(PidOrPort) (auto-imported)
link(PidOrPort) (auto-imported)

### link/2
link(PidOrPort, OptList) (auto-imported) (since OTP 28.0) -spec link(PidOrPort, [ link_option ()]) -> true when PidOrPort :: pid () | port (). Provides an option list for modification of the link functionality provided by link/1 . The PidOrPort argument has the same meaning as when passed to link/1 . Currently available options: priority - Since OTP 28.0 Enables priority message reception of EXIT messages due to the link for the calling process. If the link
already exists without priority message reception enabled for the link,
priority message reception will be enabled on the existing link. If the link
already exists with priority message reception enabled and this option is not
passed or link/1 is called, priority message reception for this link will be
disabled. Note that priority message reception due to the link is only enabled for the
process that passed this option. If the linked process also wants to enable
priority message reception, it needs to call link/2 passing the priority option itself. Warning You very seldom need to resort to using priority messages and you may cause issues instead of solving issues if not used with care. For more information see the Adding Messages to the Message Queue section of the Erlang Reference Manual .
link(PidOrPort, OptList) (auto-imported) (since OTP 28.0)
link(PidOrPort, OptList) (auto-imported) (since OTP 28.0)

### unlink/1
unlink(Id) (auto-imported) -spec unlink(Id) -> true when Id :: pid () | port (). Removes a link between the calling process and another process or a port
identified by Id . We will from here on call the identified process or port unlinkee. A link can be set up using the link/1 BIF. For more information on links and
exit signals due to links, see the Processes chapter in the Erlang Reference
Manual : Links Sending Exit Signals Receiving Exit Signals Once unlink(Id) has returned, it is guaranteed that the link
between the caller and the unlinkee has no effect on the caller in the future
(unless the link is setup again). Note that if the caller is trapping exits , an {'EXIT', Id, ExitReason} message due to the link may have been placed in the
message queue of the caller before the unlink(Id) call
completed. Also note that the {'EXIT', Id, ExitReason} message may be the
result of the link, but may also be the result of the unlikee sending the caller
an exit signal by calling the exit_signal/2 BIF. Therefore, it may or may not be
appropriate to clean up the message queue after a call to unlink(Id) as follows, when trapping exits: unlink ( Id ) , receive { 'EXIT' , Id , _ } -> true after 0 -> true end The link removal is performed asynchronously. If such a link does not exist,
nothing is done. A detailed description of the link protocol can be found in the Distribution Protocol chapter of the ERTS User's Guide . Note For some important information about distributed signals, see the Blocking Signaling Over Distribution section in the Processes chapter of the Erlang Reference Manual . Failure: badarg if Id does not identify a process or a node local port.
unlink(Id) (auto-imported)
unlink(Id) (auto-imported)

### monitor/2
monitor(Type, Item) (auto-imported) -spec monitor(process, monitor_process_identifier ()) -> MonitorRef when MonitorRef :: reference ();
             (port, monitor_port_identifier ()) -> MonitorRef when MonitorRef :: reference ();
             (time_offset, clock_service) -> MonitorRef when MonitorRef :: reference (). Sends a monitor request of type Type to the entity identified by Item . If the monitored entity does not exist or it changes monitored state, the caller
of monitor/2 is notified by a message on the following format: { Tag , MonitorRef , Type , Object , Info } Note The monitor request is an asynchronous signal. That is, it takes time before
the signal reaches its destination. Type can be one of the following atoms: process , port or time_offset . A process or port monitor is triggered only once, after that it is removed
from both monitoring process and the monitored entity. Monitors are fired when
the monitored process or port terminates, does not exist at the moment of
creation, or if the connection to it is lost. If the connection to it is lost,
we do not know if it still exists. The monitoring is also turned off when demonitor/1 is called. A process or port monitor by name resolves the RegisteredName to pid/0 or port/0 only once at the moment of monitor instantiation, later changes to
the name registration will not affect the existing monitor. When a process or port monitor is triggered, a 'DOWN' message is sent that
has the following pattern: { 'DOWN' , MonitorRef , Type , Object , Info } In the monitor message MonitorRef and Type are the same as described
earlier, and: Object - The monitored entity, which triggered the event. When
monitoring a process or a local port, Object will be equal to the pid/0 or port/0 that was being monitored. When monitoring process or port by
name, Object will have format {RegisteredName, Node} where RegisteredName is the name which has been used with monitor/2 call and Node is local or remote node name (for
ports monitored by name, Node is always local node name). Info - Either the exit reason of the process, noproc (process or port
did not exist at the time of monitor creation), or noconnection (no
connection to the node where the monitored process resides). Monitoring a process - Creates monitor between the
current process and another process identified by Item , which can be a pid/0 (local or remote), an atom RegisteredName or a tuple {RegisteredName, Node} for a registered process, located elsewhere. Cha
...[truncated]

### monitor/3
monitor(Type, Item, Opts) (auto-imported) (since OTP 24.0) -spec monitor(process, monitor_process_identifier (), [ monitor_option ()]) -> MonitorRef
                 when MonitorRef :: reference ();
             (port, monitor_port_identifier (), [ monitor_option ()]) -> MonitorRef
                 when MonitorRef :: reference ();
             (time_offset, clock_service, [ monitor_option ()]) -> MonitorRef
                 when MonitorRef :: reference (). Provides an option list for modification of monitoring functionality provided by monitor/2 . The Type and Item arguments have the same meaning as when
passed to monitor/2 . Currently available options: {alias, UnaliasOpt} - The returned monitor reference will also become an
alias for the calling process. That is, the returned reference can be used for
sending messages to the calling process. See also alias/0 . The UnaliasOpt determines how the alias should be deactivated. explicit_unalias - Only an explicit call to unalias/1 will
deactivate the alias. demonitor - The alias will be automatically deactivated when the
monitor is removed. This either via an explicit call to demonitor/1 or
when it is automatically removed at the same time as a 'DOWN' message is
delivered due to the monitor. The alias can also still be deactivated via a
call to unalias/1 . reply_demonitor - The alias will be automatically deactivated when the
monitor is removed (see demonitor option above) or a reply message sent
via the alias is received. When a reply message is received via the alias
the monitor will also be automatically removed. This is useful in
client/server scenarios when a client monitors the server and will get the
reply via the alias. Once the response is received both the alias and the
monitor will be automatically removed regardless of whether the response is
a reply or a 'DOWN' message. The alias can also still be deactivated via a
call to unalias/1 . Note that if the alias is removed using
the unalias/1 BIF, the monitor will still be left active. Example: server ( ) -> receive { request , AliasReqId , Request } -> Result = perform_request ( Request ) , AliasReqId ! { reply , AliasReqId , Result } end , server ( ) . client ( ServerPid , Request ) -> AliasMonReqId = monitor ( process , ServerPid , [ { alias , reply_demonitor } ] ) , ServerPid ! { request , AliasMonReqId , Request } , %% Alias as well as monitor will be automatically deactivated if we %% receive a reply or a 'DOWN' message since we used 'reply_demon
...[truncated]

### demonitor/1
demonitor(MonitorRef) (auto-imported) -spec demonitor(MonitorRef) -> true when MonitorRef :: reference (). If MonitorRef is a reference that the calling process obtained by calling monitor/2 , this monitoring is turned off. If the monitoring is already turned
off, nothing happens. Once demonitor(MonitorRef) has returned, it is guaranteed
that no {'DOWN', MonitorRef, _, _, _} message, because of the monitor, will be
placed in the caller message queue in the future. However, a {'DOWN', MonitorRef, _, _, _} message can have been placed in the caller
message queue before the call. It is therefore usually advisable to remove such
a 'DOWN' message from the message queue after monitoring has been stopped. demonitor(MonitorRef, [flush]) can be used instead of demonitor(MonitorRef) if this cleanup is wanted. Note For some important information about distributed signals, see the Blocking Signaling Over Distribution section in the Processes chapter of the Erlang Reference Manual . Change Before Erlang/OTP R11B (ERTS 5.5) demonitor/1 behaved
completely asynchronously, that is, the monitor was active until the
"demonitor signal" reached the monitored entity. This had one undesirable
effect. You could never know when you were guaranteed not to receive a DOWN message because of the monitor. The current behavior can be viewed as two combined operations: asynchronously
send a "demonitor signal" to the monitored entity and ignore any future
results of the monitor. Failure: It is an error if MonitorRef refers to a monitoring started by
another process. Not all such cases are cheap to check. If checking is cheap,
the call fails with badarg , for example if MonitorRef is a remote reference.
demonitor(MonitorRef) (auto-imported)
demonitor(MonitorRef) (auto-imported)

### demonitor/2
demonitor(MonitorRef, OptionList) (auto-imported) -spec demonitor(MonitorRef, OptionList) -> boolean ()
                   when MonitorRef :: reference (), OptionList :: [Option], Option :: flush | info. The returned value is true unless info is part of OptionList . demonitor(MonitorRef, []) is equivalent to demonitor(MonitorRef) . Option s: flush - Removes (one) {_, MonitorRef, _, _, _} message, if there is
one, from the caller message queue after monitoring has been stopped. Calling demonitor(MonitorRef, [flush]) is equivalent to the
following, but more efficient: demonitor ( MonitorRef ) , receive { _ , MonitorRef , _ , _ , _ } -> true after 0 -> true end info - The returned value is one of the following: true - The monitor was found and removed. In this case, no 'DOWN' message corresponding to this monitor has been delivered and will not be
delivered. false - The monitor was not found and could not be removed. This
probably because someone already has placed a 'DOWN' message corresponding
to this monitor in the caller message queue. If option info is combined with option flush , false is returned if a
flush was needed, otherwise true . Change More options can be added in a future release. Failures: badarg - If OptionList is not a list. badarg - If Option is an invalid option. badarg - The same failure as for demonitor/1 .
demonitor(MonitorRef, OptionList) (auto-imported)
demonitor(MonitorRef, OptionList) (auto-imported)

### alias/0
alias() (auto-imported) (since OTP 24.0) -spec alias() -> Alias when Alias :: reference (). Equivalent to alias([]) .
alias() (auto-imported) (since OTP 24.0)
alias() (auto-imported) (since OTP 24.0)

### alias/1
alias(Opts) (auto-imported) (since OTP 24.0) -spec alias(Opts) -> Alias when Alias :: reference (), Opts :: [explicit_unalias | reply | priority]. Create an alias which can be used when sending messages to the process that
created the alias. When the alias has been deactivated, messages sent using the
alias will be dropped. An alias can be deactivated using unalias/1 . Currently available options for alias/1 : explicit_unalias - The alias can only be deactivated via a call to unalias/1 . This is also the default behaviour if no options
are passed or if alias/0 is called. reply - The alias will be automatically deactivated when a reply message
sent via the alias is received. The alias can also still be deactivated via a
call to unalias/1 . priority - Since OTP 28.0 The alias can be used for sending priority messages to the
process that created this alias. An alias created with this option is also
known as a priority process alias or shorter priority alias . Warning You very seldom need to resort to using priority messages and you may cause issues instead of solving issues if not used with care. For more information see, the Enabling Priority Message Reception section of the Erlang Reference Manual . Example: server ( ) -> receive { request , AliasReqId , Request } -> Result = perform_request ( Request ) , AliasReqId ! { reply , AliasReqId , Result } end , server ( ) . client ( ServerPid , Request ) -> AliasReqId = alias ( [ reply ] ) , ServerPid ! { request , AliasReqId , Request } , %% Alias will be automatically deactivated if we receive a reply %% since we used the 'reply' option... receive { reply , AliasReqId , Result } -> Result after 5000 -> unalias ( AliasReqId ) , %% Flush message queue in case the reply arrived %% just before the alias was deactivated... receive { reply , AliasReqId , Result } -> Result after 0 -> exit ( timeout ) end end . Note that both the server and the client in this example must be executing on at
least OTP 24 systems in order for this to work. For more information on process aliases see the Process Aliases section of
the Erlang Reference Manual .
alias(Opts) (auto-imported) (since OTP 24.0)
alias(Opts) (auto-imported) (since OTP 24.0)

### unalias/1
unalias(Alias) (auto-imported) (since OTP 24.0) -spec unalias(Alias) -> boolean () when Alias :: reference (). Deactivate the alias Alias previously created by the calling process. An alias can, for example, be created via alias/0 or monitor/3 . unalias/1 will always deactivate the alias regardless of
options used when creating the alias. Returns true if Alias was a currently active alias for current processes;
otherwise, false. For more information on process aliases see the Process Aliases section of
the Erlang Reference Manual .
unalias(Alias) (auto-imported) (since OTP 24.0)
unalias(Alias) (auto-imported) (since OTP 24.0)

### t:monitor_option/0
monitor_option() -type monitor_option() ::
          {alias, explicit_unalias | demonitor | reply_demonitor} | {tag, term ()} | priority. See monitor/3 .
monitor_option()
monitor_option()

### t:link_option/0
link_option() (since OTP 28.0) -type link_option() :: priority. See link/2 .
link_option() (since OTP 28.0)
link_option() (since OTP 28.0)

## process_flag values (full list)
Flag names accepted by `process_flag/2,3`: `trap_exit`, `priority` (low|normal|high|max), `save_calls`, `sensitive`, `message_queue_data` (on_heap|off_heap), `min_heap_size`, `max_heap_size`, `fullsweep_after`, `async_dist`.

### process_flag/2
process_flag(Flag, Value) (auto-imported) -spec process_flag(async_dist, Boolean) -> OldBoolean when Boolean :: boolean (), OldBoolean :: boolean ();
                  (trap_exit, Boolean) -> OldBoolean when Boolean :: boolean (), OldBoolean :: boolean ();
                  (error_handler, Module) -> OldModule when Module :: atom (), OldModule :: atom ();
                  (fullsweep_after, FullsweepAfter) -> OldFullsweepAfter
                      when FullsweepAfter :: non_neg_integer (), OldFullsweepAfter :: non_neg_integer ();
                  (min_heap_size, MinHeapSize) -> OldMinHeapSize
                      when MinHeapSize :: non_neg_integer (), OldMinHeapSize :: non_neg_integer ();
                  (min_bin_vheap_size, MinBinVHeapSize) -> OldMinBinVHeapSize
                      when MinBinVHeapSize :: non_neg_integer (), OldMinBinVHeapSize :: non_neg_integer ();
                  (max_heap_size, MaxHeapSize) -> OldMaxHeapSize
                      when MaxHeapSize :: max_heap_size (), OldMaxHeapSize :: max_heap_size ();
                  (message_queue_data, MQD) -> OldMQD
                      when MQD :: message_queue_data (), OldMQD :: message_queue_data ();
                  (priority, Level) -> OldLevel
                      when Level :: priority_level (), OldLevel :: priority_level ();
                  (save_calls, N) -> OldN when N :: 0..10000, OldN :: 0..10000;
                  (sensitive, Boolean) -> OldBoolean when Boolean :: boolean (), OldBoolean :: boolean ();
                  ({monitor_nodes, term ()}, term ()) -> term ();
                  (monitor_nodes, term ()) -> term (). Sets the process flag indicated to the specified value. Returns the previous value
of the flag. Flag is one of the following: process_flag ( async_dist , boolean ( ) ) Enable or disable fully asynchronous distributed signaling for the calling
process. When disabled, which is the default, the process sending a distributed
signal will block in the send operation if the buffer for the distribution
channel reach the distribution buffer busy limit . The
process will remain blocked until the buffer shrinks enough. This might in some
cases take a substantial amount of time. When async_dist is enabled, send
operations of distributed signals will always buffer the signal on the outgoing
distribution channel and then immediately return. That is, these send operations
will never block the sending process. Note Since no flow control is enforced by the runtime syste
...[truncated]

### process_flag/3
process_flag(Pid, Flag, Value) (auto-imported) -spec process_flag(Pid, Flag, Value) -> OldValue
                      when
                          Pid :: pid (),
                          Flag :: save_calls,
                          Value :: non_neg_integer (),
                          OldValue :: non_neg_integer (). Sets certain flags for the process Pid , in the same manner as process_flag/2 . Returns the old value of the flag. The valid values for Flag are only a subset of those allowed in process_flag/2 ,
namely save_calls . Failure: badarg if Pid is not a local process.
process_flag(Pid, Flag, Value) (auto-imported)
process_flag(Pid, Flag, Value) (auto-imported)

### process_info/1
process_info(Pid) (auto-imported) -spec process_info(Pid) -> Info
                      when
                          Pid :: pid (),
                          Info :: [InfoTuple] | undefined,
                          InfoTuple :: process_info_result_item (). Returns a list containing InfoTuple s with miscellaneous information about the
process identified by Pid , or undefined if the process is not alive. The order of the InfoTuple s is undefined and all InfoTuple s are not
mandatory. The InfoTuple s part of the result can be changed without prior
notice. The InfoTuple s with the following items are part of the result: current_function initial_call status message_queue_len links dictionary trap_exit error_handler priority group_leader total_heap_size heap_size stack_size reductions garbage_collection If the process identified by Pid has a registered name, also an InfoTuple with item registered_name is included. For information about specific InfoTuple s, see process_info/2 . Warning This BIF is intended for debugging only . For all other purposes, use process_info/2 . Failure: badarg if Pid is not a local process.
process_info(Pid) (auto-imported)
process_info(Pid) (auto-imported)

### process_info/2
process_info(Pid, ItemSpec) (auto-imported) -spec process_info(Pid, Item) -> InfoTuple | [] | undefined
                      when
                          Pid :: pid (),
                          Item :: process_info_item (),
                          InfoTuple :: process_info_result_item ();
                  (Pid, ItemList) -> InfoTupleList | [] | undefined
                      when
                          Pid :: pid (),
                          ItemList :: [Item],
                          Item :: process_info_item (),
                          InfoTupleList :: [InfoTuple],
                          InfoTuple :: process_info_result_item (). Returns information about the process identified by Pid , as specified by Item or ItemList . Returns undefined if the process is not alive. If the process is alive and a single Item is specified, the returned value is
the corresponding InfoTuple , unless Item =:= registered_name and the process
has no registered name. In this case, [] is returned. This strange behavior is
because of historical reasons, and is kept for backward compatibility. If ItemList is specified, the result is InfoTupleList . The InfoTuple s in InfoTupleList are included with the corresponding Item s in the same order as
the Item s were included in ItemList . Valid Item s can be included multiple
times in ItemList . Getting process information follows the signal ordering guarantees described in
the Processes Chapter in the Erlang
Reference Manual . Note If registered_name is part of ItemList and the process has no name
registered, a {registered_name, []} , InfoTuple will be included in the
resulting InfoTupleList . This behavior is different when a single Item =:= registered_name is specified, and when process_info/1 is used. Valid InfoTuple s with corresponding Item s: {async_dist, Enabled} - Current value of the async_dist process flag. Since: OTP 25.3 {backtrace, Bin} - Binary Bin contains the same information as the
output from erlang:process_display(Pid, backtrace) . Use binary_to_list/1 to obtain the string of characters
from the binary. {binary, BinInfo} - BinInfo is a list containing miscellaneous
information about binaries on the heap of this process. This InfoTuple can
be changed or removed without prior notice. In the current implementation BinInfo is a list of tuples. The tuples contain; BinaryId , BinarySize , BinaryRefcCount . Depending on the value of the message_queue_data process
flag the message queue may be stored on the 
...[truncated]

### is_process_alive/1
is_process_alive(Pid) (auto-imported) -spec is_process_alive(Pid) -> boolean () when Pid :: pid (). Pid must refer to a process at the local node. Returns true if the process exists and is alive, that is, is not exiting and
has not exited. Otherwise returns false . If process P1 calls is_process_alive(P2Pid) it is
guaranteed that all signals, sent from P1 to P2 ( P2 is the process with
identifier P2Pid ) before the call, will be delivered to P2 before the
aliveness of P2 is checked. This guarantee means that one can use is_process_alive/1 to let a process P1 wait until a
process P2 , which has got an exit signal with reason kill from P1, is
killed. For example: 1> P2Pid = spawn ( fun ( ) -> receive after infinity -> ok end end ) . 2> exit ( P2Pid , kill ) . true % P2 might not be killed 3> is_process_alive ( P2Pid ) . false % P2 is not alive (the call above always return false) See the documentation about signals and erlang:exit_signal/2 for more information about signals and
exit signals.
is_process_alive(Pid) (auto-imported)
is_process_alive(Pid) (auto-imported)

### t:process_info_item/0
process_info_item() -type process_info_item() ::
          async_dist | backtrace | binary | catchlevel | current_function | current_location |
          current_stacktrace | dictionary |
          {dictionary, Key :: term ()} |
          error_handler | garbage_collection | garbage_collection_info | group_leader | heap_size |
          initial_call | links | label | last_calls | memory | message_queue_len | messages |
          min_heap_size | min_bin_vheap_size | monitored_by | monitors | message_queue_data | parent |
          priority | priority_messages | reductions | registered_name | sequential_trace_token |
          stack_size | status | suspending | total_heap_size | trace | trap_exit.
process_info_item()
process_info_item()

## Registration

### register/2
register(RegName, PidOrPort) (auto-imported) -spec register(RegName, PidOrPort) -> true when RegName :: atom (), PidOrPort :: port () | pid (). Registers the name RegName with a process identifier (pid) or a port
identifier in the name registry . RegName , which must be an atom, can be used instead of the pid or port
identifier in send operator ( RegName ! Message ) and most other BIFs that take
a pid or port identifies as an argument. For example: 1> Pid = spawn ( fun ( ) -> receive after infinity -> ok end end ) . 2> register ( db , Pid ) . true The registered name is considered a Directly Visible Erlang Resource and is automatically unregistered when the process terminates. Failures: badarg - If PidOrPort is not an existing local process or port. badarg - If RegName is already in use. badarg - If the process or port is already registered (already has a
name). badarg - If RegName is the atom undefined .
register(RegName, PidOrPort) (auto-imported)
register(RegName, PidOrPort) (auto-imported)

### unregister/1
unregister(RegName) (auto-imported) -spec unregister(RegName) -> true when RegName :: atom (). Removes the registered name RegName associated with a
process identifier or a port identifier from the name registry . For example: > unregister ( db ) . true Keep in mind that you can still receive signals associated with the registered
name after it has been unregistered as the sender may have looked up the name
before sending to it. Users are advised not to unregister system processes. Failure: badarg if RegName is not a registered name.
unregister(RegName) (auto-imported)
unregister(RegName) (auto-imported)

### whereis/1
whereis(RegName) (auto-imported) -spec whereis(RegName) -> pid () | port () | undefined when RegName :: atom (). Returns the process identifier or port identifier with the registered name RegName from the name registry . Returns undefined if the name is not registered. For example: > whereis ( db ) . < 0.43 . 0 >
whereis(RegName) (auto-imported)
whereis(RegName) (auto-imported)

### registered/0
registered() (auto-imported) -spec registered() -> [RegName] when RegName :: atom (). Returns a list of names that have been registered using register/2 . For example: > registered ( ) . [ code_server , file_server , init , user , my_db ]
registered() (auto-imported)
registered() (auto-imported)

## Introspection (system_info/statistics/memory/process_info items)

### system_info/1
system_info(Item) -spec system_info(allocated_areas) -> [ tuple ()];
                 (allocator) -> {Allocator, Version, Features, Settings}
                     when
                         Allocator :: undefined | glibc,
                         Version :: [ non_neg_integer ()],
                         Features :: [ atom ()],
                         Settings :: [{Subsystem :: atom (), [{Parameter :: atom (), Value :: term ()}]}];
                 ({allocator, Alloc}) -> [_] when Alloc :: atom ();
                 (alloc_util_allocators) -> [Alloc] when Alloc :: atom ();
                 ({allocator_sizes, Alloc}) -> [_] when Alloc :: atom ();
                 (atom_count) -> pos_integer ();
                 (atom_limit) -> pos_integer ();
                 (build_type) -> opt | debug | gcov | valgrind | gprof | lcnt | frmptr;
                 (c_compiler_used) -> { atom (), term ()};
                 (check_io) -> [_];
                 (cpu_topology) -> CpuTopology when CpuTopology :: cpu_topology ();
                 ({cpu_topology, defined | detected | used}) -> CpuTopology
                     when CpuTopology :: cpu_topology ();
                 (cpu_quota) -> pos_integer () | unknown;
                 (creation) -> integer ();
                 (debug_compiled) -> boolean ();
                 (delayed_node_table_gc) -> infinity | non_neg_integer ();
                 (dirty_cpu_schedulers) -> non_neg_integer ();
                 (dirty_cpu_schedulers_online) -> non_neg_integer ();
                 (dirty_io_schedulers) -> non_neg_integer ();
                 (dist) -> binary ();
                 (dist_buf_busy_limit) -> non_neg_integer ();
                 (dist_ctrl) -> [{Node :: node (), ControllingEntity :: port () | pid ()}];
                 (driver_version) -> string ();
                 (dynamic_trace) -> none | dtrace | systemtap;
                 (dynamic_trace_probes) -> boolean ();
                 (eager_check_io) -> boolean ();
                 (embedded_3pps) -> #{included := [ atom ()], excluded := [ atom ()]};
                 (emu_flavor) -> emu | jit;
                 (emu_type) -> opt | debug | gcov | valgrind | gprof | lcnt | frmptr;
                 (end_time) -> non_neg_integer ();
                 (ets_count) -> pos_integer ();
                 (ets_limit) -> pos_integer ();
                 (fullsweep_after) -> {fullsweep_after, non_neg_integer ()};
                 (garbage_collection) -> garbage_collection_defaults ();
   
...[truncated]

### statistics/1
statistics(Item) (auto-imported) -spec statistics(active_tasks) -> [ActiveTasks] when ActiveTasks :: non_neg_integer ();
                (active_tasks_all) -> [ActiveTasks] when ActiveTasks :: non_neg_integer ();
                (context_switches) -> {ContextSwitches, 0} when ContextSwitches :: non_neg_integer ();
                (exact_reductions) -> {Total_Exact_Reductions, Exact_Reductions_Since_Last_Call}
                    when
                        Total_Exact_Reductions :: non_neg_integer (),
                        Exact_Reductions_Since_Last_Call :: non_neg_integer ();
                (garbage_collection) -> {Number_of_GCs, Words_Reclaimed, 0}
                    when Number_of_GCs :: non_neg_integer (), Words_Reclaimed :: non_neg_integer ();
                (io) -> {{input, Input}, {output, Output}}
                    when Input :: non_neg_integer (), Output :: non_neg_integer ();
                (microstate_accounting) -> [MSAcc_Thread] | undefined
                    when
                        MSAcc_Thread ::
                            #{type := MSAcc_Thread_Type,
                              id := MSAcc_Thread_Id,
                              counters := MSAcc_Counters},
                        MSAcc_Thread_Type ::
                            async | aux | dirty_io_scheduler | dirty_cpu_scheduler | poll | scheduler,
                        MSAcc_Thread_Id :: non_neg_integer (),
                        MSAcc_Counters :: #{MSAcc_Thread_State => non_neg_integer ()},
                        MSAcc_Thread_State ::
                            alloc | aux | bif | busy_wait | check_io | emulator | ets | gc |
                            gc_fullsweep | nif | other | port | send | sleep | timers;
                (reductions) -> {Total_Reductions, Reductions_Since_Last_Call}
                    when
                        Total_Reductions :: non_neg_integer (),
                        Reductions_Since_Last_Call :: non_neg_integer ();
                (run_queue) -> non_neg_integer ();
                (run_queue_lengths) -> [RunQueueLength] when RunQueueLength :: non_neg_integer ();
                (run_queue_lengths_all) -> [RunQueueLength] when RunQueueLength :: non_neg_integer ();
                (runtime) -> {Total_Run_Time, Time_Since_Last_Call}
                    when Total_Run_Time :: non_neg_integer (), Time_Since_Last_Call :: non_neg_integer ();
                (scheduler_wall_time) -> [{SchedulerId, ActiveTime, TotalTime}] | undefined
  
...[truncated]

### memory/0
memory() -spec memory() -> [{Type, Size}] when Type :: memory_type (), Size :: non_neg_integer (). Returns a list with information about memory dynamically allocated by the Erlang
emulator. Each list element is a tuple {Type, Size} . The first element Type is an atom describing memory type. The second element Size is the memory size
in bytes. Memory types: total - The total amount of memory currently allocated. This is the same
as the sum of the memory size for processes and system . processes - The total amount of memory currently allocated for the
Erlang processes. processes_used - The total amount of memory currently used by the Erlang
processes. This is part of the memory presented as processes memory. system - The total amount of memory currently allocated for the emulator
that is not directly related to any Erlang process. Memory presented as processes is not included in this memory. instrument can be used to get
a more detailed breakdown of what memory is part of this type. atom - The total amount of memory currently allocated for atoms. This
memory is part of the memory presented as system memory. atom_used - The total amount of memory currently used for atoms. This
memory is part of the memory presented as atom memory. binary - The total amount of memory currently allocated for binaries.
This memory is part of the memory presented as system memory. code - The total amount of memory currently allocated for Erlang code.
This memory is part of the memory presented as system memory. ets - The total amount of memory currently allocated for ETS tables.
This memory is part of the memory presented as system memory. maximum - The maximum total amount of memory allocated since the
emulator was started. This tuple is only present when the emulator is run with
instrumentation. For information on how to run the emulator with instrumentation, see instrument and/or erl(1) . Note The system value is not complete. Some allocated memory that is to be part
of this value is not. When the emulator is run with instrumentation, the system value is more
accurate, but memory directly allocated for malloc (and friends) is still
not part of the system value. Direct calls to malloc are only done from
OS-specific runtime libraries and perhaps from user-implemented Erlang drivers
that do not use the memory allocation functions in the driver interface. As the total value is the sum of processes and system , the error in system propagates to the total value. The different amount
...[truncated]

### memory/1
memory/1 -spec memory(Type :: memory_type ()) -> non_neg_integer ();
            (TypeList :: [ memory_type ()]) -> [{ memory_type (), non_neg_integer ()}]. Returns the memory size in bytes allocated for memory of type Type . The
argument can also be specified as a list of memory_type/0 atoms, in which case
a corresponding list of {memory_type(), Size :: integer >= 0} tuples is
returned. Change As from ERTS 5.6.4, erlang:memory/1 requires that all erts_alloc(3) allocators are enabled (default behavior). Failures: badarg - If Type is not one of the memory types listed in the
description of erlang:memory/0 . badarg - If maximum is passed as Type and the emulator is not run in
instrumented mode. notsup - If an erts_alloc(3) allocator has been
disabled. See also erlang:memory/0 .
memory/1
memory/1

### garbage_collect/0
garbage_collect() (auto-imported) -spec garbage_collect() -> true. Forces an immediate garbage collection of the executing process. The function is not to be used unless it has been noticed (or there are good
reasons to suspect) that the spontaneous garbage collection will occur too late
or not at all. Warning Improper use can seriously degrade system performance.
garbage_collect() (auto-imported)
garbage_collect() (auto-imported)

### garbage_collect/1
garbage_collect(Pid) (auto-imported) -spec garbage_collect(Pid) -> GCResult when Pid :: pid (), GCResult :: boolean (). Equivalent to garbage_collect(Pid, []) .
garbage_collect(Pid) (auto-imported)
garbage_collect(Pid) (auto-imported)

### garbage_collect/2
garbage_collect(Pid, OptionList) (auto-imported) (since OTP 17.0) -spec garbage_collect(Pid, OptionList) -> GCResult | async
                         when
                             Pid :: pid (),
                             RequestId :: term (),
                             Option :: {async, RequestId} | {type, major | minor},
                             OptionList :: [Option],
                             GCResult :: boolean (). Garbage collects the node local process identified by Pid . Option : {async, RequestId} - The function garbage_collect/2 returns the value async immediately after the request has been sent. When the request has been
processed, the process that called this function is passed a message on the
form {garbage_collect, RequestId, GCResult} . {type, 'major' | 'minor'} - Triggers garbage collection of requested
type. Default value is 'major' , which would trigger a fullsweep GC. The
option 'minor' is considered a hint and may lead to either minor or major GC
run. If Pid equals self/0 , and no async option has been passed, the garbage
collection is performed at once, that is, the same as calling garbage_collect/0 . Otherwise a request for garbage collection is sent to the
process identified by Pid , and will be handled when appropriate. If no async option has been passed, the caller blocks until GCResult is available and can
be returned. GCResult informs about the result of the garbage collection request as
follows: true - The process identified by Pid has been garbage collected. false - No garbage collection was performed, as the process identified
by Pid terminated before the request could be satisfied. Notice that the same caveats apply as for garbage_collect/0 . Failures: badarg - If Pid is not a node local process identifier. badarg - If OptionList is an invalid list of options.
garbage_collect(Pid, OptionList) (auto-imported) (since OTP 17.0)
garbage_collect(Pid, OptionList) (auto-imported) (since OTP 17.0)

### hibernate/3
hibernate(Module, Function, Args) -spec hibernate(Module, Function, Args) -> no_return ()
                   when Module :: module (), Function :: atom (), Args :: [ term ()]. Puts the calling process into a wait state where its memory allocation has been
reduced as much as possible. This is useful if the process does not expect to
receive any messages soon. The process is awakened when a message is sent to it, and control resumes in Module:Function with the arguments specified by Args with the call stack
emptied, meaning that the process terminates when that function returns. Thus erlang:hibernate/3 never returns to its caller. The resume function Module:Function/Arity must be exported ( Arity =:= length(Args) ). If the process has any message in its message queue, the process is awakened
immediately in the same way as described earlier. In more technical terms, erlang:hibernate/3 discards the call stack for the
process, and then garbage collects the process. After this, all live data is in
one continuous heap. The heap is then shrunken to the exact same size as the
live data that it holds (even if that size is less than the minimum heap size
for the process). If the size of the live data in the process is less than the minimum heap size,
the first garbage collection occurring after the process is awakened ensures
that the heap size is changed to a size not smaller than the minimum heap size. Notice that emptying the call stack means that any surrounding catch is
removed and must be re-inserted after hibernation. One effect of this is that
processes started using proc_lib (also indirectly, such as gen_server processes), are to use proc_lib:hibernate/3 instead, to ensure that the
exception handler continues to work when the process wakes up.
hibernate(Module, Function, Args)
hibernate(Module, Function, Args)

### hibernate/0
hibernate() (since OTP 28.0) -spec hibernate() -> ok. Puts the calling process into a wait state where its memory allocation has been
reduced as much as possible. This is useful if the process does not expect to
receive any messages soon. The process is awakened when a message is sent to it, and control resumes
normally to the caller. Unlike erlang:hibernate/3 , it does not discard the
call stack.
hibernate() (since OTP 28.0)
hibernate() (since OTP 28.0)

### processes/0
processes() (auto-imported) -spec processes() -> [ pid ()]. Returns a list of process identifiers corresponding to all the processes
currently existing on the local node. Notice that an exiting process exists, but is not alive. That is, is_process_alive/1 returns false for an exiting
process, but its process identifier is part of the result returned from processes/0 . Example: > processes ( ) . [ < 0.0 . 0 > , < 0.2 . 0 > , < 0.4 . 0 > , < 0.5 . 0 > , < 0.7 . 0 > , < 0.8 . 0 > ]
processes() (auto-imported)
processes() (auto-imported)

### ports/0
ports() -spec ports() -> [ port ()]. Returns a list of port identifiers corresponding to all the ports existing on
the local node. Notice that an exiting port exists, but is not open.
ports()
ports()

## Time BIFs

### monotonic_time/0
monotonic_time() (since OTP 18.0) -spec monotonic_time() -> integer (). Returns the current Erlang monotonic time in native time unit . This is a monotonically increasing time
since some unspecified point in time. Note This is a monotonically increasing time,
but not a strictly monotonically increasing time. That is, consecutive calls to erlang:monotonic_time/0 can produce the
same result. Different runtime system instances will use different unspecified points in
time as base for their Erlang monotonic clocks. That is, it is pointless comparing monotonic times from different runtime system instances. Different
runtime system instances can also place this unspecified point in time
different relative runtime system start. It can be placed in the future (time
at start is a negative value), the past (time at start is a positive value),
or the runtime system start (time at start is zero). The monotonic time at
runtime system start can be retrieved by calling erlang:system_info(start_time) .
monotonic_time() (since OTP 18.0)
monotonic_time() (since OTP 18.0)

### monotonic_time/1
monotonic_time(Unit) (since OTP 18.0) -spec monotonic_time(Unit) -> integer () when Unit :: time_unit (). Returns the current Erlang monotonic time converted into
the Unit passed as argument. Same as calling erlang:convert_time_unit ( erlang:monotonic_time() , native, Unit) ,
however optimized for commonly used Unit s.
monotonic_time(Unit) (since OTP 18.0)
monotonic_time(Unit) (since OTP 18.0)

### system_time/0
system_time() (since OTP 18.0) -spec system_time() -> integer (). Returns current Erlang system time in native time unit . Calling erlang:system_time() is equivalent to erlang:monotonic_time() + erlang:time_offset() . Note This time is not a monotonically increasing time in the general case. For
more information, see the documentation of time warp modes in the User's Guide.
system_time() (since OTP 18.0)
system_time() (since OTP 18.0)

### system_time/1
system_time(Unit) (since OTP 18.0) -spec system_time(Unit) -> integer () when Unit :: time_unit (). Returns current Erlang system time converted into the Unit passed as argument. Calling erlang:system_time(Unit) is equivalent to erlang:convert_time_unit ( erlang:system_time() , native, Unit) . Note This time is not a monotonically increasing time in the general case. For
more information, see the documentation of time warp modes in the User's Guide.
system_time(Unit) (since OTP 18.0)
system_time(Unit) (since OTP 18.0)

### timestamp/0
timestamp() (since OTP 18.0) -spec timestamp() -> Timestamp when Timestamp :: timestamp (). Returns current Erlang system time on
the format {MegaSecs, Secs, MicroSecs} . This format is the same as os:timestamp/0 and the deprecated erlang:now/0 use.
The reason for the existence of erlang:timestamp() is purely to simplify use for existing
code that assumes this time stamp format. Current Erlang system time can more
efficiently be retrieved in the time unit of your choice using erlang:system_time/1 . The erlang:timestamp() BIF is equivalent to: timestamp ( ) -> ErlangSystemTime = erlang : system_time ( microsecond ) , MegaSecs = ErlangSystemTime div 1000_000_000_000 , Secs = ErlangSystemTime div 1000_000 - MegaSecs * 1000_000 , MicroSecs = ErlangSystemTime rem 1000_000 , { MegaSecs , Secs , MicroSecs } . It, however, uses a native implementation that does not build garbage on the
heap and with slightly better performance. Note This time is not a monotonically increasing time in the general case. For
more information, see the documentation of time warp modes in the User's Guide.
timestamp() (since OTP 18.0)
timestamp() (since OTP 18.0)

### start_timer/3
start_timer(Time, Dest, Msg) -spec start_timer(Time, Dest, Msg) -> TimerRef
                     when
                         Time :: non_neg_integer (),
                         Dest :: pid () | atom (),
                         Msg :: term (),
                         TimerRef :: reference (). Equivalent to erlang:start_timer(Time, Dest, Msg, []) .
start_timer(Time, Dest, Msg)
start_timer(Time, Dest, Msg)

### start_timer/4
start_timer(Time, Dest, Msg, Options) (since OTP 18.0) -spec start_timer(Time, Dest, Msg, Options) -> TimerRef
                     when
                         Time :: integer (),
                         Dest :: pid () | atom (),
                         Msg :: term (),
                         Options :: [Option],
                         Abs :: boolean (),
                         Option :: {abs, Abs},
                         TimerRef :: reference (). Starts a timer. When the timer expires, the message {timeout, TimerRef, Msg} is sent to the process identified by Dest . Option s: {abs, false} - This is the default. It means the Time value is
interpreted as a time in milliseconds relative current Erlang monotonic time . {abs, true} - Absolute Time value. The Time value is interpreted as
an absolute Erlang monotonic time in milliseconds. More Option s can be added in the future. The absolute point in time, the timer is set to expire on, must be in the
interval [ erlang:convert_time_unit ( erlang:system_info (start_time), native, millisecond), erlang:convert_time_unit ( erlang:system_info (end_time), native, millisecond) ] .
If a relative time is specified, the Time value is not allowed to be negative. If Dest is a pid/0 , it must be a pid/0 of a process created on the
current runtime system instance. This process has either terminated or not. If Dest is an atom/0 , it is interpreted as the name of a locally registered
process. The process referred to by the name is looked up at the time of timer
expiration. No error is returned if the name does not refer to a process. If Dest is a pid/0 , the timer is automatically canceled if the process
referred to by the pid/0 is not alive, or if the process exits. This feature
was introduced in ERTS 5.4.11. Notice that timers are not automatically canceled
when Dest is an atom/0 . See also erlang:send_after/4 , erlang:cancel_timer/2 , and erlang:read_timer/2 . For more information on timers in Erlang in general, see the Timers section of the Time and Time Correction in Erlang ERTS User's guide. Failure: badarg if the arguments do not satisfy the requirements specified
here.
start_timer(Time, Dest, Msg, Options) (since OTP 18.0)
start_timer(Time, Dest, Msg, Options) (since OTP 18.0)

### cancel_timer/1
cancel_timer(TimerRef) -spec cancel_timer(TimerRef) -> Result
                      when TimerRef :: reference (), Time :: non_neg_integer (), Result :: Time | false. Equivalent to erlang:cancel_timer(TimerRef, []) .
cancel_timer(TimerRef)
cancel_timer(TimerRef)

### cancel_timer/2
cancel_timer(TimerRef, Options) (since OTP 18.0) -spec cancel_timer(TimerRef, Options) -> Result | ok
                      when
                          TimerRef :: reference (),
                          Async :: boolean (),
                          Info :: boolean (),
                          Option :: {async, Async} | {info, Info},
                          Options :: [Option],
                          Time :: non_neg_integer (),
                          Result :: Time | false. Cancels a timer that has been created by erlang:start_timer or erlang:send_after . TimerRef identifies the timer, and
was returned by the BIF that created the timer. Option s: {async, Async} - Asynchronous request for cancellation. Async defaults
to false , which causes the cancellation to be performed synchronously. When Async is set to true , the cancel operation is performed asynchronously.
That is, cancel_timer() sends an asynchronous request for cancellation to
the timer service that manages the timer, and then returns ok . {info, Info} - Requests information about the Result of the
cancellation. Info defaults to true , which means the Result is given.
When Info is set to false , no information about the result of the
cancellation is given. When Async is false : if Info is true , the Result is returned by erlang:cancel_timer() . otherwise ok is returned. When Async is true : if Info is true , a message on the form {cancel_timer, TimerRef, Result} is sent to the caller of erlang:cancel_timer() when the cancellation operation has been performed,
otherwise no message is sent. More Option s may be added in the future. If Result is an integer, it represents the time in milliseconds left until the
canceled timer would have expired. If Result is false , a timer corresponding to TimerRef could not be found.
This can be either because the timer had expired, already had been canceled, or
because TimerRef never corresponded to a timer. Even if the timer had expired,
it does not tell you if the time-out message has arrived at its destination yet. Note The timer service that manages the timer can be co-located with another
scheduler than the scheduler that the calling process is executing on. If so,
communication with the timer service takes much longer time than if it is
located locally. If the calling process is in critical path, and can do other
things while waiting for the result of this operation, or is not interested in
the result of the operation, you want to use option {asy
...[truncated]

### read_timer/1
read_timer(TimerRef) -spec read_timer(TimerRef) -> Result
                    when TimerRef :: reference (), Time :: non_neg_integer (), Result :: Time | false. Equivalent to erlang:read_timer(TimerRef, []) .
read_timer(TimerRef)
read_timer(TimerRef)

### read_timer/2
read_timer(TimerRef, Options) (since OTP 18.0) -spec read_timer(TimerRef, Options) -> Result | ok
                    when
                        TimerRef :: reference (),
                        Async :: boolean (),
                        Option :: {async, Async},
                        Options :: [Option],
                        Time :: non_neg_integer (),
                        Result :: Time | false. Reads the state of a timer that has been created by either erlang:start_timer or erlang:send_after . TimerRef identifies the timer, and was
returned by the BIF that created the timer. Options : {async, Async} - Asynchronous request for state information. Async defaults to false , which causes the operation to be performed synchronously.
In this case, the Result is returned by erlang:read_timer . When Async is true , erlang:read_timer sends an asynchronous request for the state
information to the timer service that manages the timer, and then returns ok . A message on the format {read_timer, TimerRef, Result} is sent to the
caller of erlang:read_timer when the operation has been processed. More Option s can be added in the future. If Result is an integer, it represents the time in milliseconds left until the
timer expires. If Result is false , a timer corresponding to TimerRef could not be found.
This because the timer had expired, or been canceled, or because TimerRef never has corresponded to a timer. Even if the timer has expired, it does not
tell you whether or not the time-out message has arrived at its destination yet. Note The timer service that manages the timer can be co-located with another
scheduler than the scheduler that the calling process is executing on. If so,
communication with the timer service takes much longer time than if it is
located locally. If the calling process is in a critical path, and can do
other things while waiting for the result of this operation, you want to use
option {async, true} . If using option {async, false} , the calling process
is blocked until the operation has been performed. See also erlang:send_after/4 , erlang:start_timer/4 , and erlang:cancel_timer/2 .
read_timer(TimerRef, Options) (since OTP 18.0)
read_timer(TimerRef, Options) (since OTP 18.0)

## Strict rules (trap_exit; exit/1 vs exit/2 vs exit_signal/2; spawn_opt monitor disallowed; signal ordering)
- **trap_exit**: when `process_flag(trap_exit, true)`, exit signals are converted to `{'EXIT', From, Reason}` messages instead of killing the receiving process. `normal` reason never kills a linked process; any other reason propagates to linked processes unless trapped.
- **exit/1**: `exit(Reason)` terminates the *current* process with `Reason`; if `Reason` is `kill` the process is unconditionally terminated (untrappable). Does NOT set the exit reason of others.
- **exit/2**: `exit(Pid, Reason)` sends an exit signal with `Reason` to `Pid`. `kill` is untrappable; `normal` is ignored by a live process.
- **exit_signal/2,3** (OTP 24+ split): `exit_signal/2` sends an exit signal to a process identified by Pid or alias WITHOUT going through the message queue of the caller's node distribution (lower-level than exit/2). `exit_signal/3` adds options. Introduced to separate 'send exit signal' from 'raise exit'. exit/2 remains the high-level API; exit_signal is the primitive used by the runtime.
- **raise/3**: `raise(Class, Reason, Stacktrace)` aborts the current process with a given class (error|exit|throw) and stacktrace, as if that exception occurred. Does not return.
- **spawn_opt monitor disallowed**: passing both `link` and `monitor` to spawn_opt raises `badarg`.
- **Signal ordering**: exit signals from the same sender to the same receiver are delivered in sending order; signals from different senders have no guaranteed interleaving. Monitor messages (`'DOWN'`) are delivered after the corresponding exit signal when a monitored process dies.

## Verbatim quotes (the key 4-8)
- **exit/2**: exit(Dest, Reason) (auto-imported) -spec exit(Dest, Reason) -> true when Dest :: pid () | port () | reference (), Reason :: term (). Old form of exit_signal/2 , with a quirk when sender and receiver are the same.
- **exit_signal/2**: exit_signal(Pid, Reason) (auto-imported) (since OTP 29.0) -spec exit_signal(Pid, Reason) -> true when Pid :: pid () | port () | reference (), Reason :: term (). Sends an exit signal with exit reason Reason to the process or port identified
by Dest .
- **spawn_opt/5**: spawn_opt(Node, Module, Function, Args, Options) (auto-imported) -spec spawn_opt(Node, Module, Function, Args, Options) -> pid () | { pid (), reference ()}
                   when
                       Node :: node (),
                       Module :: module (),
                       Function :: atom (),
                       Args :: [ term ()],
                       Options :: [monitor | {monitor, [ monitor_option ()]} | link | OtherOption],
                       OtherOption :: term (). Returns the process identifier (pid) of a new process started by the application
of Module:Function to Args on Node .
- **monitor/2**: monitor(Type, Item) (auto-imported) -spec monitor(process, monitor_process_identifier ()) -> MonitorRef when MonitorRef :: reference ();
             (port, monitor_port_identifier ()) -> MonitorRef when MonitorRef :: reference ();
             (time_offset, clock_service) -> MonitorRef when MonitorRef :: reference (). Sends a monitor request of type Type to the entity identified by Item .
- **process_flag/2**: process_flag(Flag, Value) (auto-imported) -spec process_flag(async_dist, Boolean) -> OldBoolean when Boolean :: boolean (), OldBoolean :: boolean ();
                  (trap_exit, Boolean) -> OldBoolean when Boolean :: boolean (), OldBoolean :: boolean ();
                  (error_handler, Module) -> OldModule when Module :: atom (), OldModule :: atom ();
                  (fullsweep_after, FullsweepAfter) -> OldFullsweepAfter
                      when FullsweepAfter :: non_neg_integer (), OldFullsweepAfter :: non_neg_integer ();
                  (min_heap_size, MinHeapSize) -> OldMinHeapSize
                      when MinHeapSize :: non_neg_integer (), OldMinHeapSize :: non_neg_integer ();
                  (min_bin_vheap_size, MinBinVHeapSize) -> OldMinBinVHeapSize
                      when MinBinVHeapSize :: non_neg_integer (), OldMinBinVHeapSize :: non_neg_integer ();
                  (max_heap_size, MaxHeapSize) -> OldMaxHeapSize
                      when MaxHeapSize :: max_heap_size (), OldMaxHeapSize :: max_heap_size ();
                  (message_queue_data, MQD) -> OldMQD
                      when MQD :: message_queue_data (), OldMQD :: message_queue_data ();
                  (priority, Level) -> OldLevel
                      when Level :: priority_level (), OldLevel :: priority_level ();
                  (save_calls, N) -> OldN when N :: 0..10000, OldN :: 0..10000;
                  (sensitive, Boolean) -> OldBoolean when Boolean :: boolean (), OldBoolean :: boolean ();
                  ({monitor_nodes, term ()}, term ()) -> term ();
                  (monitor_nodes, term ()) -> term (). Sets the process flag indicated to the specified value.
- **hibernate/3**: hibernate(Module, Function, Args) -spec hibernate(Module, Function, Args) -> no_return ()
                   when Module :: module (), Function :: atom (), Args :: [ term ()]. Puts the calling process into a wait state where its memory allocation has been
reduced as much as possible.
- **raise/3**: raise(Class, Reason, Stacktrace) -spec raise(Class, Reason, Stacktrace) -> badarg
               when Class :: error | exit | throw, Reason :: term (), Stacktrace :: raise_stacktrace (). Raises an exception of the specified class, reason, and call stack backtrace
( stacktrace ).
- **alias/0**: alias() (auto-imported) (since OTP 24.0) -spec alias() -> Alias when Alias :: reference (). Equivalent to alias([]) .

## Version notes (OTP-version callouts for new BIFs)
- `exit_signal/2,3`: introduced in OTP 24 (split of exit/2 into send-signal vs raise).
- `alias/0,1`, `unalias/1`: introduced in OTP 24 (process aliases for race-free monitoring).
- `spawn_opt/5`: the 5-arg form with options map/list; `spawn_opt/2,3,4` are older arities.
- `raise/3`: introduced in OTP 24.
- `process_flag(max_heap_size, _)`: OTP 19+.
- `process_flag(message_queue_data, _)`: OTP 19+.
- `process_flag(async_dist, _)`: OTP 23+.
- `monotonic_time/0,1`, `system_time/0,1`, `timestamp/0` (extended): OTP 18 time API overhaul.
- `get_stacktrace/0`: DEPRECATED since OTP 21, removed in OTP 24. Use `erlang:process_info(self(), current_stacktrace)` or the `t:stacktrace/0` type from a caught exception via `try ... catch Class:Reason:Stacktrace`.
- `spawn_request/1..5`, `spawn_request_abandon/1`, `processes_iterator/0`, `processes_next/1`: OTP 25+ (async spawn + safe process iteration).

## Discovered links
### Relevant (crawl later)
- https://www.erlang.org/doc/apps/erts/time_correction.html
- https://www.erlang.org/doc/apps/erts/time_correction.html#timers
- https://www.erlang.org/doc/apps/kernel/net_kernel.html
- https://www.erlang.org/doc/apps/kernel/net_kernel.html#monitor_nodes/1
- https://www.erlang.org/doc/apps/kernel/trace.html
- https://www.erlang.org/doc/apps/kernel/seq_trace.html
- https://www.erlang.org/doc/apps/kernel/logger.html
- https://www.erlang.org/doc/apps/kernel/os.html#perf_counter/0
- https://www.erlang.org/doc/apps/kernel/os.html#timestamp/0
- https://www.erlang.org/doc/apps/kernel/application.html#start/2
- https://www.erlang.org/doc/apps/kernel/error_handler.html
- https://www.erlang.org/doc/apps/runtime_tools/msacc.html
- https://www.erlang.org/doc/apps/runtime_tools/scheduler.html
- https://www.erlang.org/doc/apps/runtime_tools/instrument.html
- https://www.erlang.org/doc/apps/stdlib/calendar.html#t:datetime/0
### Skipped
- All intra-page `#anchor` links (same document).
- Math/binary/code-loading/port-program BIF sections (out of focus).
- ExDoc asset/JS/CSS links.