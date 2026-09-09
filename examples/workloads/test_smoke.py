#!/usr/bin/env python3
"""Test smoke-runner isolation and teardown without launching any workloads."""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

import smoke


class SmokeTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="smoke-fixture-")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name) / "case"
        self.root.mkdir()

    def runner(self, case):
        with patch("smoke.tempfile.mkdtemp", return_value=str(self.root)):
            return smoke.Smoke(Path("/pinned/workestrate"), Path("/pinned/nix"), case)

    def test_environment_does_not_inherit_runtime_or_secret_selectors(self):
        with patch.dict("os.environ", {"MSB_BACKEND": "cloud", "SSH_AUTH_SOCK": "/private", "OPENAI_API_KEY": "fixture"}):
            env = smoke.isolated_environment(self.root, Path("/config"), Path("/nix/bin/nix"))
        self.assertEqual(env["MSB_BACKEND"], "local")
        self.assertEqual(env["MSB_HOME"], str(self.root / "msb"))
        self.assertEqual(env["MSB_CONFIG_PATH"], str(self.root / "msb.json"))
        self.assertEqual(env["HOME"], str(self.root / "user"))
        self.assertNotIn("SSH_AUTH_SOCK", env)
        self.assertNotIn("OPENAI_API_KEY", env)

    def test_constructor_pins_empty_microsandbox_config(self):
        runner = self.runner("baseline")
        self.assertEqual((self.root / "msb.json").read_text(), "{}\n")
        self.assertEqual(runner.command, ["/pinned/workestrate", "--home", str(self.root / "tool"), "--no-project-config"])

    def test_baseline_cleanup_targets_only_its_named_instance(self):
        runner = self.runner("baseline")
        runner.attempted = True
        with patch.object(runner, "run", return_value=(0, "")) as run, patch.object(runner, "ps", return_value=[]), patch.object(runner, "backend", return_value=[]):
            runner.cleanup()
        run.assert_called_once_with(["workload", "down", "smoke", "--instance", "baseline"], check=False)

    def test_cleanup_attempts_server_even_if_client_stop_fails(self):
        runner = self.runner("comms")
        runner.attempted = True
        with patch.object(runner, "run", side_effect=[(1, "failed"), (0, ""), (0, ""), (0, "")]) as run, patch.object(runner, "ps", return_value=[]), patch.object(runner, "backend", return_value=[]):
            with self.assertRaisesRegex(RuntimeError, "teardown failed"):
                runner.cleanup()
        self.assertEqual(run.call_args_list[1].args[0], ["workload", "down", "server"])
        self.assertEqual(run.call_count, 4)

    def test_stopped_backend_record_is_a_cleanup_failure(self):
        runner = self.runner("baseline")
        with patch.object(runner, "ps", return_value=[]), patch.object(runner, "backend", return_value=[{"name": "stopped", "status": "Stopped"}]):
            with self.assertRaisesRegex(RuntimeError, "records remain"):
                runner.cleanup()

    def test_healthy_cleanup_runs_normal_down_before_fallback_signals(self):
        runner = self.runner("baseline")
        runner.attempted = True
        events = []
        runner.supervisor = Mock()
        runner.supervisor.quiesce.side_effect = lambda: events.append("quiesce")
        runner.supervisor.close.side_effect = lambda: events.append("close")
        with patch.object(runner, "run", side_effect=lambda *args, **kwargs: (events.append("down") or 0, "")), patch.object(runner, "ps", return_value=[]), patch.object(runner, "backend", return_value=[]):
            runner.cleanup()
        self.assertEqual(events, ["down", "quiesce", "close"])

    def test_failed_cleanup_quiesces_producers_before_exact_down(self):
        runner = self.runner("baseline")
        runner.attempted = True
        runner.failed = True
        events = []
        runner.supervisor = Mock()
        runner.supervisor.quiesce.side_effect = lambda: events.append("quiesce")
        runner.supervisor.close.side_effect = lambda: events.append("close")
        with patch.object(runner, "run", side_effect=lambda *args, **kwargs: (events.append("down") or 0, "")), patch.object(runner, "ps", return_value=[]), patch.object(runner, "backend", return_value=[]):
            runner.cleanup()
        self.assertEqual(events, ["quiesce", "down", "quiesce", "close"])

    def test_readiness_poll_uses_remaining_deadline_and_rejects_late_success(self):
        runner = self.runner("baseline")
        log = runner.root / "state/logs/smoke@baseline/workestrate.log"
        log.parent.mkdir(parents=True)
        log.write_text("SMOKE_OK\n")
        now = [10.0]

        def slow_ps(*, timeout):
            now[0] += timeout
            return [{"instance": "smoke@baseline", "status": "running_healthy"}]

        with patch("smoke.time.monotonic", side_effect=lambda: now[0]), patch("smoke.time.sleep") as sleep, patch.object(runner, "ps", side_effect=slow_ps) as ps:
            with self.assertRaisesRegex(RuntimeError, "not observed"):
                runner.wait_for_markers(timeout=0.25)
        ps.assert_called_once_with(timeout=0.25)
        sleep.assert_called_once_with(0)
        self.assertEqual(now[0], 10.25)

    def test_command_timeout_marks_launch_failed_for_pre_teardown_quiescence(self):
        runner = self.runner("baseline")
        runner.supervisor = Mock()
        runner.supervisor.run.side_effect = subprocess.TimeoutExpired("fixture", 0.1)
        with self.assertRaises(subprocess.TimeoutExpired):
            runner.run(["fixture"], timeout=0.1)
        self.assertTrue(runner.failed)

    def test_noninteractive_commands_receive_eof_on_stdin(self):
        runner = self.runner("baseline")
        runner.supervisor = Mock()
        runner.supervisor.run.return_value = 0
        self.assertEqual(runner.run(["fixture"]), (0, ""))
        self.assertEqual(runner.supervisor.run.call_args.kwargs["stdin"], subprocess.DEVNULL)

    def test_empty_stores_do_not_hide_failed_child_quiescence(self):
        runner = self.runner("baseline")
        runner.supervisor = Mock()
        runner.supervisor.quiesce.side_effect = RuntimeError("owned child remains")
        with patch.object(runner, "ps", return_value=[]), patch.object(runner, "backend", return_value=[]):
            with self.assertRaisesRegex(RuntimeError, "owned child remains"):
                runner.cleanup()

    def test_supervisor_rejects_pre_existing_children(self):
        with patch.object(smoke.ChildSupervisor, "children", return_value={123}):
            with self.assertRaisesRegex(RuntimeError, "pre-existing children"):
                smoke.ChildSupervisor()

    def test_supervisor_refuses_pidfd_that_is_not_our_child(self):
        supervisor = smoke.ChildSupervisor.__new__(smoke.ChildSupervisor)
        supervisor.processes = []
        supervisor.pidfds = {}
        with patch.object(supervisor, "children", return_value={123}), patch("smoke.os.pidfd_open", return_value=77), patch("smoke.os.waitid", side_effect=ChildProcessError), patch("smoke.os.close") as close, patch("smoke.signal.pidfd_send_signal") as send:
            supervisor.refresh()
        self.assertEqual(supervisor.pidfds, {})
        close.assert_called_once_with(77)
        send.assert_not_called()

    def test_timeout_reaps_detached_double_fork_before_late_state_creation(self):
        self.check_detached_child_cleanup(timeout=True)

    def test_successful_launcher_exit_still_owns_detached_grandchild(self):
        self.check_detached_child_cleanup(timeout=False)

    def check_detached_child_cleanup(self, *, timeout):
        # The subreaper lives in a separate fixture process, not this test
        # process. Every fake descendant has a short self-expiry as a backstop.
        child = r'''
import json, os, signal, sys, time
from pathlib import Path
signal.signal(signal.SIGTERM, signal.SIG_IGN)
if os.fork() == 0:
    os.setsid()
    if os.fork() == 0:
        Path(sys.argv[1]).write_text(json.dumps({"pid": os.getpid(), "group": os.getpgrp()}))
        deadline = time.monotonic() + 5
        while not Path(sys.argv[3]).exists() and time.monotonic() < deadline:
            time.sleep(0.01)
        if Path(sys.argv[3]).exists():
            Path(sys.argv[2]).write_text("late state must never be created")
        os._exit(0)
    os._exit(0)
if sys.argv[4] == "timeout":
    time.sleep(5)
os._exit(0)
'''
        controller = f'''
import json, os, select, subprocess, sys, time
from pathlib import Path
sys.path.insert(0, {str(Path(smoke.__file__).parent)!r})
from smoke import ChildSupervisor
ready = Path({str(self.root / "ready")!r})
late = Path({str(self.root / "late")!r})
release = Path({str(self.root / "release")!r})
supervisor = ChildSupervisor()
try:
    timed_out = False
    try:
        result = supervisor.run([sys.executable, "-B", "-c", {child!r}, str(ready), str(late), str(release), {"timeout" if timeout else "exit"!r}], timeout=0.2)
        assert result == 0
    except subprocess.TimeoutExpired:
        timed_out = True
    assert timed_out == {timeout!r}
    deadline = time.monotonic() + 1
    while not ready.exists() and time.monotonic() < deadline:
        time.sleep(0.01)
    evidence = json.loads(ready.read_text())
    assert evidence["group"] != os.getpgrp()
    descendant_fd = os.pidfd_open(evidence["pid"])
    try:
        supervisor.quiesce(timeout=3, grace=0.05)
        exited = select.poll()
        exited.register(descendant_fd, select.POLLIN)
        assert exited.poll(1000), "detached descendant is still alive"
    finally:
        os.close(descendant_fd)
    assert supervisor.children() == set()
    release.write_text("allow the delayed state write only AFTER cleanup")
    time.sleep(0.05)
    assert not late.exists(), "detached producer escaped cleanup"
finally:
    supervisor.close()
print("OWNED_CHILDREN_REAPED")
'''
        result = subprocess.run([sys.executable, "-B", "-c", controller], capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("OWNED_CHILDREN_REAPED", result.stdout)


if __name__ == "__main__":
    unittest.main()
