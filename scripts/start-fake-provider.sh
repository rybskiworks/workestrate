#!/usr/bin/env bash
# Start the fake OpenAI-compatible provider on 127.0.0.1:8081.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p var/log
nohup python3 infra/fake-provider/server.py \
  > var/log/fake-provider.stdout.log 2>&1 &
echo $! > var/run/fake-provider.pid
echo "fake-provider pid=$(cat var/run/fake-provider.pid) port=8081 log=var/log/fake-provider.log"
