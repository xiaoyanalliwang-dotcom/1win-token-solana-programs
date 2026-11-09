# Initial Code Auditor review

## Scope and evidence limits

This is a read-only source review of Sale `claim`, `refund`, activation, allocation math, account construction and phase dependencies. Vesting and Burn receive architecture mapping and bounded read-only spot checks of their payout/burn authorization; this is not a full audit of those programs. No RPC, external chain data, production keys, public transactions or external security targets were used. No program source was modified. Execution evidence belongs in `verification.md`; test names below identify existing evidence candidates, not a claim that this agent ran them.

Repository guidance read: `AGENTS.md`, `CLAUDE.md`, `.claude/rules/*.md`, and relevant sections of the Sale architecture and technical requirements. The specification explicitly treats issuer/mint authorities and privileged roles as trust boundaries. A security hypothesis is not a confirmed vulnerability or bounty claim.

## Architecture map

| Component | State and authority | Money path and security boundary | Source starting points |
| --- | --- | --- | --- |
| Sale | Singleton `SaleState`, per-wallet aggregate, per-purchase deposit; Owner/Admin/Operator powers are separate | Purchase adds USDC to deposit vault; Operator refund returns one deposit before settlement; Admin freezes the pool and splits proceeds/refunds; wallet claim receives tokens and any excess USDC | `programs/sale/src/instructions/purchase.rs:16`, `refund.rs:21`, `activate_claim.rs:19`, `claim.rs:20` |
| Vesting | Shared TGE config plus separate activity records and vaults | Recipient or configured Operator triggers an unlocked-amount claim; the destination remains the recorded recipient's ATA, irrespective of who triggers it | `programs/vesting/src/instructions/claim.rs:18`, `:23`, `:37`, `:65` |
| Burn | Burn state with Operator and limits; separate daily/weekly counters; PDA-owned vault | Operator burns from the canonical vault; both time-window limits must pass; SPL Token performs the actual supply reduction | `programs/burn/src/instructions/burn.rs:16`, `:28`, `:50`, `:66`, `:98` |

Programs use separate program IDs/PDA namespaces; sharing a token mint does not itself share vault authority. Sale initialization creates its deposit/token vaults with the appropriate mint and PDA authority (`programs/sale/src/instructions/initialize.rs:39`, `:49`) and stores the authority bump (`:131`). The entry requires the program's upgrade authority (`:22`). Privileged owners/upgraders and mint freeze authorities remain meaningful trust assumptions, not capabilities an arbitrary buyer has.

Sale lifecycle:

```text
PreSale -> Sale -> SaleEnded -> activate_claim -> wait for claim start
                  |                              |
                  + refund permitted before fix  + Claim -> ClaimClosed
                                                     |        |
                                           close aggregate   sweep/cleanup
```

Actual phases derive from `Clock` and stored flags rather than a mutable phase enum (`programs/sale/src/phase.rs:27`). Refund also explicitly rejects `pool_fixed`, including the waiting interval after activation (`refund.rs:84`).

## Bounded Vesting and Burn spot checks

- **Vesting claim:** canonical config, activity and vault PDAs bind the record and custody (`programs/vesting/src/instructions/claim.rs:18`, `:23`, `:30`); vault mint/authority are checked (`:32`). Recipient address comes from `vesting.recipient` (`:37`) and its ATA is constrained to that recipient (`:40`). Signer must be recipient or configured Operator (`:70`); being Operator does not permit changing the payout destination. TGE must have begun (`:77`); owed amount is vested minus already claimed, checked for underflow and positive value (`:80`); record advances only after transfer (`:107`). No obvious arbitrary-recipient or caller-controlled-amount bypass was found in this spot check.
- **Vesting trust boundary:** Admin can reassign the recipient at any time (`programs/vesting/src/instructions/recipient.rs:18`, `:41`–`:46`). This is explicit in that instruction's documentation, so a malicious authorized Admin is not equivalent to an unauthenticated attacker discovering a bypass. Reviewers should confirm the policy is acceptable. Gaps: no exhaustive schedule, create/close, role rotation or recipient-lifecycle audit in this pass.
- **Burn execution:** canonical state binds configured Operator (`programs/burn/src/instructions/burn.rs:16`–`:26`); counters/vault/authority have canonical seeds (`:28`–`:42`); mint is state-bound (`:44`). Amount must be positive and time comes from Clock (`:50`–`:61`). Both daily and weekly totals are checked before mutation (`:66`–`:92`). `period::accumulate` uses checked addition and rejects values above the period limit (`programs/burn/src/period.rs:28`–`:44`). SPL burn is fixed to the Token Program (`burn.rs:47`, `:98`). Vault mint and authority are fixed at initialization (`programs/burn/src/instructions/initialize.rs:46`–`:54`). No obvious unauthenticated burn or simple limit bypass was found. Gaps: no exhaustive epoch/time-assumption, role rotation, upgrade or operational policy review.

## Sale claim/refund protections checked in source

