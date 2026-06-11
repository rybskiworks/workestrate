# ai-workbench

A local AI workbench that runs Pi and Odysseus coding agents inside
Microsandbox microVMs, with LiteLLM as the unified LLM proxy.

## Current Milestone (M1)

Compile-checked Rust + Nix + Microsandbox SDK + sandbox plans.
Runtime execution is blocked: this environment lacks /dev/kvm.

## Architecture

```
┌─────────────────────────────────────────────┐
│              Host (Nix + KVM)               │
│  ┌─────────────┐  ┌─────────────────────┐   │
│  │  LiteLLM    │  │   agentctl (Rust)   │   │
│  │  proxy      │  │   plan / check      │   │
│  │  :4000      │  │                     │   │
│  └──────┬──────┘  └─────────────────────┘   │
│         │                                    │
│  ┌──────┴──────┐  ┌─────────────────────┐   │
│  │  Pi agent   │  │  Odysseus agent     │   │
│  │  microVM    │  │  microVM            │   │
│  │  (future)   │  │  (future)           │   │
│  └─────────────┘  └─────────────────────┘   │
└─────────────────────────────────────────────┘
```

## Quick Start

Enter the dev shell:
```bash
nix develop
```

Run checks:
```bash
just check
```

Run agentctl plans:
```bash
just agentctl check
just agentctl litellm plan
just agentctl agent plan pi
just agentctl agent plan odysseus
```

## Directory Layout

```
.
├── control/agentctl/     Rust CLI (plans, checks)
├── infra/
│   ├── litellm/          LiteLLM proxy config
│   └── microsandbox/     SDK notes, runtime docs
├── profiles/
│   └── agents/           Agent runtime profiles
├── agents/               Agent repos (optional, gitignored)
├── nix/                  Nix expressions (dev shell, packages)
├── docs/                 ADRs, known gaps
└── var/                  Runtime state (logs, pidfiles)
```

## Future Scope (Intentionally Deferred)

- Runtime Microsandbox execution (requires KVM host)
- Postgres-backed LiteLLM (virtual keys, spend tracking)
- Pi/Odysseus full integration (OPENAI_BASE_URL config)
- Production deployment profiles
- CI/CD pipeline

## Honest Note

Runtime testing of Microsandbox microVMs is blocked in this container:
`/dev/kvm` is absent and nested virtualization is unavailable.
All SDK integration is compile-checked only. A host with KVM or
Apple Silicon is required for actual sandbox execution.
