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
    entrypoint::ProgramResult,
    msg,
    native_token::lamports_to_sol,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

use crate::utils::{any_as_u8_slice, calculate_streamed, unpack_account_data};

/// Program function to withdraw unlocked funds.
pub fn sol_withdraw_unlocked(pid: &Pubkey, accounts: &[AccountInfo], ix: &[u8]) -> ProgramResult {
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
