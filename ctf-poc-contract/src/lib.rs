use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult,
    pubkey::Pubkey,
    program::invoke, msg,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    match spl_token::instruction::TokenInstruction::unpack(instruction_data).unwrap() {
        spl_token::instruction::TokenInstruction::Transfer { amount, .. } => {
            let _source = &accounts[0];
            let _destination = &accounts[1];
            let _authority = &accounts[2];
            msg!("SENDING AMOUNT ****: {}", amount);
            Ok(()) //@TODO simply return true, maybe tey invoke later

        }
        _ => {
            panic!("wrong ix")
        }
    }
}
