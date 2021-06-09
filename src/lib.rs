// Copyright (c) 2021 Ivan Jelincic <parazyd@dyne.org>
//
// This file is part of streamflow-program
// https://github.com/StreamFlow-Finance/streamflow-program
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
use std::str::FromStr;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    native_token::lamports_to_sol,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{clock::Clock, fees::Fees, rent::Rent, Sysvar},
};

/// StreamFlow is the struct containing all our necessary metadata.
#[repr(C)]
pub struct StreamFlow {
    /// Timestamp when the funds start unlocking
    pub start_time: u64,
    /// Timestamp when all funds should be unlocked
    pub end_time: u64,
    /// Amount of funds locked
    pub amount: u64,
    /// Amount of funds withdrawn
    pub withdrawn: u64,
    /// Pubkey of the program initializer
    pub sender: [u8; 32],
    /// Pubkey of the funds' recipient
    pub recipient: [u8; 32],
}

/// Serialize any to u8 slice.
/// # Safety
///
/// :)
pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

/// Deserialize instruction_data into StreamFlow struct.
/// This is used to read instructions given to us by the program's initializer.
pub fn unpack_init_instruction(ix: &[u8], alice: &Pubkey, bob: &Pubkey) -> StreamFlow {
    StreamFlow {
        start_time: u64::from(u32::from_le_bytes(ix[1..5].try_into().unwrap())),
        end_time: u64::from(u32::from_le_bytes(ix[5..9].try_into().unwrap())),
        amount: u64::from_le_bytes(ix[9..17].try_into().unwrap()),
        withdrawn: 0,
        sender: alice.to_bytes(),
        recipient: bob.to_bytes(),
    }
}

/// Deserialize account data into StreamFlow struct.
/// This is used for reading the metadata from the account holding the locked funds.
pub fn unpack_account_data(ix: &[u8]) -> StreamFlow {
    StreamFlow {
        start_time: u64::from_le_bytes(ix[0..8].try_into().unwrap()),
        end_time: u64::from_le_bytes(ix[8..16].try_into().unwrap()),
        amount: u64::from_le_bytes(ix[16..24].try_into().unwrap()),
        withdrawn: u64::from_le_bytes(ix[24..32].try_into().unwrap()),
        sender: ix[32..64].try_into().unwrap(),
        recipient: ix[64..96].try_into().unwrap(),
    }
}

