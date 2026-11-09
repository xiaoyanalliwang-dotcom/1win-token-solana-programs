//! Local security regression: a failed second payout must not leave a partial claim.
//! Uses real Sale/SPL instructions in LiteSVM, synthetic wallets and synthetic mints.

use anchor_lang::error::ErrorCode;
use sale_test_common::{
    activate_claim_ix, claim_ix, custom_error_code, init_aggregate_ix, initialize_ix, purchase_ix,
    refund_vault_pda, token_vault_pda, TestEnv,
};
use solana_signer::Signer;
use spl_token_interface::error::TokenError;

#[test]
fn frozen_refund_destination_rolls_back_tokens_then_thaw_allows_one_claim() {
    let mut env = TestEnv::with_usdc_freeze_authority();
    let (usdc, win) = (env.usdc_mint, env.win_mint);
    let owner = env.actors.owner.insecure_clone();
    let admin = env.actors.admin.insecure_clone();
    let a = env.actors.users[0].insecure_clone();
    let b = env.actors.users[1].insecure_clone();
    let mut p = env.valid_init_params();
    // Independently derived: 1,000 tokens at $0.03 gives a $30 cap.
    // Two $20 buyers each receive 500 tokens and a $5 USDC refund.
    p.emission = 100_000_000_000;
    p.price_usdc = 30_000;
    env.send_ixs(
        &[initialize_ix(owner.pubkey(), usdc, win, &p)],
        &owner.pubkey(),
        &[&owner],
    )
    .expect("initialize synthetic sale");
    env.warp_to(p.sale_start_date);
    let a_usdc = env.create_ata(&a.pubkey(), &usdc);
    let b_usdc = env.create_ata(&b.pubkey(), &usdc);
    for (buyer, ata) in [(&a, a_usdc), (&b, b_usdc)] {
        env.mint_to(&usdc, &ata, 20_000_000);
        env.send_ixs(
            &[
                init_aggregate_ix(buyer.pubkey()),
                purchase_ix(buyer.pubkey(), ata, 0, 20_000_000),
            ],
            &buyer.pubkey(),
            &[buyer],
        )
        .expect("synthetic purchase");
    }
    env.mint_to(&win, &token_vault_pda(), p.emission);
    env.warp_to(p.sale_end_date);
    env.send_ixs(
        &[activate_claim_ix(
            admin.pubkey(),
            usdc,
            Some(refund_vault_pda()),
        )],
        &admin.pubkey(),
        &[&admin],
    )
    .expect("activate synthetic oversubscribed sale");
    env.warp_to(p.claim_start_date);
    // Precreate the token ATA so unchanged balance proves rollback independently
    // of account-creation rollback. Freeze only the *second* payout destination.
    let a_tokens = env.create_ata(&a.pubkey(), &win);
    env.freeze_account(&a_usdc, &usdc);
    let token_before = env.token_balance(&token_vault_pda());
    let refund_before = env.token_balance(&refund_vault_pda());
    let claim = claim_ix(a.pubkey(), usdc, win, true);
    let failed = env.send_ixs(std::slice::from_ref(&claim), &a.pubkey(), &[&a]);
    assert_eq!(
        custom_error_code(&failed),
        Some(TokenError::AccountFrozen as u32)
    );
    // The first CPI must actually have succeeded before the second one failed.
    let logs = &failed
        .as_ref()
        .expect_err("frozen destination rejects claim")
        .meta
        .logs;
    let success_line = format!("Program {} success", spl_token_interface::id());
    let transfers: Vec<_> = logs
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (line == "Program log: Instruction: Transfer").then_some(index))
        .collect();
    assert_eq!(
        transfers.len(),
        2,
        "both payout CPIs were attempted: {logs:?}"
    );
    let first_success = logs
        .iter()
        .position(|line| line == &success_line)
        .expect("first SPL transfer succeeded");
    assert!(transfers[0] < first_success && first_success < transfers[1]);
    let frozen_failure = format!(
        "Program {} failed: custom program error: 0x11",
        spl_token_interface::id()
    );
    assert!(logs
        .iter()
        .enumerate()
        .any(|(index, line)| index > transfers[1] && line == &frozen_failure));
    println!(
        "Frozen USDC ATA: expected SPL AccountFrozen (17), observed {:?}",
        custom_error_code(&failed)
    );
    for line in logs {
        println!("failed-claim: {line}");
    }
    assert_eq!(env.token_balance(&a_tokens), 0);
    assert_eq!(env.token_balance(&a_usdc), 0);
    assert_eq!(env.token_balance(&token_vault_pda()), token_before);
    assert_eq!(env.token_balance(&refund_vault_pda()), refund_before);
    assert!(env.aggregate_exists(&a.pubkey()));
    println!("Rollback verified: token payout=0, USDC payout=0, both vault balances unchanged, aggregate retained");

    let freeze_authority = env.payer.insecure_clone();
    let thaw = spl_token_interface::instruction::thaw_account(
        &spl_token_interface::id(),
        &a_usdc,
        &usdc,
        &freeze_authority.pubkey(),
        &[],
    )
    .expect("thaw instruction");
    env.send_ixs(&[thaw], &freeze_authority.pubkey(), &[&freeze_authority])
        .expect("issuer thaws synthetic account");
    env.expire_blockhash();
    env.send_ixs(std::slice::from_ref(&claim), &a.pubkey(), &[&a])
        .expect("claim succeeds after thaw");
    assert_eq!(env.token_balance(&a_tokens), 50_000_000_000);
    assert_eq!(env.token_balance(&a_usdc), 5_000_000);
    assert_eq!(
        env.token_balance(&token_vault_pda()),
        token_before - 50_000_000_000
    );
    assert_eq!(
        env.token_balance(&refund_vault_pda()),
        refund_before - 5_000_000
    );
    assert!(!env.aggregate_exists(&a.pubkey()));
    println!("Thaw/retry verified: 500 tokens and 5 USDC paid; aggregate closed");
    env.expire_blockhash();
    let replay = env.send_ixs(&[claim], &a.pubkey(), &[&a]);
    assert_eq!(
        custom_error_code(&replay),
        Some(ErrorCode::AccountNotInitialized as u32)
    );
    assert_eq!(env.token_balance(&a_tokens), 50_000_000_000);
    assert_eq!(env.token_balance(&a_usdc), 5_000_000);
    println!("Replay rejected with AccountNotInitialized (3012); payouts unchanged");
}
