#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

// Program entrypoint
entrypoint!(process_instruction);

// Function to route instructions to the correct handler
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Unpack instruction data
    let instruction = CounterInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Match instruction type
    match instruction {
        CounterInstruction::InitializeCounter { initial_value } => {
            process_initialize_counter(program_id, accounts, initial_value)?
        }
        CounterInstruction::IncrementCounter => process_increment_counter(program_id, accounts)?,
        CounterInstruction::ConditionalCounter {
            should_increment,
            amount,
        } => process_my_struct(program_id, accounts, should_increment, amount)?,
    };
    Ok(())
}

// Instructions that our program can execute
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum CounterInstruction {
    InitializeCounter { initial_value: u64 },
    IncrementCounter,
    ConditionalCounter { should_increment: bool, amount: u64 },
}

fn process_my_struct(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    should_increment: bool,
    amount: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let counter_account = next_account_info(accounts_iter)?;

    // is this really necessary? is there a reason this isn't exposed at runtime as a static
    // variable or something? this seems very clunky for no reason
    if counter_account.owner != program_id {
        // it's crazy you have to lay out accounts like this- this seems like it should have been
        // part of the core solana library
        return Err(ProgramError::IncorrectProgramId);
    }

    // can this functionality below be composable? can I define a function above that i re-use across
    // multiple program instructions?? since this ultimately just increments
    // ALSO: which parts of the program are composable?
    if should_increment {
        let mut data = counter_account.data.borrow_mut();
        let mut counter_data: CounterAccount = CounterAccount::try_from_slice(&data)?;

        // Increment the counter value and re-serialize.
        counter_data.count = counter_data
            .count
            .checked_add(amount)
            .ok_or(ProgramError::InvalidAccountData)?;
        counter_data.serialize(&mut &mut data[..])?;
        msg!("Counter incremented to: {}", counter_data.count);
    }

    Ok(())
}

// Initialize a new counter account
fn process_initialize_counter(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    initial_value: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let counter_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    // Size of our counter account
    let account_space = 8; // u64 requires 8 bytes

    // Calculate minimum balance for rent exemption
    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(account_space);

    // Create the counter account
    invoke(
        &system_instruction::create_account(
            payer_account.key,    // Account paying for the new account
            counter_account.key,  // Account to be created
            required_lamports,    // Amount of lamports to transfer to the new account
            account_space as u64, // Size in bytes to allocate for the data field
            program_id,           // Set program owner to our program
        ),
        &[
            payer_account.clone(),
            counter_account.clone(),
            system_program.clone(),
        ],
    )?;

    // Create a new CounterAccount struct with the initial value
    let counter_data = CounterAccount {
        count: initial_value,
    };

    // Get a mutable reference to the counter account's data
    let mut account_data = &mut counter_account.data.borrow_mut()[..];

    // Serialize the CounterAccount struct into the account's data
    counter_data.serialize(&mut account_data)?;

    msg!("Counter initialized with value: {}", initial_value);

    Ok(())
}

// Update an existing counter's value
fn process_increment_counter(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let counter_account = next_account_info(accounts_iter)?;

    // Verify account ownership
    if counter_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Mutable borrow the account data
    let mut data = counter_account.data.borrow_mut();

    // Deserialize the account data into our CounterAccount struct
    let mut counter_data: CounterAccount = CounterAccount::try_from_slice(&data)?;

    // Increment the counter value
    counter_data.count = counter_data
        .count
        .checked_add(1)
        .ok_or(ProgramError::InvalidAccountData)?;

    // Serialize the updated counter data back into the account
    counter_data.serialize(&mut &mut data[..])?;

    msg!("Counter incremented to: {}", counter_data.count);
    Ok(())
}

// Struct representing our counter account's data
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CounterAccount {
    count: u64,
}

#[cfg(test)]
mod test {
    use super::*;
    use litesvm::LiteSVM;
    use solana_sdk::{
        account::ReadableAccount,
        instruction::{AccountMeta, Instruction},
        message::Message,
        signature::{Keypair, Signer},
        system_program,
        transaction::Transaction,
    };

    #[test]
    fn test_counter_program() {
        // Create a new LiteSVM instance
        let mut svm = LiteSVM::new();

        // Create a keypair for the transaction payer
        let payer = Keypair::new();

        // Airdrop some lamports to the payer
        svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

        // Load our program
        let program_keypair = Keypair::new();
        let program_id = program_keypair.pubkey();
        svm.add_program_from_file(program_id, "target/deploy/counter_program.so")
            .unwrap();

        // Create a new keypair to use as the address for our counter account
        let counter_keypair = Keypair::new();
        let initial_value: u64 = 42;

        // Step 1: Initialize the counter
        println!("Testing counter initialization...");

        // Use Borsh serialization for the instruction
        let init_instruction_data =
            borsh::to_vec(&CounterInstruction::InitializeCounter { initial_value })
                .expect("Failed to serialize instruction");

        let initialize_instruction = Instruction::new_with_bytes(
            program_id,
            &init_instruction_data,
            vec![
                AccountMeta::new(counter_keypair.pubkey(), true),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        // Create transaction
        let message = Message::new(&[initialize_instruction], Some(&payer.pubkey()));
        let transaction =
            Transaction::new(&[&payer, &counter_keypair], message, svm.latest_blockhash());

        // Send transaction
        let result = svm.send_transaction(transaction);
        assert!(result.is_ok(), "Initialize transaction should succeed");

        let logs = result.unwrap().logs;
        println!("Transaction logs:\n{:#?}", logs);

        // Check account data
        let account = svm
            .get_account(&counter_keypair.pubkey())
            .expect("Failed to get counter account");

        let counter: CounterAccount = CounterAccount::try_from_slice(account.data())
            .expect("Failed to deserialize counter data");
        assert_eq!(counter.count, 42);
        println!(
            "Counter initialized successfully with value: {}",
            counter.count
        );

        // Step 2: Increment the counter
        println!("Testing counter increment...");

        // Use Borsh serialization for increment instruction
        let increment_data = borsh::to_vec(&CounterInstruction::IncrementCounter)
            .expect("Failed to serialize instruction");

        let increment_instruction = Instruction::new_with_bytes(
            program_id,
            &increment_data,
            vec![AccountMeta::new(counter_keypair.pubkey(), true)],
        );

        // Create transaction
        let message = Message::new(&[increment_instruction], Some(&payer.pubkey()));
        let transaction =
            Transaction::new(&[&payer, &counter_keypair], message, svm.latest_blockhash());

        // Send transaction
        let result = svm.send_transaction(transaction);
        assert!(result.is_ok(), "Increment transaction should succeed");

        let logs = result.unwrap().logs;
        println!("Transaction logs:\n{:#?}", logs);

        // Check account data
        let account = svm
            .get_account(&counter_keypair.pubkey())
            .expect("Failed to get counter account");

        let counter: CounterAccount = CounterAccount::try_from_slice(account.data())
            .expect("Failed to deserialize counter data");
        assert_eq!(counter.count, 43);
        println!("Counter incremented successfully to: {}", counter.count);
    }
}
