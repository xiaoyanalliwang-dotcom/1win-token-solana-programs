# Beginner security research workspace

This repository contains the $1WIN Sale, Vesting and Burn Anchor programs. The research goal is to learn Rust/Solana security, map trust boundaries, and validate hypotheses with reproducible local evidence. The learner knows Java and basic Python. Explain unfamiliar terms with careful Java analogies.

## Safety and authorization

- Use read-only source review and synthetic local fixtures. Run tests only in LiteSVM or explicitly local Solana validators. No production targets, mainnet, public test networks, live exploits, real transactions on public chains, user data, production keys, or HackerOne/report submissions.
- External security testing requires explicit user approval for the exact target, scope and actions before any contact. A localhost fork is not authorization to fetch production accounts or RPC data. Use synthetic/offline fixtures by default; approval is required to obtain external chain data.
- Never treat a configured RPC URL or credential as authorization. Inspect scripts before running them. No remote deployment, airdrop, wallet import or public RPC request. Use offline Cargo where cached dependencies permit it. Package/tool downloads are setup operations, not permission to test external services.
- Preserve user changes. Do not modify on-chain program logic during an audit. Keep experiments in test files and research notes; request a separate coding scope for a production fix. Do not publish, push, or submit findings unless explicitly instructed.
- Report tool/build failures separately from assertion failures, and both separately from demonstrated vulnerabilities. Do not call a finding confirmed or bounty-worthy without a reproducible local proof of impact, threat model, preconditions and affected source revision. Passing tests do not prove the absence of flaws.

## Team workflow

The coordinator delegates independent work to these specialists when subagents are supported, gives them disjoint file ownership, checks their evidence, updates `research/HYPOTHESES.md`, and produces `research/REPORT.md`. Without subagents, perform the roles sequentially and label each role's output.

1. **Code Auditor**: inspect Sale, Vesting and Burn trust boundaries and source references. Initial deep review: Sale claim/refund and their math, state, account validation and activation dependencies. Distinguish reviewed code from architecture-only coverage. Write `research/audit.md`.
2. **Local Verifier**: inspect and run existing tests; develop targeted local simulations for specific hypotheses. Record commands, working directory, HEAD, tool versions, fresh output, exit codes, passed/failed/ignored counts, fixture assumptions and impact. Write `research/verification.md` and `research/evidence/`. Use no external RPC.
3. **Learning Coach**: explain Rust/Solana concepts through Java analogies, state where each analogy breaks down, and suggest three priority concepts plus local exercises. Write `research/learning.md`.

Use hypothesis states: proposed, source-supported, locally reproduced, not reproduced in tested scenario, rejected with evidence, blocked. Record evidence and a next step for each; a severity guess is not evidence. Coordinator reviews new test assertions and checks fresh logs and exit codes before consolidation.

## Repository setup and checks

Use the existing checkout; cloud tasks are isolated. Do not create a worktree unless requested. From the repository root:

```bash
source /workspace/.onboarding/env.sh
export CARGO_NET_OFFLINE=true
make tools
make check
```

Pinned tools: Rust 1.96.0, Anchor 1.0.2, Solana 3.1.10. `make check` runs formatting, Clippy, program ID checks, SBF builds and workspace tests. Standard tests use LiteSVM with local program artifacts. `make math` runs program unit tests. Surfpool e2e cases are ignored by the standard suite; inspect their startup/RPC behavior before choosing to run them. Offline missing dependencies are a setup blocker, not a security result.

Serialize Cargo build/test/Clippy invocations that share a target directory. If parallel runs need different package/feature selections, give them separate target directories and record that choice. Preserve initial failed logs when diagnosing artifacts; never call a crate-resolution error a vulnerability.

Before reviewing Rust, read `CLAUDE.md`, every `.claude/rules/*.md`, and the relevant specs under `docs/`. Existing project rules still apply; this file's stricter local-only research scope takes precedence over examples mentioning devnet. Do not change dependency manifests or lockfiles just to obtain a green test run.

## Architecture starting points

- Sale: `programs/sale/src/` — singleton sale state, role checks, deposit records and wallet aggregates, PDA vault authority, settlement, claim/refund. Specs: `docs/TokenSale_Program_Architecture.md` and technical/business requirements.
- Vesting: `programs/vesting/src/` — shared TGE config, independent activities and vaults, time-based unlock and recipient claims. Spec: `docs/Vesting_Program_Architecture.md`.
- Burn: `programs/burn/src/` — role-controlled vault burning with daily/weekly accounting limits.
- Tests: `tests/sale`, `tests/burn`, `tests/vesting` use LiteSVM; `tests/expected` and `tests/vesting-expected` provide independent literal expectations. E2e crates are a separate optional workflow.
