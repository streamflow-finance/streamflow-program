// Copyright (c) 2021 Ivan Jelincic <parazyd@dyne.org>
//
// This file is part of strfi
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License version 3
// as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::convert::TryInto;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    fee_calculator::DEFAULT_TARGET_LAMPORTS_PER_SIGNATURE,
    msg,
    native_token::lamports_to_sol,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::{clock::Clock, Sysvar},
};

// StreamFlow is the struct containing all our necessary metadata.
#[repr(C)]
struct StreamFlow {
    // instruction: u8
    start_time: u64,     // Timestamp when the funds start unlocking
    end_time: u64,       // Timestamp when all funds should be unlocked
    amount: u64,         // Amount of funds locked
    withdrawn: u64,      // Amount of funds withdrawn
    sender: [u8; 32],    // Pubkey of the program initializer
    recipient: [u8; 32], // Pubkey of the funds' recipient
}

// Used to serialize StreamFlow to bytes.
unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

// Deserialize instruction_data into StreamFlow struct.
// This is used to read instructions given to us by the program's initializer.
fn unpack_init_instruction(ix: &[u8], alice: &Pubkey, bob: &Pubkey) -> StreamFlow {
    let sf = StreamFlow {
        start_time: u64::from(u32::from_le_bytes(ix[1..5].try_into().unwrap())),
        end_time: u64::from(u32::from_le_bytes(ix[5..9].try_into().unwrap())),
        amount: u64::from_le_bytes(ix[9..17].try_into().unwrap()),
        withdrawn: 0,
        sender: alice.to_bytes(),
        recipient: bob.to_bytes(),
    };

    return sf;
}

// Deserialize account data into StreamFlow struct.
// This is used for reading the metadata from the account holding the locked funds.
fn unpack_account_data(ix: &[u8]) -> StreamFlow {
    let sf = StreamFlow {
        start_time: u64::from_le_bytes(ix[0..8].try_into().unwrap()),
        end_time: u64::from_le_bytes(ix[8..16].try_into().unwrap()),
        amount: u64::from_le_bytes(ix[16..24].try_into().unwrap()),
        withdrawn: u64::from_le_bytes(ix[24..32].try_into().unwrap()),
        sender: ix[32..64].try_into().unwrap(),
        recipient: ix[64..96].try_into().unwrap(),
    };

    return sf;
}

