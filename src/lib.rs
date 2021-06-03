// Copyright (c) 2021 Ivan J. <parazyd@dyne.org>
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

pub mod strfi;

use crate::strfi::{cancel_stream, initialize_stream, withdraw_unlocked};

use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};

entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("StreamFlow Finance v0.0.1");

    match instruction_data[0] {
        0 => initialize_stream(program_id, accounts, instruction_data),
        1 => withdraw_unlocked(program_id, accounts, instruction_data),
        2 => cancel_stream(program_id, accounts, instruction_data),
        _ => Err(ProgramError::InvalidArgument),
    }
}
