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
pub mod sol_cancel;
pub mod sol_initialize;
pub mod sol_withdraw;
pub mod tok_initialize;
pub mod utils;

use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};

use sol_cancel::sol_cancel_stream;
use sol_initialize::sol_initialize_stream;
use sol_withdraw::sol_withdraw_unlocked;
use tok_initialize::tok_initialize_stream;

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
        // These are for native SOL
        0 => sol_initialize_stream(program_id, accounts, instruction_data),
        1 => sol_withdraw_unlocked(program_id, accounts, instruction_data),
        2 => sol_cancel_stream(program_id, accounts, instruction_data),
        // These are for SPL tokens
        3 => tok_initialize_stream(program_id, accounts, instruction_data),
        // 4 => tok_withdraw_unlocked(program_id, accounts, instruction_data),
        // 5 => tok_cancel_stream(program_id, accounts, instruction_data),
        // Invalid
        _ => Err(ProgramError::InvalidArgument),
    }
}
