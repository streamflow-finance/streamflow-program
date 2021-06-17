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
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

use crate::utils::{calculate_streamed, unpack_account_data};

/// Program function to cancel an initialized stream of funds.
pub fn sol_cancel_stream(pid: &Pubkey, accounts: &[AccountInfo], _ix: &[u8]) -> ProgramResult {
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