1. **Claim identity and payout routing:** signer wallet; canonical aggregate seeds include that wallet; `has_one = wallet`; aggregate closes to wallet (`claim.rs:24`–`:36`). Token destination is the wallet's ATA (`:41`); mints are bound to state (`:75`). The refund vault is canonical and constrained by mint/authority (`:52`). This structurally restricts substitution rather than trusting client labels.
2. **Claim timing and replay:** phase must be Claim (`claim.rs:86`–`:95`), net must be positive (`:97`). Successful claim closes the aggregate (`:31`); no subsequent purchase can recreate a claim entitlement during Claim because aggregate initialization requires Sale (`purchase.rs:54`). Existing tests include `claim_twice_rejected_account_not_initialized` (`tests/sale/tests/claim.rs:279`) and exact time boundaries (`:352`, `:404`, `:433`).
3. **Refund identity and routing:** state `has_one = operator` plus a signer (`refund.rs:22`–`:31`); deposit binds wallet and index (`:38`), aggregate is canonical for the same wallet (`:48`). Refund destination is a newly initialized PDA token account with wallet authority (`:58`–`:66`), so Operator cannot choose an arbitrary recipient or amount. No wallet signature is required for this compliance refund by design.
4. **Refund accounting and replay:** both aggregate net and global contribution count decrease by the deposit's stored amount using checked subtraction (`refund.rs:98`–`:112`); deposit closes into the refund account (`:41`). Keeping an emptied aggregate preserves its monotonic deposit index, avoiding a re-used deposit address. Existing negative tests cover replay (`tests/sale/tests/refund.rs:551`), non-Operator (`:619`), substituted vault (`:656`) and mint (`:695`).
5. **Frozen-pool solvency:** activation records the accounted contributions, not unsolicited token transfers (`activate_claim.rs:75`, `:125`). It requires funded vaults (`:98`), splits refunds physically away from proceeds (`:109`), and reserves one minimal USDC unit per initialized aggregate for rounding (`:111`). Claim uses `pool_fixed_amount` (`claim.rs:103`). `math.rs:37` uses checked wide multiplication/division; `:73` floors tokens and `:81` rounds spent upward.
6. **Atomicity:** claim sends tokens before checking whether both positive-refund accounts are present (`claim.rs:112`–`:141`). Correctness therefore depends on Solana transaction rollback: failure must undo the first CPI and account creation. This ordering is not itself a demonstrated partial-payment flaw. SPL Token is fixed by `Program<Token>` (`claim.rs:81`), rather than a caller-chosen CPI implementation.

## Hypotheses and disposition

| ID | Question / impact if real | Source evidence and existing checks | Current disposition / next local step |
| --- | --- | --- | --- |
| AUD-01 | Can a failed refund leg leave the earlier token payment committed, enabling repeat payouts? | First transfer at `claim.rs:112`; required refund-account errors at `:128`/`:133`; second transfer at `:134`. Missing-vault test already checks ATA creation and aggregate rollback (`tests/sale/tests/claim.rs:520`); missing-ATA test checks aggregate survives (`:556`). | Not established as a flaw. Verifier should force a real second-CPI failure with an existing frozen USDC ATA, assert unchanged token/refund balances and aggregate net, then thaw and successfully retry. This exercises rollback beyond a missing-account error. |
| AUD-02 | Could rounding exhaust the refund vault after the Owner withdraws proceeds? | Allocation `math.rs:59`–`:92`; headroom at `activate_claim.rs:107`–`:115`; price bounded below token units at `state/sale_state.rs:91`. Existing dust scenarios: `tests/sale/tests/dust_worstcase.rs:130`, `:241`, `:309`. | Source shows an explicit mitigation; no shortfall found by review. Re-run dust and lifecycle tests locally; passing tested vectors do not prove all integer combinations. |
| AUD-03 | Can Operator refund an unrelated deposit or redirect a refund? | Wallet/index seeds and `has_one` on deposit (`refund.rs:38`), matching aggregate seeds (`:48`), fixed destination authority (`:64`) and amount (`:98`). | Rejected as a direct arbitrary-recipient hypothesis by source structure; runtime adversarial tests remain important. Retain exact negative-test evidence from the verifier. |
| AUD-04 | Can a third party prevent claims by freezing the destination ATA? | Claim destination is a canonical ATA (`claim.rs:41`, `:62`); SPL transfers can reject frozen accounts. TRD explicitly gives the token freeze authority to Ops MS (`docs/1WIN_Programs_Technical_Requirements.md:89`). | Trust/availability assumption, not an unauthorized exploit. Only demonstrate using a synthetic fixture's freeze authority. A transaction failure must leave entitlement intact; recovery before the deadline requires authority cooperation. Do not describe an issuer exercising its documented authority as a new bounty-worthy bug. |
| AUD-05 | Can empty aggregate creation inflate the rounding reserve and delay Owner proceeds? | `purchase.rs:50`–`:76` increments participants once when `init_aggregate` creates a new aggregate, before any contribution; later purchases/refunds do not create additional increments merely because net is zero. Activation subtracts this monotonic count (`activate_claim.rs:111`). Architecture explicitly accepts overestimation (`docs/TokenSale_Program_Architecture.md:176`). | Source-supported policy/economic hypothesis; impact not locally reproduced and not a confirmed security defect: reservation grows by one minimal USDC unit per rent-funded aggregate, preserves claimant solvency, and remains in refund vault until terminal sweep. Quantify local costs and intended policy before considering further. |

## Initial outcome

No reproducible unauthorized theft, overpayment, bypass or permanently destroyed entitlement has been established by this review. The strongest targeted follow-up is AUD-01's second-CPI failure/recovery test. AUD-02 deserves continued boundary/property coverage. Broader Vesting/Burn, upgrade authority, terminal cleanup, transaction composition and token-authority policy review remain future work; bounded spot checks are not evidence that those surfaces are vulnerability-free.