fn calculate_streamed(now: u64, start: u64, end: u64, amount: u64) -> u64 {
    // This is valid float division, but we lose precision when going u64.
    // The loss however should not matter, as in the end we will simply
    // send everything that is remaining.
    (((now - start) as f64) / ((end - start) as f64) * amount as f64) as u64
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

    // We also transfer enough to be rent-exempt (about 0.00156 SOL) to the
    // new account. After all funds are withdrawn and unlocked, this might
    // be returned to the initializer or put in another pool for future reuse.
    let cluster_rent = Rent::get()?;
    if alice.lamports() < sf.amount + cluster_rent.minimum_balance(struct_size) {
        msg!("Not enough funds in sender's account to initialize stream");
        return Err(ProgramError::InsufficientFunds);
    }

    let now = Clock::get()?.unix_timestamp as u64;
    if sf.start_time < now || sf.start_time >= sf.end_time {
        msg!("Timestamps are invalid!");
        msg!("Solana cluster time: {}", now);
        msg!("Stream start time:   {}", sf.start_time);
        msg!("Stream end time:     {}", sf.end_time);
        msg!("Stream duration:     {}", sf.end_time - sf.start_time);
        return Err(ProgramError::InvalidArgument);
    }

    // Create the account holding locked funds and data
    invoke(
        &system_instruction::create_account(
            &alice.key,
            &pda.key,
            sf.amount + cluster_rent.minimum_balance(struct_size),
            struct_size as u64,
            &pid,
        ),
        &[alice.clone(), pda.clone(), system_program.clone()],
    )?;

    // Send enough for one transaction to Bob, so Bob can do an initial
    // withdraw without having previous funds on their account.
    let fees = Fees::get()?;
    **pda.try_borrow_mut_lamports()? -= fees.fee_calculator.lamports_per_signature * 2;
    **bob.try_borrow_mut_lamports()? += fees.fee_calculator.lamports_per_signature * 2;
    sf.withdrawn += fees.fee_calculator.lamports_per_signature * 2;

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

fn withdraw_unlocked(pid: &Pubkey, accounts: &[AccountInfo], ix: &[u8]) -> ProgramResult {
    msg!("Requested withdraw of unlocked funds");
    let account_info_iter = &mut accounts.iter();
    let bob = next_account_info(account_info_iter)?;
    let pda = next_account_info(account_info_iter)?;
    let lld = next_account_info(account_info_iter)?;

    if ix.len() != 9 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Hardcoded rent collector
    let rent_reaper = Pubkey::from_str("DrFtxPb9F6SxpHHHFiEtSNXE3SZCUNLXMaHS6r8pkoz2").unwrap();
    if lld.key != &rent_reaper {
        msg!("Got unexpected rent collection account");
        return Err(ProgramError::InvalidAccountData);
    }

    if !bob.is_signer || !bob.is_writable || !pda.is_writable || !lld.is_writable {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if pda.data_is_empty() || pda.owner != pid {
        return Err(ProgramError::UninitializedAccount);
    }

    let mut data = pda.try_borrow_mut_data()?;
    let mut sf = unpack_account_data(&data);

    if bob.key.to_bytes() != sf.recipient {
        msg!("This stream isn't indented for {}", bob.key);
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Current cluster time used to calculate unlocked amount.
    let now = Clock::get()?.unix_timestamp as u64;

    let amount_unlocked = calculate_streamed(now, sf.start_time, sf.end_time, sf.amount);
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
            "Available: {} SOL ({} lamports)",
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

    /*
    if sf.withdrawn == sf.amount {
        // Collect rent after stream is finished.
        let rent = pda.lamports();
        **pda.try_borrow_mut_lamports()? -= rent;
        **lld.try_borrow_mut_lamports()? += rent;
    }
    */

    Ok(())
}

fn cancel_stream(pid: &Pubkey, accounts: &[AccountInfo], _ix: &[u8]) -> ProgramResult {
    msg!("Requested stream cancellation");
    let account_info_iter = &mut accounts.iter();
    let alice = next_account_info(account_info_iter)?;
    let bob = next_account_info(account_info_iter)?;
    let pda = next_account_info(account_info_iter)?;

    if !alice.is_signer || !alice.is_writable || !bob.is_writable || !pda.is_writable {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if pda.data_is_empty() || pda.owner != pid {
        return Err(ProgramError::UninitializedAccount);
    }

    let data = pda.try_borrow_data()?;
    let sf = unpack_account_data(&data);

    if alice.key.to_bytes() != sf.sender {
        msg!("Unauthorized to withdraw for {}", alice.key);
        return Err(ProgramError::MissingRequiredSignature);
    }

    if bob.key.to_bytes() != sf.recipient {
        msg!("This stream isn't intended for {}", bob.key);
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Current cluster time used to calculate unlocked amount.
    let now = Clock::get()?.unix_timestamp as u64;

    // Transfer what was unlocked but not withdrawn to Bob.
    let amount_unlocked = calculate_streamed(now, sf.start_time, sf.end_time, sf.amount);
    let available = amount_unlocked - sf.withdrawn;
    **pda.try_borrow_mut_lamports()? -= available;
    **bob.try_borrow_mut_lamports()? += available;

    // Alice decides to cancel, and withdraws from the derived account,
    // resulting in its purge.
    let remains = pda.lamports();
    **pda.try_borrow_mut_lamports()? -= remains;
    **alice.try_borrow_mut_lamports()? += remains;

    msg!("Successfully cancelled stream on {} ", pda.key);
    msg!(
        "Transferred unlocked {} SOL ({} lamports to {}",
        lamports_to_sol(available),
        available,
        bob.key
    );
    msg!(
        "Returned {} SOL ({} lamports) to {}",
        lamports_to_sol(remains),
        remains,
        alice.key
    );

    Ok(())
}

entrypoint!(process_instruction);
/// The program entrypoint
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!(
        "StreamFlowFinance v{}.{}.{}",
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
        env!("CARGO_PKG_VERSION_PATCH")
    );

    match instruction_data[0] {
        0 => initialize_stream(program_id, accounts, instruction_data),
        1 => withdraw_unlocked(program_id, accounts, instruction_data),
        2 => cancel_stream(program_id, accounts, instruction_data),
        _ => Err(ProgramError::InvalidArgument),
    }
}
