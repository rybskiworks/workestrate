#!/usr/bin/env python3
"""Run disposable, provider-free workloads through an explicitly selected CLI."""

import argparse
import ctypes
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time


EXAMPLES = Path(__file__).resolve().parent


class ChildSupervisor:
    """Own and reap this single-threaded runner's children, including double forks."""

    PR_SET_CHILD_SUBREAPER = 36
    PR_GET_CHILD_SUBREAPER = 37

    def __init__(self):
        if sys.platform != "linux" or not all((
            hasattr(os, "pidfd_open"), hasattr(os, "P_PIDFD"),
            hasattr(signal, "pidfd_send_signal"),
        )):
            raise RuntimeError("smoke supervision requires Linux pidfds and child subreaping")
        if self.children():
            raise RuntimeError("smoke supervision requires a runner without pre-existing children")
        # Check kernel support before launching anything that would need cleanup.
        fd = os.pidfd_open(os.getpid())
        try:
            signal.pidfd_send_signal(fd, 0)
            try:
                os.waitid(os.P_PIDFD, fd, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            except ChildProcessError:
                pass  # We are not our own child; the pidfd operation is supported.
        finally:
            os.close(fd)
        self.libc = ctypes.CDLL(None, use_errno=True)
        previous = ctypes.c_int()
        if self.libc.prctl(self.PR_GET_CHILD_SUBREAPER, ctypes.byref(previous), 0, 0, 0) != 0:
            raise OSError(ctypes.get_errno(), "cannot read child-subreaper setting")
        if self.libc.prctl(self.PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) != 0:
            raise OSError(ctypes.get_errno(), "cannot enable child subreaping")
        self.previous = previous.value
        self.processes = []
        self.pidfds = {}
        self.closed = False

    @staticmethod
    def children():
        # Read only our own child lists, never scan process names or environments.
        children = set()
        for task in Path("/proc/self/task").iterdir():
            try:
                children.update(map(int, (task / "children").read_text().split()))
            except FileNotFoundError:
                pass
        return children

    def run(self, command, *, timeout, **kwargs):
        if self.closed:
            raise RuntimeError("cannot launch through closed smoke supervision")
        deadline = time.monotonic() + timeout
        process = subprocess.Popen(command, **kwargs)
        self.processes.append(process)
        # Unlike subprocess.run, a timeout leaves the whole owned launch for
        # coordinated quiescence, not just a killed immediate launcher.
        return process.wait(timeout=max(0, deadline - time.monotonic()))

    def refresh(self):
        self.processes = [process for process in self.processes if process.poll() is None]
        for pid in self.children():
            if pid in self.pidfds:
                continue
            try:
                fd = os.pidfd_open(pid)
            except ProcessLookupError:
                continue
            try:
                # A pidfd pins identity; waitid also proves it is OUR child.
                # If a listed PID exited and was reused, ECHILD refuses it.
                os.waitid(os.P_PIDFD, fd, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            except ChildProcessError:
                os.close(fd)
                continue
            except BaseException:
                os.close(fd)
                raise
            self.pidfds[pid] = fd
        for pid, fd in list(self.pidfds.items()):
            try:
                status = os.waitid(os.P_PIDFD, fd, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                if status is None:
                    continue
                os.waitid(os.P_PIDFD, fd, os.WEXITED | os.WNOHANG)
            except ChildProcessError:
                pass  # Popen.poll already reaped this direct launcher.
            os.close(fd)
            del self.pidfds[pid]

    def quiesce(self, timeout=5, grace=1):
        """Stop exact owned producers before checking the final backend state."""
        deadline = time.monotonic() + timeout
        gentle_until = time.monotonic() + grace
        signalled = set()
        while True:
            self.refresh()
            if not self.pidfds and not self.children():
                return
            now = time.monotonic()
            if now >= deadline:
                raise RuntimeError(f"owned children did not exit before cleanup deadline: {sorted(self.pidfds)}")
            sig = signal.SIGTERM if now < gentle_until else signal.SIGKILL
            for fd in self.pidfds.values():
                key = (fd, sig)
                if sig == signal.SIGKILL or key not in signalled:
                    try:
                        signal.pidfd_send_signal(fd, sig)
                    except ProcessLookupError:
                        pass
                    signalled.add(key)
            # Parent exit reparents even new-process-group descendants to us.
            # Repeatedly adopt/reap until there can be no late creator left.
            time.sleep(min(0.05, max(0, deadline - time.monotonic())))

    def close(self):
        if not self.closed:
            self.quiesce()
            if self.libc.prctl(self.PR_SET_CHILD_SUBREAPER, self.previous, 0, 0, 0) != 0:
                raise OSError(ctypes.get_errno(), "cannot restore child-subreaper setting")
            self.closed = True


def isolated_environment(root, config_dir, nix):
    """Do not inherit homes, backend selection, credentials or project layers."""
    return {
        "HOME": str(root / "user"),
        "PATH": os.pathsep.join([str(nix.parent), "/usr/bin", "/bin"]),
        "MSB_HOME": str(root / "msb"),
        "MSB_CONFIG_PATH": str(root / "msb.json"),
        "MSB_BACKEND": "local",
        "WORKESTRATE_STATE_DIR": str(root / "state"),
        "WORKESTRATE_CONFIG_DIR": str(config_dir),
        "WORKESTRATE_REFERENCE_CONFIG": "0",
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_CONFIG_GLOBAL": "/dev/null",
    }


class Smoke:
    def __init__(self, workestrate, nix, case):
        # Short paths are necessary for the runtime's Unix-domain sockets.
        self.root = Path(tempfile.mkdtemp(prefix="wk-test.", dir="/tmp"))
        for directory in ("user", "tool", "state", "msb", "cwd", "logs"):
            (self.root / directory).mkdir()
        (self.root / "msb.json").write_text("{}\n")
        self.env = isolated_environment(self.root, EXAMPLES / case, nix)
        self.command = [str(workestrate), "--home", str(self.root / "tool"), "--no-project-config"]
        self.case = case
        self.sequence = 0
        self.attempted = False
        self.failed = False
        self.supervisor = None
        print(f"Smoke artifacts: {self.root}", flush=True)

    def run(self, arguments, timeout=30, check=True):
        self.sequence += 1
        log = self.root / "logs" / f"{self.sequence:03d}-{arguments[0]}.log"
        print("workestrate " + " ".join(arguments), flush=True)
        try:
            if self.supervisor is None:
                self.supervisor = ChildSupervisor()
            with log.open("w") as output:
                status = self.supervisor.run(
                    self.command + arguments,
                    cwd=self.root / "cwd",
                    env=self.env,
                    # Noninteractive msb exec reads stdin to EOF before sending
                    # its request; never inherit an indefinitely open input.
                    stdin=subprocess.DEVNULL,
                    stdout=output,
                    stderr=subprocess.STDOUT,
                    timeout=timeout,
                )
        except BaseException:
            self.failed = True
            raise
        text = log.read_text()
        if check and status:
            self.failed = True
            raise RuntimeError(f"command failed ({status}); see {log}\n{text[-4000:]}")
        return status, text

    def ps(self, timeout=30):
        return json.loads(self.run(["ps", "--json"], timeout=timeout)[1])

    def backend(self):
        return json.loads(self.run(["msb", "--", "list", "--format", "json"])[1])

    def wait_for_markers(self, timeout=45):
        expected = {"smoke@baseline"} if self.case == "baseline" else {"client@one", "server"}
        marker_slot = "smoke@baseline" if self.case == "baseline" else "client@one"
        marker = "SMOKE_OK" if self.case == "baseline" else "DEPENDENCY_OK"
        child_log = self.root / "state" / "logs" / marker_slot / "workestrate.log"
        deadline = time.monotonic() + timeout
        while (remaining := deadline - time.monotonic()) > 0:
            rows = self.ps(timeout=remaining)
            healthy = {row["instance"] for row in rows if row.get("status") == "running_healthy"}
            if time.monotonic() < deadline and expected <= healthy and child_log.exists() and marker in child_log.read_text():
                return rows
            time.sleep(min(0.5, max(0, deadline - time.monotonic())))
        raise RuntimeError(f"healthy instances/marker {marker} not observed; inspect {self.root}/state/logs")

    def execute(self):
        self.run(["validate-config"])
        target = "smoke" if self.case == "baseline" else "client"
        instance = "baseline" if self.case == "baseline" else "one"
        self.run(["workload", "plan", target, "--instance", instance])
        # Exercise Workestrate's Nix eval/build/import pipeline, not a separate
        # registry pull. Both examples carry complete independent flake locks.
        self.run(["workload", "build"], timeout=600)
        self.attempted = True
        self.run(["workload", "up", target, "--instance", instance])
        self.wait_for_markers()
        if self.case == "baseline":
            backends = self.backend()
            if len(backends) != 1:
                raise RuntimeError(f"expected exactly one isolated backend, got {backends}")
            _, output = self.run([
                "msb", "--", "exec", backends[0]["name"], "--no-tty", "--timeout", "15s",
                "--", "/bin/sh", "-c", "echo EXEC_OK; test \"$PWD\" = /tmp",
            ])
            if "EXEC_OK" not in output:
                raise RuntimeError("guest exec did not emit EXEC_OK")
        else:
            server_log = self.root / "state/logs/server/workestrate.log"
            if "GET / HTTP/1.1\" 200" not in server_log.read_text():
                raise RuntimeError("server did not record a successful client request")
            self.run(["workload", "down", "client", "--instance", "one"])
            if not any(row["instance"] == "server" and row["status"] == "running_healthy" for row in self.ps()):
                raise RuntimeError("stopping the client stopped its shared server")
            self.run(["workload", "down", "server"])
            self.attempted = False

    def cleanup(self):
        failures = []
        targets = [["smoke", "--instance", "baseline"]] if self.case == "baseline" else [
            ["client", "--instance", "one"], ["server"],
        ]

        def quiesce():
            if self.supervisor is not None:
                try:
                    self.supervisor.quiesce()
                except (OSError, RuntimeError) as error:
                    failures.append(str(error))

        def down():
            failed = False
            for target in targets:
                try:
                    status, _ = self.run(["workload", "down", *target], check=False)
                    if status:
                        failures.append(f"exact teardown failed for {target}")
                        failed = True
                except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
                    failures.append(str(error))
                    failed = True
            return failed

        # On failure, prevent a delayed detached child creating state AFTER
        # teardown. Healthy runs must exercise normal down before fallback signals.
        if self.failed:
            quiesce()
        retry = down() if self.attempted else False
        quiesce()
        if retry:
            down()  # Remove exact stopped records after fallback termination.
            quiesce()
        try:
            if self.ps() or self.backend():
                failures.append("isolated instance/backend records remain")
        except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
            failures.append(str(error))
        finally:
            if self.supervisor is not None:
                try:
                    self.supervisor.close()
                except (OSError, RuntimeError) as error:
                    failures.append(str(error))
        if failures:
            raise RuntimeError(f"cleanup requires attention at {self.root}: " + "; ".join(failures))


def executable(value):
    path = Path(shutil.which(value) or value).absolute()
    if not path.is_file() or not os.access(path, os.X_OK):
        raise argparse.ArgumentTypeError(f"not an executable: {value}")
    return path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("case", choices=["baseline", "comms"])
    parser.add_argument("--workestrate", required=True, type=executable)
    parser.add_argument("--nix", required=True, type=executable)
    parser.add_argument("--run", action="store_true", help="explicitly authorize disposable host KVM workloads")
    args = parser.parse_args()
    if not args.run:
        parser.error("--run is required; this command starts real microVMs")
    if not os.access("/dev/kvm", os.R_OK | os.W_OK):
        parser.error("host /dev/kvm must be readable and writable")
    smoke = Smoke(args.workestrate, args.nix, args.case)
    try:
        smoke.execute()
    except BaseException:
        smoke.failed = True
        raise
    finally:
        smoke.cleanup()
    print(f"PASS: {args.case}; logs/images/state retained at {smoke.root}", flush=True)


if __name__ == "__main__":
    main()
