#!/usr/bin/env python3
"""Check source bounds and explicit metadata reads in standalone flakes."""

from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


GUARD = Path(__file__).with_name("check-nix-paths.sh")


class PurityTests(unittest.TestCase):
    def check_example(self, source):
        with tempfile.TemporaryDirectory(prefix="workestrate-purity-") as directory:
            root = Path(directory)
            for name in ("scripts", "nix", "templates", "examples/baseline", "docs"):
                (root / name).mkdir(parents=True, exist_ok=True)
            guard = root / "scripts/check-nix-paths.sh"
            shutil.copyfile(GUARD, guard)
            (root / "flake.nix").write_text("{}\n")
            (root / "justfile").write_text("")
            (root / "examples/baseline/flake.nix").write_text(source)
            return subprocess.run(["bash", str(guard)], capture_output=True, text=True, timeout=10)

    def test_example_with_unbounded_source_is_rejected(self):
        result = self.check_example("{ source = builtins.path { path = ./source; }; }\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("examples/baseline/flake.nix", result.stdout + result.stderr)

    def test_example_with_bounded_source_is_accepted(self):
        result = self.check_example('{ source = builtins.path {\n path = ./source;\n name = "fixture";\n filter = path: type: baseNameOf path == "Cargo.toml";\n}; }\n')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_formatted_single_file_manifest_read_is_accepted(self):
        result = self.check_example(
            "{ manifest = builtins.fromTOML (builtins.readFile ../../control/agentctl/Cargo.toml); }\n"
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_single_file_lock_metadata_read_is_accepted(self):
        result = self.check_example(
            "{ metadata = builtins.fromJSON (builtins.readFile ../../flake.lock); }\n"
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_read_file_does_not_exempt_root_or_directory_literals(self):
        for literal in ("../../", "../../control/agentctl", "../../Cargo.toml/.."):
            with self.subTest(literal=literal):
                result = self.check_example(f"{{ metadata = builtins.readFile {literal}; }}\n")
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("repo-root path literal", result.stdout + result.stderr)

    def test_metadata_read_does_not_hide_another_broad_path_on_the_line(self):
        for source in (
            "{ metadata = builtins.readFile ../../Cargo.toml; copied = ../../; }\n",
            "{ copied = ../../; metadata = builtins.readFile ../../Cargo.toml; }\n",
            "{ metadata = builtins.readFile ../../Cargo.toml + ../../; }\n",
            "{ metadata = [ (builtins.readFile ../../Cargo.toml) ../../ ]; }\n",
        ):
            with self.subTest(source=source):
                result = self.check_example(source)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("repo-root path literal", result.stdout + result.stderr)

    def test_plain_file_path_is_not_a_metadata_read(self):
        result = self.check_example("{ metadata = ../../control/agentctl/Cargo.toml; }\n")
        self.assertNotEqual(result.returncode, 0)

    def test_similarly_named_function_is_not_a_builtin_read(self):
        for function in ("notbuiltins.readFile", "custom.builtins.readFile"):
            with self.subTest(function=function):
                result = self.check_example(f"{{ metadata = {function} ../../Cargo.toml; }}\n")
                self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
