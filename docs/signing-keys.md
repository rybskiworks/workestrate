# Encrypted SSH signing keys

Workestrate can generate an Ed25519 signing key in an existing registered
config repository's encrypted secrets file. This provisions key material; it
does not grant any workload access to it or configure Git signing.

```sh
workestrate --home /path/to/tool-home credentials signing generate \
  MACHINE_GIT_SIGNING_KEY --config personal
workestrate --home /path/to/tool-home credentials signing public-key \
  MACHINE_GIT_SIGNING_KEY --config personal
```

The name is the environment key inside the SOPS document, not a GitHub username
or an SSH authentication credential. `--config` is mandatory and selects the
same write-side target as `workestrate secrets-target`. The encrypted file must
already exist; initialize it through the existing secrets workflow first.
Registry store, relative secrets-file and age-key overrides are respected.
Targets outside the selected directory, symlinks and unsafe writable paths are
refused. The age identity must be owner-private.

The private key is generated in memory. The command asks SOPS to add its value
through standard input to a private staged copy of the existing ciphertext,
preserving that document's encryption recipients. It never writes the private
key to a plaintext file or passes it as a command-line argument. A real decrypt
roundtrip verifies the complete multiline key and every existing value before
atomic ciphertext replacement. The resulting encrypted file is mode 0600.

Generation refuses a pre-existing name, even if its value is empty. It does not
rotate keys or create new age identities. Concurrent generators serialize on
the selected parent directory. Changes observed from other editors cause a
refusal; this advisory lock is not a mandatory lock on unrelated editors.
Existing ciphertext is retained when generation, encryption or verification
fails. A durability error after replacement explicitly reports that the key
was saved; use `public-key` to inspect before retrying.

Only ciphertext is staged on disk. An abrupt termination can leave a private
`.signing-key-*` staging directory containing ciphertext, not a plaintext key;
normal completion and errors remove the owned staging directory. SOPS
operations have bounded output and a 30-second deadline. Private subprocess
diagnostics are never printed. The command disables ordinary core dumps before
handling private material; SOPS inherits that resource limit. This is not a
substitute for the host's swap, privileged crash-collector or memory-inspection
policy: decrypted values necessarily exist in process memory during use.

Standard output is the copyable public-key line. The fingerprint and target
are printed to standard error. Global `--json` instead returns public metadata
with `name`, `config`, `secrets_file`, `public_key`, `fingerprint` and `created`.
The `public-key` command decrypts only the selected secrets file to derive the
public key; it never prints private material and does not change ciphertext.

## Registering and consuming the key

Add the public key to the intended GitHub account as a **signing key**. Check
the account before using any upload command: a human operator's authenticated
GitHub CLI will not implicitly target a machine account. Registration for SSH
login is separate and is not needed merely to sign Git commits over HTTPS.

Keep Git author attribution, machine committer metadata, HTTPS authentication
tokens and signing-key authority separate. These commands do not edit global
Git configuration, SSH agents, workload grants, secret definitions, config
registry revisions, GitHub accounts or the installed Workestrate profile. A
Git-backed config consumer must adopt its new encrypted-file revision through
the ordinary reviewed config publication/update workflow.

Runtime signing requires an explicit signing catalog entry, the `git`
namespace and an authorized workload grant. Existing broker key custody
requires a clear OpenSSH Ed25519 key **inside the SOPS encryption envelope**;
an additional SSH passphrase is not supported. Merely generating the key does
not establish that a guest Git adapter is connected. Validate an actual signed
commit and the relevant denial cases before enabling production signing.

Back up the encrypted config and its age identity separately. The public key
alone cannot recover signing capability. No encryption identity or private
signing material belongs in a Nix expression or the Nix store.
