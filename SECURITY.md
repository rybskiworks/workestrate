# Security

This project is under active development. A passing build is not evidence that
all isolation, credential, kernel or runtime boundaries have been validated.

Use GitHub private vulnerability reporting from the Security tab when enabled.
Otherwise request a private channel from the maintainer without including exploit
details, credentials, private configurations or live workload logs in a public
issue. No response-time guarantee is implied.

Provide the exact source commit, locked dependencies, host/platform, expected
boundary, observed behavior and a minimal disposable reproduction. Test only
infrastructure you own or are authorized to test. Rotate exposed credentials;
deleting a file alone does not revoke them.

Keep encrypted examples and synthetic fixtures distinct from live operator state.
A secret scanner supplements review; it does not make broad mounts, credentials
or untrusted configuration safe.