fn initialize_stream(pid: &Pubkey, accounts: &[AccountInfo], ix: &[u8]) -> ProgramResult {
    msg!("Requested stream initialization");
    let account_info_iter = &mut accounts.iter();
    let alice = next_account_info(account_info_iter)?;
    let bob = next_account_info(account_info_iter)?;
    let pda = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    if ix.len() != 17 {
        return Err(ProgramError::InvalidInstructionData);
    }

    if !pda.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if !alice.is_writable
        || !bob.is_writable
        || !pda.is_writable
        || !alice.is_signer
        || !pda.is_signer
    {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut sf = unpack_init_instruction(ix, alice.key, bob.key);
    let struct_size = std::mem::size_of::<StreamFlow>();

    // We transfer also enough to be rent-exempt (about 0.00156 SOL) to the
    // new account. After all funds are withdrawn and unlocked, this might
    // be returned to the initializer or put in another pool for future reuse.
    let rent_min = Rent::default().minimum_balance(struct_size);

    if alice.lamports() < sf.amount + rent_min {
        msg!("Not enough funds in sender's account to initialize stream");
        return Err(ProgramError::InsufficientFunds);
    }

    match Clock::get() {
        Ok(v) => {
            if sf.start_time < v.unix_timestamp as u64 || sf.start_time >= sf.end_time {
                msg!("Timestamps are invalid!");
                msg!("Solana cluster time: {}", v.unix_timestamp);
                msg!("Stream start time:   {}", sf.start_time);
                msg!("Stream end time:     {}", sf.end_time);
                msg!("Stream duration:     {}", sf.end_time - sf.start_time);
                return Err(ProgramError::InvalidArgument);
            }
        }
        Err(e) => return Err(e),
    }

    // Create the account holding locked funds and data
    invoke(
        &system_instruction::create_account(
            &alice.key,
            &pda.key,
            sf.amount + rent_min,
            struct_size as u64,
            &pid,
        ),
        &[alice.clone(), pda.clone(), system_program.clone()],
    )?;

    // Send enough for one transaction to Bob, so Bob can do an initial
    // withdraw without having previous funds on their account.
    **pda.try_borrow_mut_lamports()? -= DEFAULT_TARGET_LAMPORTS_PER_SIGNATURE;
    **bob.try_borrow_mut_lamports()? += DEFAULT_TARGET_LAMPORTS_PER_SIGNATURE;
    sf.withdrawn += DEFAULT_TARGET_LAMPORTS_PER_SIGNATURE;

    // Write our metadata to pda's data.
    let mut data = pda.try_borrow_mut_data()?;
    let bytes: &[u8] = unsafe { any_as_u8_slice(&sf) };
    data[0..bytes.len()].clone_from_slice(bytes);

    msg!(
        "Successfully initialized {} SOL ({} lamports) stream for: {}",
        lamports_to_sol(sf.amount),
        sf.amount,
        bob.key
    );
    msg!("Called by account: {}", alice.key);
    msg!("Funds locked in account: {}", pda.key);
    msg!("Stream duration: {} seconds", sf.end_time - sf.start_time);

    Ok(())
}

fn withdraw_unlocked(_pid: &Pubkey, accounts: &[AccountInfo], ix: &[u8]) -> ProgramResult {
    msg!("Requested withdraw of unlocked funds");
    let account_info_iter = &mut accounts.iter();
    let bob = next_account_info(account_info_iter)?;
    let pda = next_account_info(account_info_iter)?;

    if ix.len() != 9 {
        return Err(ProgramError::InvalidInstructionData);
    }

    if !bob.is_signer {
        msg!("ERROR: Bob didn't sign tx");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if pda.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    let mut data = pda.try_borrow_mut_data()?;
    let mut sf = unpack_account_data(&data);

    if bob.key.to_bytes() != sf.recipient {
        msg!("ERROR: bob.key != sf.recipient");
        return Err(ProgramError::InvalidArgument);
    }

    // Current cluster time used to calculate unlocked amount.
    let now: u64;
    match Clock::get() {
        Ok(v) => now = v.unix_timestamp as u64,
        Err(e) => return Err(e),
    }

    // This is valid float division, but we lose precision when going u64.
    // The loss however should not matter, as in the end we will simply
    // send everything that is remaining.
    let amount_unlocked = (((now - sf.start_time) as f64) / ((sf.end_time - sf.start_time) as f64)
        * sf.amount as f64) as u64;
    let mut available = amount_unlocked - sf.withdrawn;

    // In case we're past the set time, everything is available.
    if now >= sf.end_time {
        available = sf.amount - sf.withdrawn;
    }

    let mut requested = u64::from_le_bytes(ix[1..9].try_into().unwrap());
    if requested == 0 {
        requested = available;
    }

    if requested > available {
        msg!("Amount requested for withdraw is larger than what is available.");
        msg!(
            "Requested: {} SOL ({} lamports)",
            lamports_to_sol(requested),
            requested
        );
        msg!(
            "Available: {}SOL ({} lamports)",
            lamports_to_sol(available),
            available
        );
        return Err(ProgramError::InvalidArgument);
    }

    **pda.try_borrow_mut_lamports()? -= requested;
    **bob.try_borrow_mut_lamports()? += requested;

    // Update account data
    sf.withdrawn += available as u64;
    let bytes: &[u8] = unsafe { any_as_u8_slice(&sf) };
    data[0..bytes.len()].clone_from_slice(bytes);

    msg!(
        "Successfully withdrawn: {} SOL ({} lamports)",
        lamports_to_sol(available),
        available
    );
    msg!(
        "Remaining: {} SOL ({} lamports)",
        lamports_to_sol(sf.amount - sf.withdrawn),
        sf.amount - sf.withdrawn
    );

    // TODO: Return remaining rent somewhere.

    Ok(())
}

fn cancel_stream(_pid: &Pubkey, accounts: &[AccountInfo], _ix: &[u8]) -> ProgramResult {
    msg!("Requested stream cancellation");
    let account_info_iter = &mut accounts.iter();
    let alice = next_account_info(account_info_iter)?;
    let pda = next_account_info(account_info_iter)?;

    if !alice.is_signer || !alice.is_writable || !pda.is_writable {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if pda.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    let data = pda.try_borrow_data()?;
    let sf = unpack_account_data(&data);

    if alice.key.to_bytes() != sf.sender {
        msg!("ERROR: alice.key != sf.sender");
        return Err(ProgramError::InvalidArgument);
    }

    // Alice decides to cancel, and withdraws from the derived account,
    // resulting in its purge.
    let avail = pda.lamports();
    **pda.try_borrow_mut_lamports()? -= avail;
    **alice.try_borrow_mut_lamports()? += avail;

    msg!("Successfully cancelled stream on {} account", pda.key);
    msg!("Remaining lamports ({}) returned to {}", avail, alice.key);

    Ok(())
}

entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("StreamFlowFinance v0.0.1");

    match instruction_data[0] {
        0 => initialize_stream(program_id, accounts, instruction_data),
        1 => withdraw_unlocked(program_id, accounts, instruction_data),
        2 => cancel_stream(program_id, accounts, instruction_data),
        _ => Err(ProgramError::InvalidArgument),
    }
}
