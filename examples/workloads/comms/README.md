# Dependency communication fixture

Two small services share one Python image: `server` publishes an HTTP endpoint;
`client` resolves that declared dependency and checks its `DEPENDENCY_OK` response.
The client prints the resolved address and success marker, then stays alive for
five minutes so lifecycle and teardown can be observed.

```sh
nix flake check --no-update-lock-file
nix build --no-update-lock-file .#workestrate-network-smoke
```

The static image check starts no services. The directory is self-contained and
can live in its own Git repository; its pinned inputs follow shared tooling.

For an explicitly isolated runtime test, select this directory as the config and
start `workload up client`. Its declared dependency starts the server as needed.
The server uses an automatically allocated loopback host port, allows only local
TCP ingress to guest port 8080, and otherwise denies ingress and egress. The
client's egress permission is derived from its required `server` dependency;
there are no blanket Internet or host-network grants. The injected `SMOKE_SERVER`
value is `host:port`, without an HTTP scheme.

Stop the client first with `workload down client`, confirm the server remains
healthy, then stop the server with `workload down server`. Use the same isolated
home and microsandbox store for every command. Do not run this example against
a personal fleet containing workloads with these names. No credentials, host
mounts, image downloads inside the guest, or development-shell entry are needed.
