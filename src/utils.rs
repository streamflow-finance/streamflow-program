// Copyright (c) 2021 Ivan Jelincic <parazyd@dyne.org>
//
// This file is part of streamflow-program
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
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    pubkey::Pubkey,
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
    /// Pubkey of the token mint (can be zeroes for native SOL)
    pub mint: [u8; 32],
    /// Pubkey of the account holding the locked tokens
    /// (should be zeroes for native SOL)
    pub escrow: [u8; 32],
}

/// Serialize anything to u8 slice.
/// # Safety
///
/// :)
pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

/// Deserialize instruction_data into StreamFlow struct.
/// This is used to read instructions given to us by the program's initializer.
pub fn unpack_init_instruction(
    ix: &[u8],
    alice: &Pubkey,
    bob: &Pubkey,
    mint: &Pubkey,
) -> StreamFlow {
    StreamFlow {
        start_time: u64::from_le_bytes(ix[1..9].try_into().unwrap()),
        end_time: u64::from_le_bytes(ix[9..17].try_into().unwrap()),
        amount: u64::from_le_bytes(ix[17..25].try_into().unwrap()),
        withdrawn: 0,
        sender: alice.to_bytes(),
        recipient: bob.to_bytes(),
        mint: mint.to_bytes(),
        escrow: mint.to_bytes(),
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
        mint: ix[96..128].try_into().unwrap(),
        escrow: ix[96..128].try_into().unwrap(),
    }
}

/// Calculate unlocked funds from start to end.
pub fn calculate_streamed(now: u64, start: u64, end: u64, amount: u64) -> u64 {
    // This is valid float division, but we lose precision when going u64.
    // The loss however should not matter, as in the end we will simply
    // send everything that is remaining.
    (((now - start) as f64) / ((end - start) as f64) * amount as f64) as u64
}

/// Do a sanity check with given Unix timestamps.
pub fn duration_sanity(now: u64, start: u64, end: u64) -> bool {
    if start < now || start >= end {
        msg!("Timestamps are invalid!");
        msg!("Solana cluster time: {}", now);
        msg!("Stream start time:   {}", start);
        msg!("Stream end time:     {}", end);
        msg!("Stream duration:     {}", end - start);
        return false;
    }

    return true;
}

/// Structure used to pass parameters to spl_token_init_account()
pub struct TokenInitializeAccountParams<'a> {
    /// Account to initialize
    pub account: AccountInfo<'a>,
    /// Token mint account
    pub mint: AccountInfo<'a>,
    /// Account owner
    pub owner: AccountInfo<'a>,
    /// Rent account
    pub rent: AccountInfo<'a>,
    /// Token program account
    pub token_program: AccountInfo<'a>,
}

/// Used to initialize an SPL token account using given parameters
pub fn spl_token_init_account(params: TokenInitializeAccountParams<'_>) -> ProgramResult {
    let TokenInitializeAccountParams {
        account,
        mint,
        owner,
        rent,
        token_program,
    } = params;

    invoke(
        &spl_token::instruction::initialize_account(
            token_program.key,
            account.key,
            mint.key,
            owner.key,
        )?,
        &[account, mint, owner, rent, token_program],
    )
}

/// Structure used to pass parameters to spl_token_transfer()
pub struct TokenTransferParams<'a: 'b, 'b> {
    /// Source account
    pub source: AccountInfo<'a>,
    /// Destination account
    pub destination: AccountInfo<'a>,
    /// Amount of tokens to transfer (keep decimals in mind!)
    pub amount: u64,
    /// Account authority
    pub authority: AccountInfo<'a>,
    /// Account authority signer seeds
    pub authority_signer_seeds: &'b [&'b [u8]],
    /// Token program account
    pub token_program: AccountInfo<'a>,
}

/// Used to make a token transfer from A to B using given parameters
pub fn spl_token_transfer(params: TokenTransferParams<'_, '_>) -> ProgramResult {
    let TokenTransferParams {
        source,
        destination,
        authority,
        token_program,
        amount,
        authority_signer_seeds,
    } = params;

    invoke_signed(
        &spl_token::instruction::transfer(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?,
        &[source, destination, authority, token_program],
        &[authority_signer_seeds],
    )
}
