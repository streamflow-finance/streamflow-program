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
    program::invoke,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{clock::Clock, fees::Fees, rent::Rent, Sysvar},
};
use spl_token::state::Account;

use crate::utils::{
    any_as_u8_slice, duration_sanity, spl_token_init_account, spl_token_transfer,
    unpack_init_instruction, StreamFlow, TokenInitializeAccountParams, TokenTransferParams,
};

/// Program function to initialize a stream of tokens.
pub fn tok_initialize_stream(pid: &Pubkey, accounts: &[AccountInfo], ix: &[u8]) -> ProgramResult {
    msg!("Requested SPL token initialize_stream");
    let account_info_iter = &mut accounts.iter();

    let alice_authority = next_account_info(account_info_iter)?;
    let alice_tokens = next_account_info(account_info_iter)?;
    let bob_authority = next_account_info(account_info_iter)?;
    let bob_tokens = next_account_info(account_info_iter)?;
    let data_acc = next_account_info(account_info_iter)?;
    let escrow_acc = next_account_info(account_info_iter)?;
    let token_mint = next_account_info(account_info_iter)?;
    let rent_acc = next_account_info(account_info_iter)?;
    let self_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    if ix.len() != 25 {
        return Err(ProgramError::InvalidInstructionData);
    }

    if token_program.key != &spl_token::id() {
        msg!("Mismatched Token program address in [accounts]!");
        return Err(ProgramError::InvalidInstructionData);
    }

    if self_program.key != pid {
        msg!("Mismatched program address in [accounts]");
        return Err(ProgramError::InvalidInstructionData);
    }

    if !data_acc.data_is_empty() || !escrow_acc.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if !alice_authority.is_signer
        || !alice_authority.is_writable
        || !alice_tokens.is_writable
        || !bob_authority.is_writable
        || !bob_tokens.is_writable
        || !data_acc.is_signer
        || !data_acc.is_writable
        || !escrow_acc.is_signer
        || !escrow_acc.is_writable
    {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Rent calculation
    let cluster_rent = Rent::get()?;
    let data_struct_size = std::mem::size_of::<StreamFlow>();
    let toks_struct_size = Account::LEN;
    let data_rent = cluster_rent.minimum_balance(data_struct_size);
    let toks_rent = cluster_rent.minimum_balance(toks_struct_size);

    // Unpack instruction into struct
    let sf = unpack_init_instruction(ix, alice_authority.key, bob_authority.key, token_mint.key);

    let now = Clock::get()?.unix_timestamp as u64;
    if !duration_sanity(now, sf.start_time, sf.end_time) {
        return Err(ProgramError::InvalidArgument);
    }

    // Fee calculator
    let fees = Fees::get()?;
    let lps = fees.fee_calculator.lamports_per_signature;

    // We also transfer enough to be rent-exempt (about 0.0016 SOL) to the
    // new accounts. After all funds are unlocked and withdrawn, this shall
    // be transferred to a rent-reaping address.
    if alice_authority.lamports() < data_rent + toks_rent + (4 * lps) {
        msg!("Not enough funds in sender's account to initialize SPL token stream");
        return Err(ProgramError::InsufficientFunds);
    }

    // Create the account holding this stream's metadata
    invoke(
        &system_instruction::create_account(
            &alice_authority.key,
            &data_acc.key,
            cluster_rent.minimum_balance(data_struct_size) + (lps * 4),
            data_struct_size as u64,
            &pid,
        ),
        &[
            alice_authority.clone(),
            data_acc.clone(),
            system_program.clone(),
        ],
    )?;

    // Send enough for one instruction to Bob, so Bob can do an initial
    // withdraw without having previous funds on their account.
    **data_acc.try_borrow_mut_lamports()? -= lps * 3;
    **bob_authority.try_borrow_mut_lamports()? += lps * 3;

    // Write our metadata to data_acc's data.
    let mut data_acc_data = data_acc.try_borrow_mut_data()?;
    let bytes: &[u8] = unsafe { any_as_u8_slice(&sf) };
    data_acc_data[0..bytes.len()].clone_from_slice(bytes);

    // Create escrow account so we can transfer tokens to it.
    invoke(
        &system_instruction::create_account(
            alice_authority.key,
            escrow_acc.key,
            toks_rent,
            toks_struct_size as u64,
            &spl_token::id(),
        ),
        &[
            alice_authority.clone(),
            escrow_acc.clone(),
            system_program.clone(),
        ],
    )?;

    // Initialize the created escrow account with SPL token data
    spl_token_init_account(TokenInitializeAccountParams {
        account: escrow_acc.clone(),
        mint: token_mint.clone(),
        owner: self_program.clone(),
        rent: rent_acc.clone(),
        token_program: token_program.clone(),
    })?;

    // Transfer tokens into escrow
    spl_token_transfer(TokenTransferParams {
        source: alice_tokens.clone(),
        destination: escrow_acc.clone(),
        amount: sf.amount,
        authority: alice_authority.clone(), // TODO: <--- check what is correct here
        authority_signer_seeds: &[],
        token_program: token_program.clone(),
    })?;

    // TODO: Better output
    msg!("Successfully initialized stream for: {}", bob_authority.key);
    msg!("Called by account: {}", alice_authority.key);
    msg!("Funds locked in account: {}", escrow_acc.key);
    msg!("Stream start:    {}", sf.start_time);
    msg!("Stream end:      {}", sf.end_time);
    msg!("Stream duration: {} seconds", sf.end_time - sf.start_time);

    Ok(())
}
