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
    program_pack::Pack,
    pubkey::Pubkey,
    system_instruction,
    sysvar::rent::Rent,
    sysvar::Sysvar,
};
use spl_token::state::Account;

use crate::utils::{
    spl_token_init_account, spl_token_transfer, StreamFlow, TokenInitializeAccountParams,
    TokenTransferParams,
};

/// Program function to initialize a stream of tokens.
pub fn tok_initialize_stream(_pid: &Pubkey, accounts: &[AccountInfo], _ix: &[u8]) -> ProgramResult {
    msg!("Requested SPL token initialize_stream");
    let account_info_iter = &mut accounts.iter();

    let alice_authority = next_account_info(account_info_iter)?;
    let alice_tokens = next_account_info(account_info_iter)?;
    let bob = next_account_info(account_info_iter)?;
    let data_acc = next_account_info(account_info_iter)?;
    let escrow_acc = next_account_info(account_info_iter)?;
    let token_mint = next_account_info(account_info_iter)?;
    let rent_acc = next_account_info(account_info_iter)?;
    let self_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Rent calculation
    let cluster_rent = Rent::get()?;
    let data_struct_size = std::mem::size_of::<StreamFlow>();
    let toks_struct_size = Account::LEN;
    let data_rent = cluster_rent.minimum_balance(data_struct_size);
    let toks_rent = cluster_rent.minimum_balance(toks_struct_size);

    // TODO: Check if token_program == &spl_token::id()
    // TODO: Check for rent balances
    // TODO: Check if accounts are initialized
    // TODO: Check if everything is signed and writable as it should be
    // TODO: Create data account (c/p from native sol)

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
        amount: 100000,
        authority: alice_authority.clone(), // TODO: <--- check what is correct here
        authority_signer_seeds: &[],
        token_program: token_program.clone(),
    })?;

    Ok(())
}
