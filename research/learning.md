# Learning Coach: reading Sale claim and refund as a Java programmer

This is a reading guide, not a finding or a claim that the programs are secure. Exercises use synthetic local accounts and LiteSVM only. Never connect these exercises to production, mainnet, devnet, testnet, user wallets, or real funds.

## 1. Accounts, identity, and authorization

Think of a Solana program as a Java service whose durable state is stored in records outside the service. An instruction is a method call, but its caller supplies the records the method will use. A parameter named `vault` is not automatically the correct vault, just as a request containing `bankAccountId` does not prove that the requester owns it.

Anchor's `#[derive(Accounts)]` generates validation code around the handler. It resembles a combination of Java request validation, deserialization, and authorization middleware. In `programs/sale/src/instructions/claim.rs:21`, seeds identify the sale state. At line 27, `Signer` requires signer privilege. Lines 29–36 bind the aggregate to the signing wallet using seeds and `has_one`. Line 38 binds the token vault to the address stored in sale state. Read these declarations before reading the handler.

A PDA (program-derived address) is like a deterministic record key built from a namespace and inputs: `aggregate + wallet` identifies one wallet's contribution record. Unlike a Java map key, a PDA also has cryptographic and runtime authorization properties: there is no ordinary private key for that address, and its program can authorize actions with matching seeds. The bump is part of deriving a valid off-curve address; it is not a password. A token account's authority is also different from the Solana program that owns its account data.

`UncheckedAccount` does not mean automatically exploitable. It means Anchor does not deserialize it into a particular account type. Refund uses the wallet only as an address (`programs/sale/src/instructions/refund.rs:33`); the deposit's seeds and `has_one` bind that address at lines 39–46. The operator, not that wallet, must sign (lines 22–31).

**Analogy limit:** `Signer` is not equivalent to an authenticated human user or a Java session. Program-derived signers can receive signer privilege through a CPI. Likewise, a correctly typed record alone does not establish business authorization.

**Sale-specific policy:** although `Signer` generally accepts a PDA signer authorized through a CPI, Sale entry additionally checks that the buyer address is on the Ed25519 curve (`programs/sale/src/instructions/purchase.rs:27` and `purchase.rs:94`). A PDA is off-curve and is rejected by these constraints. Do not confuse a *program-owned account* with a *program-derived address*: an ordinary on-curve key can have its empty account reassigned to another data-owning program and still have its original private key. The regression tests describe exactly that reassignment in `tests/sale/tests/program_owned_wallet.rs:1`. Their program-owned wallet is not evidence that Sale permits PDA buyers.

**Local exercise:** inspect the foreign-signer claim test at `tests/sale/tests/claim.rs:598` and the foreign-vault refund test at `tests/sale/tests/refund.rs:656`. Predict which declaration rejects the substitute before checking the test's expected error. Use the verifier's recorded results to distinguish reading from executed evidence.

## 2. Rust borrowing, Option, Result, and checked integers

`let aggregate = &mut ctx.accounts.aggregate` (`programs/sale/src/instructions/refund.rs:101`) temporarily borrows the aggregate mutably. In Java you can keep many aliases to a mutable object and rely on discipline or locks; Rust restricts overlapping access statically. `&T` is a shared borrow and `&mut T` is an exclusive mutable borrow. These rules reduce memory errors; they do not prove the refund's business logic is correct.

`Option<T>` explicitly represents either `Some(value)` or `None`, roughly like `Optional<T>`. The optional refund accounts in claim (`claim.rs:59` and `claim.rs:68`) are required only when the calculated refund is positive. At lines 123–133, `as_ref()` borrows the optional value, `ok_or(...)` converts absence into an error, and `?` returns that error to the caller. This is closer to an explicit checked-result flow than a Java null-pointer exception.

`Result<T, E>` represents `Ok(T)` or `Err(E)`. The handler's `Result<()>` means success has no payload (`()` resembles a unit/void value). `?` propagates an error; it does not mean that execution continues or substitutes zero.

Money is stored in integer base units, not floating-point dollars. Refund's `checked_sub` returns an `Option`, rejecting underflow (`refund.rs:102` and `refund.rs:109`). Math widens inputs to `u128`, divides, then narrows with `u64::try_from` (`programs/sale/src/math.rs:37`). This resembles using a wider exact integer plus `Math.*Exact` and explicit range checks in Java, rather than relying on ordinary Java `long` wraparound. Rust `u64` is unsigned, unlike Java `long`. `u128` is fixed width, unlike Java `BigInteger`.

**Analogy limit:** Rust does not automatically make arithmetic safe in every context. Checked arithmetic and deliberate rounding are explicit choices. Allocation rounds tokens down and the spent amount up (`math.rs:73`), so examples must be computed in base units and independently of the implementation.

**Local exercise:** trace an oversubscribed contribution through `allocate` and calculate a tiny example on paper. Then inspect the existing expected-value fixtures and claim balance assertions. Do not write a test that merely repeats `allocate`'s formula on both sides of an equality.

## 3. Transactions, cross-program calls, and state machines

A CPI (cross-program invocation) is a program calling another program, such as Sale asking the SPL Token program to transfer funds. Think of a Java service calling a separate accounting component inside one runtime transaction. It is not an arbitrary HTTP request: the Solana runtime enforces account privileges and instruction atomicity.

Claim calculates the allocation before its transfers (`programs/sale/src/instructions/claim.rs:97`), transfers tokens at line 112, and then processes the refund at lines 123–141. If the instruction fails at the refund step, earlier token and account-data changes do not remain committed. Across a transaction, failed execution rolls back state changes, though transaction fees can still be charged. Local tests should inspect before/after balances and account existence, not just the returned error.

`close = wallet` on the aggregate (`claim.rs:31`) tells Anchor to close that record on successful exit; the missing record prevents a second claim (lines 144–146). This is similar to consuming a one-time redemption record. Refund instead closes an individual deposit into the refund account (`refund.rs:41`) and decrements both the wallet net and global contributions before the transfer (lines 101–122).

The time gate is another authorization dimension. Claim accepts only `Phase::Claim` and distinguishes an expired deadline (`claim.rs:87`); operator refund requires an unfixed pool and `Sale` or `SaleEnded` (`refund.rs:83`). In Java terms, this is a state machine plus an injected clock. The repository's architecture describes how phases derive from time and flags, rather than trusting a caller-provided timestamp (`docs/TokenSale_Program_Architecture.md`, section 4).

**Analogy limit:** an Anchor `Account<T>` is a deserialized snapshot, not a live ORM object that refreshes after another component changes it. After a CPI, code needing updated account fields must reload them. A transaction that returns an error is usually evidence of a working guard, not automatically a vulnerability.

**Local exercise:** read the claim replay test at `tests/sale/tests/claim.rs:279`, the missing-refund-account test at line 520, and the refund-after-finalization test at `tests/sale/tests/refund.rs:440`. For each, record expected rejection, pre/post token balances, account existence, and the business invariant. Compare against actual local verifier output before calling the behavior confirmed.

## What to learn next

1. Anchor account constraints, PDA identity, and the distinction between signer privilege, account ownership, and token authority.
2. Rust borrows plus `Option`/`Result` error propagation, followed by exact integer arithmetic and rounding.
3. Solana transaction atomicity, CPI privileges, and phase-based state transitions.

A useful learning milestone is explaining why a rejected transaction protects funds, while also explaining exactly what evidence would show an unintended successful transfer. None of the analogies or existing passing tests alone proves absence of vulnerabilities or bounty eligibility.
