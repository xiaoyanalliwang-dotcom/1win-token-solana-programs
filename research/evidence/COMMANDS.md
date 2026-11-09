# Reproduction commands

Working directory for every command: `/workspace/1win-token-solana-programs`.

```bash
source /workspace/.onboarding/env.sh
export CARGO_NET_OFFLINE=true
```

Run Cargo/build commands **serially**, not concurrently against this target directory.

Baseline (before the research test existed):

```bash
make check > research/evidence/make-check.log 2>&1
result=$?
printf '%s\n' "$result" > research/evidence/make-check.exit
```

New-test final output:

```bash
cargo test -p sale-test-common --test research_claim_atomicity --locked -- --nocapture \
  > research/evidence/claim-atomicity-final.log 2>&1
result=$?
printf '%s\n' "$result" > research/evidence/claim-atomicity-final.exit
cargo clippy -p sale-test-common --test research_claim_atomicity --locked -- -D warnings \
  > research/evidence/claim-atomicity-clippy.log 2>&1
result=$?
printf '%s\n' "$result" > research/evidence/claim-atomicity-clippy.exit
rustfmt --edition 2021 --check tests/sale/tests/research_claim_atomicity.rs \
  > research/evidence/claim-atomicity-fmt.log 2>&1
result=$?
printf '%s\n' "$result" > research/evidence/claim-atomicity-fmt.exit
```

Recovery diagnostic and final complete gate:

```bash
cargo test --workspace --doc --locked > research/evidence/workspace-doctest-serial.log 2>&1
result=$?
printf '%s\n' "$result" > research/evidence/workspace-doctest-serial.exit
make check > research/evidence/make-check-final.log 2>&1
result=$?
printf '%s\n' "$result" > research/evidence/make-check-final.exit
```

These commands redirect output directly, preserving the command's actual exit status rather than a pipeline's final utility status. Do not overwrite existing evidence if retaining an earlier run matters; use fresh filenames for later runs. No remote RPC or validator commands are part of reproduction.
