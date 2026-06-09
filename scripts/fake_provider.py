#!/usr/bin/env python3
"""Thin wrapper so `scripts/fake_provider.py` exists as the task spec
requires. The real implementation lives in infra/fake-provider/server.py
and is launched by scripts/start-fake-provider.sh.

Running this file directly is equivalent to running that one.
"""
import os
import runpy
import sys

here = os.path.dirname(os.path.abspath(__file__))
real = os.path.normpath(os.path.join(here, "..", "infra", "fake-provider", "server.py"))
sys.argv[0] = real
runpy.run_path(real, run_name="__main__")
