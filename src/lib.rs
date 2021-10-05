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
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use streamflow_timelock::{
    associated_token::{cancel_token_stream, initialize_token_stream, withdraw_token_stream},
    native_token::{cancel_native_stream, initialize_native_stream, withdraw_native_stream},
    state::{
        NativeStreamCancelAccounts, NativeStreamInitAccounts, NativeStreamWithdrawAccounts,
        StreamInstruction,
    },
};

fn initialize_stream(
    is_native: bool,
    pid: &Pubkey,
    accounts: &[AccountInfo],
    ix: &[u8],
) -> ProgramResult {
    msg!("Deserializing instruction data");
    let si = match bincode::deserialize::<StreamInstruction>(ix) {
        Ok(v) => v,
        Err(_) => return Err(ProgramError::InvalidInstructionData),
    };

    if is_native {
        let account_info_iter = &mut accounts.iter();
        let sender_wallet = next_account_info(account_info_iter)?;
        let recipient_wallet = next_account_info(account_info_iter)?;
        let escrow_account = next_account_info(account_info_iter)?;
        let system_program_account = next_account_info(account_info_iter)?;

        let acc = NativeStreamInitAccounts {
            sender_wallet: sender_wallet.clone(),
            recipient_wallet: recipient_wallet.clone(),
            escrow_account: escrow_account.clone(),
            system_program_account: system_program_account.clone(),
        };

        initialize_native_stream(pid, acc, si)
    } else {
        initialize_token_stream(pid, accounts, si)
    }
}

fn withdraw_stream(
    is_native: bool,
    pid: &Pubkey,
    accounts: &[AccountInfo],
    ix: &[u8],
) -> ProgramResult {
    msg!("Deserializing instruction data");

    let amount: u64;

    match bincode::deserialize::<u64>(ix) {
        Ok(v) => amount = v,
        Err(_) => return Err(ProgramError::InvalidInstructionData),
    }

    if is_native {
        let account_info_iter = &mut accounts.iter();
        let sender_wallet = next_account_info(account_info_iter)?;
        let recipient_wallet = next_account_info(account_info_iter)?;
        let escrow_account = next_account_info(account_info_iter)?;

        let acc = NativeStreamWithdrawAccounts {
            sender_wallet: sender_wallet.clone(),
            recipient_wallet: recipient_wallet.clone(),
            escrow_account: escrow_account.clone(),
        };

        withdraw_native_stream(pid, acc, amount)
    } else {
        withdraw_token_stream(pid, accounts, amount)
    }
}

fn cancel_stream(is_native: bool, pid: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if is_native {
        let account_info_iter = &mut accounts.iter();
        let sender_wallet = next_account_info(account_info_iter)?;
        let recipient_wallet = next_account_info(account_info_iter)?;
        let escrow_account = next_account_info(account_info_iter)?;

        let acc = NativeStreamCancelAccounts {
            sender_wallet: sender_wallet.clone(),
            recipient_wallet: recipient_wallet.clone(),
            escrow_account: escrow_account.clone(),
        };

        cancel_native_stream(pid, acc)
    } else {
        cancel_token_stream(pid, accounts)
    }
}

entrypoint!(process_instruction);
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
        // true means native SOL; false means SPL token
        0 => initialize_stream(true, program_id, accounts, &instruction_data[1..]),
        1 => withdraw_stream(true, program_id, accounts, &instruction_data[1..]),
        2 => cancel_stream(true, program_id, accounts),
        3 => initialize_stream(false, program_id, accounts, &instruction_data[1..]),
        4 => withdraw_stream(false, program_id, accounts, &instruction_data[1..]),
        5 => cancel_stream(false, program_id, accounts),
        _ => Err(ProgramError::InvalidArgument),
    }
}
