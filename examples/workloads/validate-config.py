"""Validate example configs with an explicit CLI and disposable home directories."""

import os
from pathlib import Path
import subprocess
import sys
import tempfile

if len(sys.argv) != 2:
    raise SystemExit("usage: validate-config.py /absolute/path/to/workestrate")
cli = Path(sys.argv[1]).resolve(strict=True)
examples = Path(__file__).resolve().parent
with tempfile.TemporaryDirectory(prefix="workestrate-example-validation-") as scratch:
    for name in ("baseline", "comms", "nested"):
        root = Path(scratch) / name
        root.mkdir()
        (root / "msb.json").write_text("{}\n")
        env = {
            "PATH": os.defpath,
            "HOME": str(root),
            "WORKESTRATE_HOME": str(root / "tool"),
            "WORKESTRATE_CONFIG_DIR": str(examples / name),
            "WORKESTRATE_NO_PROJECT_CONFIG": "1",
            "WORKESTRATE_REFERENCE_CONFIG": "0",
            "WORKESTRATE_STATE_DIR": str(root / "state"),
            "MSB_HOME": str(root / "msb"),
            "MSB_CONFIG_PATH": str(root / "msb.json"),
            "MSB_BACKEND": "local",
        }
        print(f"Validating {name}", flush=True)
        subprocess.run(
            [str(cli), "--home", str(root / "tool"), "--no-project-config", "validate-config"],
            cwd=root, env=env, stdin=subprocess.DEVNULL, check=True, timeout=30,
        )
