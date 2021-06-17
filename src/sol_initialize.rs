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
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    native_token::lamports_to_sol,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{clock::Clock, fees::Fees, rent::Rent, Sysvar},
};

use crate::utils::{any_as_u8_slice, duration_sanity, unpack_init_instruction, StreamFlow};

/// Program function to initialize a stream of native SOL.
pub fn sol_initialize_stream(pid: &Pubkey, accounts: &[AccountInfo], ix: &[u8]) -> ProgramResult {
    msg!("Requested native SOL initialize_stream");
    let account_info_iter = &mut accounts.iter();
    let alice = next_account_info(account_info_iter)?;
    let bob = next_account_info(account_info_iter)?;
    let pda = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // TODO: Organize so all sanity checks are before doing something.

    if ix.len() != 25 {
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

    let mut sf = unpack_init_instruction(ix, alice.key, bob.key, bob.key);
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
    if !duration_sanity(now, sf.start_time, sf.end_time) {
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
