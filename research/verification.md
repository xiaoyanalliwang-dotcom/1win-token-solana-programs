# Local Verifier

## Scope and setup

All commands ran from `/workspace/1win-token-solana-programs`, using `source /workspace/.onboarding/env.sh` and `CARGO_NET_OFFLINE=true`. Source revision, UTC run start, versions and initial status are recorded in [baseline metadata](evidence/baseline-metadata.txt). Revision: `68c301dfb8a2d4d0522bcd5a1d471a7905055fdc` plus the explicitly recorded new research test. No on-chain source, manifests or lockfiles were changed.

Tools: Rust/Cargo 1.96.0, Anchor 1.0.2, Solana CLI/cargo-build-sbf 3.1.10, platform-tools v1.52 / SBF rustc 1.89.0. Cached dependencies and toolchains were used. No external security testing, remote RPC, public-chain transactions, production forks, user data or external reporting occurred. No local RPC validator was needed; these runs use the in-process LiteSVM simulator.

## Existing baseline

Command: `CARGO_NET_OFFLINE=true make check`.

The command checks formatting, Clippy with warnings denied, program IDs, all three SBF artifacts, and the workspace unit/integration/doc tests. Fresh initial evidence: [make-check.log](evidence/make-check.log) and [make-check.exit](evidence/make-check.exit). The baseline started before the new test was created; its counts must be reported separately from the added test.

Initial run: **623 tests passed, 0 assertion failures, 4 ignored**, but overall `make check` exited **2** because rustdoc failed with `E0463: can't find crate for sale_test_common` in `tests/burn/src/lib.rs:17`. This is a build/doc-test failure, not a demonstrated program vulnerability. Package-specific test/Clippy builds were run while the workspace suite was executing and share unhashed program-library artifacts in `target/debug/deps`; a feature/artifact collision is the likely explanation, not independently proven.

The diagnostic serial `cargo test --workspace --doc --locked` rebuilt workspace artifacts and passed with exit **0** ([output](evidence/workspace-doctest-serial.log), [exit](evidence/workspace-doctest-serial.exit)). These crates contain **0 doctests**: this validates documentation compilation, not runtime behavior. No caches or source code were deleted or changed to fix the failure.

A final `CARGO_NET_OFFLINE=true make check` completed **serially**, after all package-specific builds stopped, now including the added research test. Final gate evidence: [make-check-final.log](evidence/make-check-final.log) and [make-check-final.exit](evidence/make-check-final.exit). **Final outcome: exit 0; 624 tests passed, 0 failed, 4 ignored.** All three SBF builds, formatting, Clippy, ID checks, workspace unit/integration tests and documentation compilation passed in the serial final run. This includes the new atomicity test; its separate targeted pass is not an additional unique test. Future package/workspace Cargo runs sharing a target directory should be serialized.

## Additional test: second-CPI failure atomicity

Source: [research_claim_atomicity.rs](../tests/sale/tests/research_claim_atomicity.rs). Final SHA-256: [source hash](evidence/claim-atomicity-source.sha256).

Run:

```bash
source /workspace/.onboarding/env.sh
export CARGO_NET_OFFLINE=true
cargo test -p sale-test-common --test research_claim_atomicity --locked -- --nocapture
cargo clippy -p sale-test-common --test research_claim_atomicity --locked -- -D warnings
rustfmt --edition 2021 --check tests/sale/tests/research_claim_atomicity.rs
```

Final results: **1 test passed, 0 failed, 0 ignored**, exit 0; targeted Clippy exit 0; formatting exit 0. Evidence: [final test output](evidence/claim-atomicity-final.log), [Clippy](evidence/claim-atomicity-clippy.log), [formatting](evidence/claim-atomicity-fmt.log), and their adjacent `.exit` files.

Synthetic economics: 1,000 tokens at $0.03 gives a $30 cap. Two synthetic wallets contribute $20 each. The independently derived claim is 500 tokens plus $5 USDC for each wallet. No contract allocation function computes the expected values in this test.

The test uses real Sale and SPL instructions against local artifacts, freezes the first buyer's USDC ATA with the synthetic mint's issuer authority, then attempts the claim. Ordered runtime logs assert the first token `Transfer` succeeded inside the transaction, and the second USDC `Transfer` failed with SPL `AccountFrozen` (17 / `0x11`). Assertions verify buyer token/USDC balances remain zero, both vault balances are unchanged, and the buyer aggregate still exists. After a genuine local SPL thaw instruction, the retry pays exactly 500 tokens and $5, closes the aggregate, and a replay fails with Anchor `AccountNotInitialized` (3012) without increasing payouts.

The token ATA is deliberately precreated: observing its zero balance tests rollback independently of account-creation rollback. Existing `tests/sale/tests/claim.rs::claim_oversubscribed_without_refund_vault_rejected` separately checks that an ATA created during a failing claim does not survive the rollback.

**Interpretation:** the partial-payout hypothesis was **not reproduced in this tested scenario**. The frozen ATA causes the expected failed transaction, not a confirmed vulnerability. Recovery requires the synthetic issuer to thaw; this does not prove a real buyer can override an issuer freeze. It demonstrates transaction atomicity and entitlement retention on retry. It does not establish a bounty-worthy flaw.

An initial development run passed with an unused-import warning. That import was removed; the final targeted run and Clippy are clean. `claim-atomicity-initial.log` retains that initial output, and `claim-atomicity.log` retains the strengthened-assertion intermediate run; use `claim-atomicity-final.log` for the final source.

## Evidence limits and next steps

- Default harness paths verify actual signatures; this new test uses `with_usdc_freeze_authority`, which retains signature verification. Existing off-curve rejection tests intentionally use `without_sigverify` as an approximation of a PDA CPI signer; those are negative account-policy tests, not an end-to-end proof of an actual multisig CPI chain. Separately, the `program_owned_wallet` lifecycle cases use an ordinary on-curve key whose account data owner changes: program-owned is not the same as program-derived/off-curve. No inconsistency in buyer eligibility was established.
- Upgrade authority/ProgramData accounts are local fixtures; mints, wallet keys and balances are synthetic. The test controls USDC freeze authority deliberately; a normal unprivileged wallet would not have that authority.
- Simulator execution does not prove validator, deployment, network, fee-market, or production configuration behavior. Time is warped locally. Existing property tests sample inputs; they are not exhaustive proofs.
- Standard suite Surfpool e2e cases remain ignored and were not executed. No validator/fork was launched, and no production accounts were fetched.
- Passing checks establish the tested behavior only. Next useful work is a scoped local test for each remaining source-supported hypothesis, retaining exact preconditions and expected errors. Do not contact external targets without explicit approval.

## Final evidence summary

Machine-readable outcomes are in [summary.json](evidence/summary.json). The final test source SHA-256 is `9620931b7ec0cfa0ec6cc9dc23b379fb452e604f15ffba2d000bb620f858141c`. A final `git diff --name-only -- programs Cargo.toml Cargo.lock` was empty: no production program or dependency changes. New files are research documentation/evidence and the local regression test.
