# Initial local security research report

Reviewed source revision: `68c301dfb8a2d4d0522bcd5a1d471a7905055fdc` plus the new research test listed below. This is an initial, bounded investigation, not a security certification or bounty claim.

## Project and team

[AGENTS.md](../AGENTS.md) defines the local-only safety boundary, the coordinator's evidence review workflow, and three specialist roles. The coordinator delegated independent source review to the Code Auditor, local execution to the Local Verifier, and Java-based explanations to the Learning Coach. Specialist output was checked against source and the new test assertions; the auditor corrected a participant-count description during review.

- [Code audit and architecture map](audit.md): deep review of Sale claim/refund and supporting state, phase, math, activation and vault handling; bounded Vesting claim and Burn authorization/limit spot checks.
- [Local verification](verification.md): commands, exit statuses, fixture assumptions, source hash and fresh output.
- [Hypothesis ledger](HYPOTHESES.md): questions, disposition, evidence and next action.
- [Learning guide](learning.md): Java analogies and local exercises.

## Architecture and initial review

Sale accepts synthetic USDC deposits, records each deposit and a per-wallet aggregate, and allows the Operator to refund a deposit before the Admin fixes the pool. Activation separates proceeds from the refund reserve. During Claim, the wallet receives its token allocation and any USDC refund, then its aggregate closes to prevent replay. Wallet, vault, mint and phase constraints are part of the authorization boundary, not merely data validation.

Vesting has a shared TGE configuration and independent activity vaults. A recipient or Operator may trigger a claim, but the payout goes to the activity's recorded recipient. Burn has a separate PDA vault and counters; configured Operator authorization and both daily/weekly limits gate its SPL burn. These are separate programs and authority namespaces, not a single shared vault. See audit.md for exact source references and coverage gaps.

## Test results

The standard workflow uses cached tools with Cargo offline and executes unit/property/LiteSVM tests, not remote RPC or a public validator. Optional Surfpool e2e cases are ignored in that workflow.

| Run | Outcome | Evidence |
|---|---|---|
| Final serial offline `make check` | **624 passed, 0 failed, 4 ignored; exit 0**. Includes the added regression, formatting, Clippy, ID checks, all SBF builds and doc-test build steps | `evidence/make-check-final.log`, `make-check-final.exit` |
| Initial offline `make check` | 623 tests passed, 0 assertion failures, 4 ignored; overall exit 2 due to rustdoc E0463 missing `sale_test_common` | `evidence/make-check.log`, `make-check.exit` |
| Serial workspace doctests after artifact rebuild | Exit 0; this is build-readiness evidence, not additional behavioral coverage (no runnable doctests) | `evidence/workspace-doctest-serial.log`, adjacent exit file |
| Added regression on final source | 1 passed, 0 failed, exit 0; targeted Clippy with warnings denied and formatting also exit 0 | `evidence/claim-atomicity-final.log`, Clippy/format logs and exit files |

The initial rustdoc failure followed package-targeted Cargo work sharing the target directory. Recovery after serial workspace rebuilding is consistent with artifact/feature interference, but the exact Cargo mechanism is not proven. It is recorded as a transient build failure, not a vulnerability. Verification.md records final full-gate results separately.

The added [claim atomicity regression](../tests/sale/tests/research_claim_atomicity.rs) uses real Sale and SPL instructions inside LiteSVM with synthetic keys/mints. Two synthetic $20 buyers oversubscribe a $30 cap. It freezes one buyer's USDC destination and verifies:

1. The first token Transfer succeeds during execution; the second Transfer fails with SPL `AccountFrozen` (17).
2. Buyer token/USDC balances and both vault balances remain unchanged after rollback; the aggregate remains usable.
3. After an authorized synthetic thaw, retry pays exactly 500 tokens and 5 USDC and closes the aggregate.
4. Replay fails with `AccountNotInitialized` (3012), with no second payout.

The token ATA is deliberately precreated to check balance rollback directly. Existing missing-refund-account tests separately cover ATA-creation rollback. This test validates error-handling behavior; the deliberately rejected claim is not a failing test or an exploit.

## Potential findings and disposition

**No confirmed unauthorized theft, overpayment, replay or access-control bypass was demonstrated. No bounty-worthiness claim is made.**

| Observation | Evidence | Interpretation |
|---|---|---|
| Earlier token payout might survive failure of the USDC leg | `claim.rs:112`, `:134`; new test and evidence/claim-atomicity-final.log | Not reproduced: full rollback, recovery and replay rejection observed in the tested scenario. |
| Foreign claim/refund substitution or replay | `claim.rs:29`, `refund.rs:26`, `:39`, `:58`; existing adversarial tests | Source shows constraints; local negative cases exercise the protections. Not an exhaustive transaction-composition proof. |
| Refund reserve could be exhausted by rounding | `math.rs:59`, `activate_claim.rs:107`; existing dust, solvency and lifecycle tests | No shortfall observed in tested cases; a formal proof over all valid parameters was not performed. |
| Issuer freezing can block the combined claim | Frozen synthetic USDC ATA reproduces expected transfer rejection | Privileged issuer/liveness assumption. Thaw before deadline restores claim; no unauthorized freeze or permanent loss established. |
| Vesting Admin can change future recipients | `programs/vesting/src/instructions/recipient.rs:18`, `:41` | Documented privileged capability; confirm deployment trust policy. Not an unauthenticated bypass. |
| Empty initialized aggregates increase rounding reserve | `purchase.rs:66`, `activate_claim.rs:111` | Documented conservative overestimate: one minimal USDC unit per initialized aggregate. Economic impact not reproduced; quantify costs before considering a defect. |

Sale path references in this table are relative to `programs/sale/src/instructions/`, except `math.rs`, which is under `programs/sale/src/`.

## Limits and next steps

Only synthetic local state was used. No production targets, public-network transactions, live exploits, user data, external security tests or report submissions were used. External security testing requires the user's explicit approval of the target and scope. No on-chain program source, dependency declarations or lockfiles were changed.

Next local work:

1. Extend transaction-composition and phase/deadline cases, including failure/retry near the deadline; keep exact before/after state assertions.
2. Quantify rounding reserve overestimation with synthetic participants, independent arithmetic expectations and the full terminal sweep lifecycle.
3. Broaden Vesting schedule/recipient lifecycle and Burn rollover/role-rotation review. Verify specifications and threat assumptions before labeling intended privileged actions as bugs.

The full baseline includes some special program-wallet fixtures with signature verification disabled; those emulate signer metadata and do not prove actual multisig CPI integration. The new frozen-ATA regression keeps signature verification enabled. Local execution does not establish live deployment configuration, operational safety, exhaustive correctness or absence of vulnerabilities.

## Three concepts to learn

1. **Accounts, PDAs and authorization:** like records supplied to a Java service, but each record's identity, ownership and permissions must be checked. A PDA is a deterministic address with program-controlled signing privileges, not merely a map key.
2. **Rust borrowing and Result/Option:** compare exclusive `&mut` access with Java mutable aliases, and explicit error/absence types with checked results and Optional. Borrow safety does not prove financial correctness.
3. **CPI atomicity and phase machines:** compare a CPI with calling an accounting component inside one database transaction. A later error rolls state back; the phase and clock determine when the operation is allowed. Transaction fees and observable logs are distinct from committed program state.
