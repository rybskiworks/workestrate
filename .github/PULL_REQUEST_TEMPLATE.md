## Change

Describe the problem and the smallest coherent fix. Link the issue or Bead.

## Evidence

Record exact commands, results and the commit tested. Distinguish evaluation,
compiled checks, native integration and KVM/runtime tests. List anything not run.

## Compatibility and safety

Describe pin/lock changes, public interface changes, state migrations and rollback.
Changes to `.github/`, security policy, schemas and dependency pins need explicit review.
Do not paste credentials, decrypted configuration or private workload logs.

- [ ] The PR targets `main` and contains no local build/session artifacts.
- [ ] Lockfiles and generated schemas changed only intentionally.
- [ ] Documentation describes implemented behavior, not planned capability.
- [ ] No live deployment, database migration or protection activation is implied.
