"""Inspect the nested image contract without extraction, KVM, or VM startup."""

import json
import sys
import tarfile

with tarfile.open(sys.argv[1]) as archive:
    manifest = json.load(archive.extractfile("manifest.json"))
    assert len(manifest) == 1, "expected exactly one image"
    entry = manifest[0]
    assert entry["RepoTags"] == ["workestrate-nested-smoke:latest"]
    config = json.load(archive.extractfile(entry["Config"]))["config"]
    assert config["WorkingDir"] == "/tmp"
    assert config["Cmd"] == ["/bin/nested-smoke"]
    assert "PATH=/bin" in config["Env"]
    paths = set()
    for layer in entry["Layers"]:
        with tarfile.open(fileobj=archive.extractfile(layer)) as contents:
            for member in contents:
                path = member.name.removeprefix("./").rstrip("/")
                paths.add(path)
                if path == "tmp":
                    assert member.mode & 0o1777 == 0o1777, "tmp must be writable"
    required = {
        "bin/sh", "bin/sleep", "bin/timeout", "bin/python3",
        "bin/workestrate", "bin/nested-smoke", "etc/passwd",
        "etc/ssl/certs/ca-bundle.crt", "tmp",
    }
    assert required <= paths, f"missing image paths: {sorted(required - paths)}"
    assert any(path.endswith("-workestrate-smoke.tar.gz") for path in paths)
print("Nested image contract passed; runtime KVM gate was not executed")
