#!/usr/bin/env python3
"""Exercise shell forwarding and caller-owned paths with an isolated Nix stub."""

import json
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest


JUSTFILE = Path(__file__).resolve().parent.parent / "justfile"
CARGO_TARGET = JUSTFILE.parent / "scripts/cargo-target.sh"
FLAKE = JUSTFILE.parent / "flake.nix"


class ShellArgumentsTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="workestrate-shell-")
        self.addCleanup(self.directory.cleanup)
        self.outer = Path(self.directory.name)
        self.root = self.outer / "checkout with spaces"
        self.root.mkdir()
        self.bin = self.root / "bin"
        self.bin.mkdir()
        for name in ("just", "bash", "sh", "mkdir", "sha256sum", "cut", "realpath"):
            executable = shutil.which(name)
            self.assertIsNotNone(executable, f"required fixture tool missing: {name}")
            (self.bin / name).symlink_to(executable)
        shutil.copyfile(JUSTFILE, self.root / "justfile")
        (self.root / "scripts").mkdir()
        shutil.copyfile(CARGO_TARGET, self.root / "scripts/cargo-target.sh")
        # Exercise the non-Git root fallback without inspecting any real checkout.
        self.write_tool("git", "#!/bin/sh\nexit 1\n")
        self.write_tool(
            "nix",
            f"#!{sys.executable}\n"
            "import json, os, pathlib, sys\n"
            "pathlib.Path(os.environ['NIX_ARGV_LOG']).write_text(json.dumps(sys.argv[1:]))\n"
            "keys = ('CARGO_TARGET_DIR', 'BEADS_DIR', 'DOLT_ROOT_PATH', 'HOME', 'XDG_CACHE_HOME')\n"
            "pathlib.Path(os.environ['NIX_ENV_LOG']).write_text(json.dumps({k: os.environ.get(k) for k in keys}))\n"
            "sys.exit(int(os.environ.get('NIX_EXIT_CODE', '0')))\n",
        )
        self.env = {
            "PATH": str(self.bin),
            "HOME": str(self.outer / "home with spaces"),
            "LC_ALL": "C",
            "NIX_ARGV_LOG": str(self.root / "nix-argv.json"),
            "NIX_ENV_LOG": str(self.root / "nix-env.json"),
        }

    def write_tool(self, name, text):
        path = self.bin / name
        path.write_text(text)
        path.chmod(0o755)

    def check_recipe(self, recipe, arguments, *, exit_code=0):
        result = subprocess.run(
            [str(self.bin / "just"), recipe, *arguments],
            cwd=self.root,
            env={**self.env, "NIX_EXIT_CODE": str(exit_code)},
            text=True,
            capture_output=True,
            timeout=10,
        )
        self.assertEqual(result.returncode, exit_code, result.stderr)
        forwarded = json.loads((self.root / "nix-argv.json").read_text())
        self.assertEqual(forwarded[:3], ["develop", "--override-input", "devenv-root"])
        prefix = "file+file://"
        self.assertTrue(forwarded[3].startswith(prefix))
        root_file = Path(forwarded[3].removeprefix(prefix))
        self.assertTrue(root_file.is_relative_to(self.env["HOME"]))
        self.assertEqual(root_file.read_bytes(), str(self.root).encode())
        prefix_args = {
            "bootstrap": [".#bootstrap"],
        }.get(recipe, [])
        expected = prefix_args + arguments
        self.assertEqual(forwarded[4:], expected)

    def observed_environment(self):
        return json.loads((self.root / "nix-env.json").read_text())

    def select_target(self, overrides=None, *, root=None):
        env = {**self.env, **(overrides or {})}
        env = {key: value for key, value in env.items() if value is not None}
        return subprocess.run(
            [str(self.bin / "bash"), str(self.root / "scripts/cargo-target.sh"), str(root or self.root)],
            cwd=self.root,
            env=env,
            stdin=subprocess.DEVNULL,
            text=True,
            capture_output=True,
            timeout=10,
        )

    def test_no_command_arguments(self):
        for recipe in ("shell", "bootstrap"):
            with self.subTest(recipe=recipe):
                self.check_recipe(recipe, [])

    def test_simple_command_arguments(self):
        for recipe in ("shell", "bootstrap"):
            with self.subTest(recipe=recipe):
                self.check_recipe(recipe, ["-c", "rustc", "--version"])

    def test_shell_text_remains_literal_and_never_runs_on_host(self):
        marker = self.root / "must not be created"
        arguments = [
            "-c",
            "bash",
            "-c",
            f"printf '%s\\n' 'inside shell'; printf escaped > {shlex.quote(str(marker))}",
            "",
            "argument with spaces",
            '"double quotes" and \'single quotes\'',
            "$(printf expanded)",
            "`printf expanded`",
            "*",
        ]
        for recipe in ("shell", "bootstrap"):
            with self.subTest(recipe=recipe):
                self.check_recipe(recipe, arguments)
                self.assertFalse(marker.exists(), "command text escaped into the host shell")

    def test_nix_failure_is_propagated(self):
        for recipe in ("shell", "bootstrap"):
            with self.subTest(recipe=recipe):
                self.check_recipe(recipe, ["-c", "false"], exit_code=23)

    def test_secrets_target_arguments_remain_literal(self):
        # The setup-secrets recipe is a thin host-side alias with the same
        # verb-hoisting + default-init adapter as scripts/setup-secrets.sh
        # and the flake's setup-secrets wrapper: the init/update verb is
        # hoisted before the target-selector flags, a bare invocation
        # defaults to init, and --help without a verb passes through to the
        # CLI group help. No nix develop; arguments are forwarded literally.
        # Stub the CLI to capture argv and control the exit code.
        self.write_tool(
            "workestrate",
            f"#!{sys.executable}\n"
            "import json, os, pathlib, sys\n"
            "pathlib.Path(os.environ['CLI_ARGV_LOG']).write_text(json.dumps(sys.argv[1:]))\n"
            "sys.exit(int(os.environ.get('CLI_EXIT_CODE', '0')))\n",
        )
        env = {**self.env, "CLI_ARGV_LOG": str(self.root / "cli-argv.json")}
        marker = self.root / "must not be created"
        # Cases are (recipe argv, expected CLI argv): the verb may appear
        # before or after the selector flags, option values named init/update
        # must not be mistaken for the verb, `--opt=value` forms are kept
        # verbatim, and a bare invocation gains `init` while staying literal
        # (no shell expansion of metacharacters).
        hoist_cases = (
            (["--config", str(self.outer / "operator config"), "--fleet", "personal", "update"],
             ["secrets", "update", "--config", str(self.outer / "operator config"), "--fleet", "personal"]),
            (["update", "--config", str(self.outer / "operator config"), "--fleet", "personal"],
             ["secrets", "update", "--config", str(self.outer / "operator config"), "--fleet", "personal"]),
            (["--fleet", "personal", "update"], ["secrets", "update", "--fleet", "personal"]),
            (["--config=personal", "update"], ["secrets", "update", "--config=personal"]),
            (["update", "--config=personal"], ["secrets", "update", "--config=personal"]),
            (["--fleet", "init"], ["secrets", "init", "--fleet", "init"]),
            (["--config=init"], ["secrets", "init", "--config=init"]),
            (["--fleet-dir", "update"], ["secrets", "init", "--fleet-dir", "update"]),
            (["--config", "init"], ["secrets", "init", "--config", "init"]),
            (["--fleet", "personal"], ["secrets", "init", "--fleet", "personal"]),
            ([], ["secrets", "init"]),
            (["--global"], ["secrets", "init", "--global"]),
            (["init", "--global"], ["secrets", "init", "--global"]),
            (["--global", "init"], ["secrets", "init", "--global"]),
            (["--help"], ["secrets", "--help"]),
            (["-h"], ["secrets", "-h"]),
            ([f"$(touch {shlex.quote(str(marker))}); `false`", "update"],
             ["secrets", "update", f"$(touch {shlex.quote(str(marker))}); `false`"]),
            (["--fleet-dir", f"$(touch {shlex.quote(str(marker))}); `false`", "update"],
             ["secrets", "update", "--fleet-dir", f"$(touch {shlex.quote(str(marker))}); `false`"]),
        )
        for arguments, expected in hoist_cases:
            with self.subTest(arguments=arguments):
                result = subprocess.run(
                    [str(self.bin / "just"), "setup-secrets", *arguments],
                    cwd=self.root, env=env, text=True, capture_output=True, timeout=10,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                forwarded = json.loads((self.root / "cli-argv.json").read_text())
                self.assertEqual(forwarded, expected)
                self.assertFalse(marker.exists())
                self.assertFalse((self.root / "nix-argv.json").exists(), "recipe must not enter nix")
        # A missing option value errors before delegating to the CLI.
        for arguments in (["--fleet"], ["--fleet-dir"], ["--config"]):
            with self.subTest(arguments=arguments):
                before = (self.root / "cli-argv.json").read_text() if (self.root / "cli-argv.json").exists() else None
                result = subprocess.run(
                    [str(self.bin / "just"), "setup-secrets", *arguments],
                    cwd=self.root, env=env, text=True, capture_output=True, timeout=10,
                )
                self.assertNotEqual(result.returncode, 0, result.stderr)
                self.assertIn("requires a value", result.stderr)
                self.assertFalse((self.root / "nix-argv.json").exists(), "recipe must not enter nix")
                after = (self.root / "cli-argv.json").read_text() if (self.root / "cli-argv.json").exists() else None
                self.assertEqual(after, before, "missing option value must not reach the CLI")
        result = subprocess.run(
            [str(self.bin / "just"), "setup-secrets", "--fleet", "personal", "update"],
            cwd=self.root, env={**env, "CLI_EXIT_CODE": "23"},
            text=True, capture_output=True, timeout=10,
        )
        self.assertEqual(result.returncode, 23, result.stderr)

    def test_shells_keep_explicit_external_target_literal(self):
        target = str(self.outer / 'cache with spaces' / '$(touch injected); `false` "quoted"')
        self.env["CARGO_TARGET_DIR"] = target
        self.env["XDG_CACHE_HOME"] = str(self.root / "unused invalid cache")
        for recipe in ("shell", "bootstrap"):
            with self.subTest(recipe=recipe):
                self.check_recipe(recipe, ["-c", "true"])
                self.assertEqual(self.observed_environment()["CARGO_TARGET_DIR"], target)
                self.assertFalse(Path(target).exists())
                self.assertFalse((self.root / "injected").exists())

    def test_default_and_empty_cargo_target_use_external_xdg_or_home(self):
        for recipe in ("shell", "bootstrap"):
            for explicit in (None, ""):
                for cache in (None, "", str(self.outer / "xdg cache")):
                    with self.subTest(recipe=recipe, explicit=explicit, cache=cache):
                        self.env.pop("CARGO_TARGET_DIR", None)
                        self.env.pop("XDG_CACHE_HOME", None)
                        if explicit is not None:
                            self.env["CARGO_TARGET_DIR"] = explicit
                        if cache is not None:
                            self.env["XDG_CACHE_HOME"] = cache
                        self.check_recipe(recipe, [])
                        expected = Path(cache or str(Path(self.env["HOME"]) / ".cache")) / "ai-workbench/agentctl-target"
                        self.assertEqual(self.observed_environment()["CARGO_TARGET_DIR"], str(expected))
                        self.assertFalse(expected.exists())

    def test_relative_and_checkout_targets_fail_before_nix_or_shell_setup(self):
        alias = self.outer / "checkout alias"
        alias.symlink_to(self.root, target_is_directory=True)
        for target in ("target", "../outside", str(self.root), str(self.root / "target"),
                       str(self.root / "sub/../target"), str(alias / "target")):
            for recipe in ("shell", "bootstrap", "beads"):
                with self.subTest(target=target, recipe=recipe):
                    result = subprocess.run(
                        [str(self.bin / "just"), recipe], cwd=self.root,
                        env={**self.env, "CARGO_TARGET_DIR": target},
                        stdin=subprocess.DEVNULL, text=True, capture_output=True, timeout=10,
                    )
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("CARGO_TARGET_DIR", result.stderr)
                    self.assertFalse((self.root / "nix-argv.json").exists())
                    self.assertFalse(Path(self.env["HOME"]).exists())
                    self.assertFalse((self.root / ".beads").exists())

    def test_default_cache_inside_checkout_or_relative_is_rejected(self):
        for cache in ("relative-cache", str(self.root / "cache")):
            with self.subTest(cache=cache):
                result = self.select_target({"XDG_CACHE_HOME": cache})
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(result.stdout, "")
                self.assertFalse((self.root / "cache").exists())

    def test_source_root_alias_and_external_prefix_are_handled(self):
        alias = self.outer / "source alias"
        alias.symlink_to(self.root, target_is_directory=True)
        denied = self.select_target({"CARGO_TARGET_DIR": str(self.root / "target")}, root=alias)
        self.assertNotEqual(denied.returncode, 0)
        external = str(self.outer / (self.root.name + "-cache") / "future/../target")
        accepted = self.select_target({"CARGO_TARGET_DIR": external}, root=alias)
        self.assertEqual(accepted.returncode, 0, accepted.stderr)
        self.assertEqual(accepted.stdout, external + "\n")
        self.assertFalse((self.outer / (self.root.name + "-cache")).exists())

    def test_external_symlink_is_preserved_but_its_target_is_checked(self):
        alias = self.outer / "cache alias"
        alias.symlink_to(self.outer / "missing external cache", target_is_directory=True)
        target = str(alias / "target")
        result = self.select_target({"CARGO_TARGET_DIR": target})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, target + "\n")
        self.assertFalse(alias.exists())

    def test_missing_home_only_fails_when_the_fallback_needs_it(self):
        for home in (None, ""):
            with self.subTest(home=home):
                failed = self.select_target({"HOME": home, "XDG_CACHE_HOME": "", "CARGO_TARGET_DIR": ""})
                self.assertNotEqual(failed.returncode, 0)
                self.assertIn("HOME is required", failed.stderr)
                for key in ("CARGO_TARGET_DIR", "XDG_CACHE_HOME"):
                    target = str(self.outer / "explicit without home")
                    accepted = self.select_target({"HOME": home, key: target})
                    self.assertEqual(accepted.returncode, 0, accepted.stderr)
                    self.assertFalse(Path(target).exists())

    def test_newline_target_and_symlink_loop_are_rejected_without_writes(self):
        loop = self.outer / "loop"
        loop.symlink_to(loop)
        for target in (str(self.outer / "target\n"), str(loop / "target")):
            with self.subTest(target=target):
                result = self.select_target({"CARGO_TARGET_DIR": target})
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(result.stdout, "")

    def test_both_enter_shell_bodies_validate_before_other_setup(self):
        source = FLAKE.read_text()
        for name in ("bootstrap", "default"):
            with self.subTest(shell=name):
                shell = source.split(f"devenv.shells.{name} = {{", 1)[1]
                body = shell.split("enterShell = ''", 1)[1].split("'';", 1)[0]
                lines = [line.strip() for line in body.splitlines() if line.strip()]
                expected = (
                    'CARGO_TARGET_DIR="$(' + '${pkgs.bash}/bin/bash ${./scripts/cargo-target.sh} '
                    + '${pkgs.lib.escapeShellArg config.devenv.shells.' + name + '.devenv.root})" || exit 1'
                )
                self.assertEqual(lines[:2], [expected, "export CARGO_TARGET_DIR"])
                self.assertEqual(body.count("CARGO_TARGET_DIR="), 1)
                # Run this exact assignment/export projection, replacing only
                # the three Nix interpolations with explicit fixture inputs.
                projected = expected.replace('${pkgs.bash}/bin/bash', shlex.quote(str(self.bin / "bash")))
                projected = projected.replace('${./scripts/cargo-target.sh}', shlex.quote(str(self.root / "scripts/cargo-target.sh")))
                projected = projected.replace('${pkgs.lib.escapeShellArg config.devenv.shells.' + name + '.devenv.root}', shlex.quote(str(self.root)))
                for target, accepted in ((str(self.outer / "shell cache"), True), (str(self.root / "target"), False)):
                    result = subprocess.run(
                        [str(self.bin / "bash"), "-c", projected + '\nexport CARGO_TARGET_DIR\nprintf "%s" "$CARGO_TARGET_DIR"'],
                        env={**self.env, "CARGO_TARGET_DIR": target}, cwd=self.root,
                        stdin=subprocess.DEVNULL, text=True, capture_output=True, timeout=10,
                    )
                    self.assertEqual(result.returncode == 0, accepted, result.stderr)
                    self.assertEqual(result.stdout, target if accepted else "")

    def test_beads_uses_pinned_app_and_preserves_arguments(self):
        arguments = ["create", "title with spaces", "--description", "$(do not expand)"]
        result = subprocess.run(
            [str(self.bin / "just"), "beads", *arguments],
            cwd=self.root,
            env=self.env,
            text=True,
            capture_output=True,
            timeout=10,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        forwarded = json.loads((self.root / "nix-argv.json").read_text())
        self.assertEqual(
            forwarded,
            [
                "run", "--no-update-lock-file", ".#beads", "--", "--sandbox",
                "-C", str(self.root), *arguments,
            ],
        )
        self.assertFalse((self.root / ".beads").exists())

    def test_beads_preserves_explicit_state_and_uses_only_missing_fallbacks(self):
        external = str(self.outer / 'shared beads "literal" $(false)')
        dolt = str(self.outer / "private dolt")
        for beads in (None, "", external):
            for root in (None, "", dolt):
                with self.subTest(beads=beads, dolt=root):
                    env = dict(self.env)
                    if beads is not None:
                        env["BEADS_DIR"] = beads
                    if root is not None:
                        env["DOLT_ROOT_PATH"] = root
                    result = subprocess.run(
                        [str(self.bin / "just"), "beads", "ready"], cwd=self.root, env=env,
                        stdin=subprocess.DEVNULL, text=True, capture_output=True, timeout=10,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    observed = self.observed_environment()
                    expected_beads = beads or str(self.root / ".beads")
                    self.assertEqual(observed["BEADS_DIR"], expected_beads)
                    self.assertEqual(observed["DOLT_ROOT_PATH"], root or expected_beads + "/dolt-global")
                    self.assertFalse(Path(expected_beads).exists())
                    self.assertFalse(Path(observed["DOLT_ROOT_PATH"]).exists())

    def test_beads_failure_propagates_with_external_state(self):
        result = subprocess.run(
            [str(self.bin / "just"), "beads", "ready"], cwd=self.root,
            env={**self.env, "BEADS_DIR": str(self.outer / "beads"), "NIX_EXIT_CODE": "23"},
            stdin=subprocess.DEVNULL, text=True, capture_output=True, timeout=10,
        )
        self.assertEqual(result.returncode, 23, result.stderr)
        self.assertFalse((self.outer / "beads").exists())


if __name__ == "__main__":
    unittest.main()
