#!/usr/bin/env python3
"""Rebuild the real version script in a dependency-free, retained Cargo target."""

import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import time
import unittest


DIRECTIVE = '    println!("cargo:rerun-if-env-changed=WORKESTRATE_REV");\n'
OPTIONS = None


def run(command, *, cwd, env, timeout):
    with subprocess.Popen(
        command, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        start_new_session=True,
    ) as child:
        try:
            stdout, stderr = child.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            # Cargo can own a compiler/linker: reap the whole fixture group.
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            child.communicate()
            raise
        return subprocess.CompletedProcess(command, child.returncode, stdout, stderr)


class BuildRevisionTests(unittest.TestCase):
    def fixture(self, *, track_revision=True):
        temporary = tempfile.TemporaryDirectory(prefix="workestrate-version-")
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        script = OPTIONS.build_script.read_text()
        if not track_revision:
            self.assertEqual(script.count(DIRECTIVE), 1)
            script = script.replace(DIRECTIVE, "")
        (root / "build.rs").write_text(script)
        (root / "Cargo.toml").write_text(
            '[package]\nname = "version-fixture"\nversion = "0.1.0"\n'
            'edition = "2024"\n[workspace]\n'
        )
        (root / "Cargo.lock").write_text(
            'version = 4\n[[package]]\nname = "version-fixture"\nversion = "0.1.0"\n'
        )
        (root / "src").mkdir()
        (root / "src/main.rs").write_text(
            'fn main() { println!("{}", env!("WORKESTRATE_VERSION")); }\n'
        )
        (root / ".cargo").mkdir()
        (root / ".cargo/config.toml").write_text(
            '[target.\'cfg(all())\']\nlinker = ' + json.dumps(str(OPTIONS.linker)) + "\n"
        )
        for name in ("home", "cargo-home", "target"):
            (root / name).mkdir()
        env = {
            "PATH": os.defpath,
            "HOME": str(root / "home"),
            "CARGO_HOME": str(root / "cargo-home"),
            "CARGO_TARGET_DIR": str(root / "target"),
            "CARGO_INCREMENTAL": "0",
            "CARGO_PROFILE_DEV_DEBUG": "0",
            "RUSTC": str(OPTIONS.rustc),
            "LC_ALL": "C",
        }
        if OPTIONS.sysroot is not None:
            # Fenix exposes compiler and target standard library separately.
            env["CARGO_ENCODED_RUSTFLAGS"] = "--sysroot\x1f" + str(OPTIONS.sysroot)
        return root, env

    def rebuild(self, root, env, revision):
        child_env = dict(env)
        if revision is not None:
            child_env["WORKESTRATE_REV"] = revision
        remaining = OPTIONS.deadline - time.monotonic()
        self.assertGreater(remaining, 0, "revision regression deadline exceeded")
        result = run(
            [str(OPTIONS.cargo), "build", "--locked", "--offline", "--jobs", "1",
             "--message-format=json"],
            cwd=root, env=child_env, timeout=remaining,
        )
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout[-16384:])
        artifacts = [
            item for line in result.stdout.splitlines()
            if (item := json.loads(line)).get("reason") == "compiler-artifact"
            and item["target"]["kind"] == ["bin"]
        ]
        self.assertEqual(len(artifacts), 1)
        binary = root / "target/debug/version-fixture"
        self.assertEqual(Path(artifacts[0]["executable"]), binary)
        observed = run(
            [str(binary)], cwd=root, env=child_env,
            timeout=min(5, max(0.01, OPTIONS.deadline - time.monotonic())),
        )
        self.assertEqual(observed.returncode, 0, observed.stderr)
        return observed.stdout.strip(), artifacts[0]["fresh"]

    def test_revision_changes_and_removal_rebuild_the_same_target(self):
        root, env = self.fixture()
        sources = {
            path: path.read_bytes() for path in root.rglob("*") if path.is_file()
        }
        target_identity = (root / "target").stat().st_ino
        for revision, expected, fresh in (
            ("revision-a", "0.1.0-revision-a", False),
            ("revision-b", "0.1.0-revision-b", False),
            (None, "0.1.0-dev", False),
            (None, "0.1.0-dev", True),
        ):
            with self.subTest(revision=revision, fresh=fresh):
                self.assertEqual(self.rebuild(root, env, revision), (expected, fresh))
                self.assertEqual((root / "target").stat().st_ino, target_identity)
                self.assertEqual({path: path.read_bytes() for path in sources}, sources)

    def test_without_environment_tracking_the_retained_target_is_stale(self):
        root, env = self.fixture(track_revision=False)
        self.assertEqual(self.rebuild(root, env, "revision-a"), ("0.1.0-revision-a", False))
        self.assertEqual(self.rebuild(root, env, "revision-b"), ("0.1.0-revision-a", True))


def executable(value):
    path = Path(value).absolute()
    if not path.is_file() or not os.access(path, os.X_OK):
        raise argparse.ArgumentTypeError(f"not an executable file: {path}")
    return path


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name, default in (("cargo", "cargo"), ("rustc", "rustc"), ("linker", "cc")):
        path = shutil.which(default)
        parser.add_argument(f"--{name}", type=executable, default=path, required=not path)
    parser.add_argument(
        "--build-script", type=Path,
        default=Path(__file__).resolve().parent.parent / "control/agentctl/build.rs",
    )
    parser.add_argument("--sysroot", type=Path)
    OPTIONS, unittest_args = parser.parse_known_args()
    OPTIONS.deadline = time.monotonic() + 60
    unittest.main(argv=[__file__, *unittest_args])
