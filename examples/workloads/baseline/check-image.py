"""Inspect the built image without extracting it or starting a workload."""

import json
import sys
import tarfile

image, tag, *extra_paths = sys.argv[1:]
with tarfile.open(image) as archive:
    manifest = json.load(archive.extractfile("manifest.json"))
    assert len(manifest) == 1, "expected exactly one image"
    entry = manifest[0]
    assert entry["RepoTags"] == [tag], entry["RepoTags"]
    config = json.load(archive.extractfile(entry["Config"]))["config"]
    assert config["WorkingDir"] == "/tmp", config
    assert config["Cmd"] == ["/bin/sh"], config
    assert "PATH=/bin" in config["Env"], config
    paths = set()
    for layer in entry["Layers"]:
        with tarfile.open(fileobj=archive.extractfile(layer)) as contents:
            for member in contents:
                path = member.name.removeprefix("./").rstrip("/")
                paths.add(path)
                if path == "tmp":
                    assert member.mode & 0o1777 == 0o1777, "tmp must be writable"
    required = {"bin/sh", "bin/sleep", "etc/passwd", "tmp", *extra_paths}
    assert required <= paths, f"missing image paths: {sorted(required - paths)}"
print(f"Image contract passed: {tag}")
