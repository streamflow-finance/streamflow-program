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

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::{clock::Clock, Sysvar},
};
use std::convert::TryInto;

#[repr(C)]
struct StreamFlow {
    start_time: u64,
    end_time: u64,
    amount: u64,
    withdrawn: u64,
    sender: [u8; 32],
    recipient: [u8; 32],
}

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

unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

pub fn initialize_stream(pid: &Pubkey, accounts: &[AccountInfo], ix: &[u8]) -> ProgramResult {
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

    match Clock::get() {
        Ok(v) => {
            msg!("SOLANATIME: {}", v.unix_timestamp);
            msg!("STARTTIME:  {}", sf.start_time);
            msg!("ENDTIME:    {}", sf.end_time);
            msg!("DURATION:   {}", sf.end_time - sf.start_time);
            if sf.start_time < v.unix_timestamp as u64 || sf.start_time >= sf.end_time {
                msg!("ERROR: Timestamps are incorrect");
                return Err(ProgramError::InvalidArgument);
            }
        }
        Err(e) => return Err(e),
    }

    let struct_size = std::mem::size_of::<StreamFlow>();
    // TODO: make this rent-exempt rather than minimum_balance
    // and on the end, return what's left to Alice.
    let rent_min = Rent::default().minimum_balance(struct_size);

    // TODO: Review exact amount
    if alice.lamports() < sf.amount + rent_min {
        msg!("Not enough funds in sender's account to initialize stream");
        return Err(ProgramError::InsufficientFunds);
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
    // TODO: Calculate correct fees
    **pda.try_borrow_mut_lamports()? -= 5000;
    **bob.try_borrow_mut_lamports()? += 5000;
    sf.withdrawn += 5000;

    // Write our metadata to pda's data.
    let mut data = pda.try_borrow_mut_data()?;
    let bytes: &[u8] = unsafe { any_as_u8_slice(&sf) };
    data[0..bytes.len()].clone_from_slice(bytes);

    Ok(())
}

pub fn withdraw_unlocked(_pid: &Pubkey, accounts: &[AccountInfo], ix: &[u8]) -> ProgramResult {
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

    // In case we're past the set time, we will withdraw what's left.
    // TODO: Send rent back to Alice.
    if now >= sf.end_time {
        available = sf.amount - sf.withdrawn;
    }

    msg!("SOLTIME:   {}", now);
    msg!("STARTTIME: {}", sf.start_time);
    msg!("ENDTIME:   {}", sf.end_time);
    msg!("TOTAL:     {}", sf.amount);
    msg!("UNLOCKED:  {}", amount_unlocked);
    msg!("AVAILABLE: {}", available);

    msg!("BOBS LAMPORTS: {}", bob.lamports());
    msg!("PDAS LAMPORTS: {}", pda.lamports());

    // TODO: Withdraw amount asked in instruction
    **pda.try_borrow_mut_lamports()? -= available;
    **bob.try_borrow_mut_lamports()? += available;

    // Update account data
    sf.withdrawn += available as u64;
    let bytes: &[u8] = unsafe { any_as_u8_slice(&sf) };
    data[0..bytes.len()].clone_from_slice(bytes);

    Ok(())
}

pub fn cancel_stream(_pid: &Pubkey, accounts: &[AccountInfo], _ix: &[u8]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let alice = next_account_info(account_info_iter)?;
    let pda = next_account_info(account_info_iter)?;

    if !alice.is_signer {
        msg!("ERROR: Alice didn't sign tx");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if pda.data_is_empty() {
        msg!("pda data is empty");
        return Err(ProgramError::UninitializedAccount);
    }

    let data = pda.try_borrow_mut_data()?;
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

    Ok(())
}
