# Managed broker image

This infrastructure leaf takes explicit `pkgs`, shared `guest` and `base`, an
immutable `brokerd` package, and an exact lowercase DNS `hostPrincipal`. It uses
`guest.mkNixosLayer`; the parent's NixOS system, `/init`, store registration,
network setup, accounts and trust policy remain unchanged.

The service and its `multi-user.target.wants` link are installed under the
standard `/usr/local/lib/systemd/system` unit search path. This avoids the
parent's activation-owned `/etc/systemd/system` without an init wrapper, a
generator, a store mutation or a command executed through the guest agent.
The selected shared base does not override `SYSTEMD_UNIT_PATH`. Its systemd
recognizes this administrator unit path and merges target dependencies from
the unit search directories. The host owner must preserve the exported
`guestInit` contract and not inject `SYSTEMD_UNIT_PATH` or replace the boot target
through extra init arguments. See [systemd.unit](https://github.com/systemd/systemd/blob/v260.1/man/systemd.unit.xml).

Before boot, the trusted host owner must mount its private, immutable credential
directory read-only at `/broker-credentials`, containing exactly:

- `host-key`: the broker's private Ed25519 host key;
- `host-certificate`: its matching CA-signed host certificate, valid for the
  exact configured principal;
- `host-ca.pub`: the CA public key, distinct from the broker host key.

The CA private key stays on the host. No credential contents are Nix inputs,
image layers, command arguments or environment values. `LoadCredential` copies
the runtime files into the service's protected credential directory; `%d` passes
only that directory's path. Missing files prevent a usable service; no default
credentials or path-existence condition silently skip startup. The executable
validates file safety, the key/certificate pairing, CA signature, host type,
exact principal and validity before opening any listener. Raw-key-only startup
is not supported. The owner must not replace credential files underneath a
running service; rotation requires a new validated service incarnation.

The unit requires `guest-store-ready.target`. It runs the ordinary `brokerd
service` command as root, not brokerd's legacy PID 1 mode. Management port 3024
and diversion port 3022 require distinct protected host-listen mappings; egress
port 3023 requires the existing host forwarder. No provider TCP port is opened
by the image declaration. Automatic service restart is disabled so failure is
visible to the retained host owner. The 30-second systemd stop limit is a
supervision bound, not proof that every SSH session was retired gracefully.

The host must retain the selected broker sandbox and verify the management
transport before adopting its Welcome. Unit activation or a live management
connection is not policy readiness: every launch needs its own current Applied
observation and heartbeat lease. Managed SSH clients require certificate-only
host-key negotiation, a destination-scoped HostKeyAlias and the CA public pin;
the broker independently verifies each upstream host key.

Run the portable source contract with:

```sh
python3 -B nix/images/broker/test_broker_image.py
```

These tests check the unit/template and installation contract without building
an image or starting a service. Actual archive inheritance/registration checks,
credential-mount startup, direct management and guest shutdown acceptance are
separate runtime gates.
