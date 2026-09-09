#!/usr/bin/env bash
set -euo pipefail

mkdir -p /tmp/inner/user /tmp/inner/config /tmp/inner/cwd
cp @childConfig@ /tmp/inner/config/workestrate.toml
cd /tmp/inner/cwd

wk() {
  env -i HOME=/tmp/inner/user PATH=/bin \
    SSL_CERT_FILE=@caBundle@ NIX_SSL_CERT_FILE=@caBundle@ \
    MSB_HOME=/tmp/inner/msb MSB_CONFIG_PATH=@backendConfig@ MSB_BACKEND=local \
    WORKESTRATE_STATE_DIR=/tmp/inner/state WORKESTRATE_CONFIG_DIR=/tmp/inner/config \
    WORKESTRATE_REFERENCE_CONFIG=0 \
    timeout -k 5s 30s @workestrate@/bin/workestrate --home /tmp/inner/tool --no-project-config "$@"
}

cleanup() {
  wk workload down child --instance one || echo INNER_CLEANUP_FAILED
}
trap cleanup EXIT

python3 -c 'import fcntl,os; fd=os.open("/dev/kvm",os.O_RDWR|os.O_CLOEXEC); api=fcntl.ioctl(fd,0xAE00,0); os.close(fd); print("OUTER_KVM_API="+str(api),flush=True); assert api==12'
wk --version
wk validate-config
gzip -dc @childImage@ | wk msb -- image load --tag workestrate-smoke:latest
wk workload plan child
wk workload up child --instance one
for attempt in $(seq 1 20); do
  names=$(wk msb -- list --running --quiet)
  if [ -n "$names" ]; then break; fi
  sleep 1
done
test "$(printf '%s' "$names" | wc -w)" -eq 1
printf 'INNER_BACKEND=%s\n' "$names"
wk ps --json
# Buffered exec consumes stdin before launching: provide EOF explicitly.
wk msb -- exec "$names" --no-tty --timeout 15s -- /bin/sh -c 'echo INNER_EXEC_OK; pwd; uname -m' </dev/null
wk workload down child --instance one
test -z "$(wk msb -- list --quiet)"
trap - EXIT
echo NESTED_TEARDOWN_OK
echo NESTED_WORKESTRATE_OK
# Keep success inspectable until exact host teardown; the workload has a
# separate 180-second command limit. Command exit is not a VM cleanup method.
exec sleep infinity
