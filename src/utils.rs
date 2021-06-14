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

use solana_program::pubkey::Pubkey;

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
pub fn unpack_init_instruction(ix: &[u8], alice: &Pubkey, bob: &Pubkey) -> StreamFlow {
    StreamFlow {
        start_time: u64::from(u32::from_le_bytes(ix[1..5].try_into().unwrap())),
        end_time: u64::from(u32::from_le_bytes(ix[5..9].try_into().unwrap())),
        amount: u64::from_le_bytes(ix[9..17].try_into().unwrap()),
        withdrawn: 0,
        sender: alice.to_bytes(),
        recipient: bob.to_bytes(),
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
    }
}

/// Calculate unlocked funds from start to end.
pub fn calculate_streamed(now: u64, start: u64, end: u64, amount: u64) -> u64 {
    // This is valid float division, but we lose precision when going u64.
    // The loss however should not matter, as in the end we will simply
    // send everything that is remaining.
    (((now - start) as f64) / ((end - start) as f64) * amount as f64) as u64
}
