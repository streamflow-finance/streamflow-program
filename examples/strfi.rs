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
use std::{env, process, str::FromStr, time::SystemTime};

use solana_client::{blockhash_query::BlockhashQuery, rpc_client::RpcClient};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use solana_sdk::{
    signer::keypair::Keypair, signer::Signer, system_program, transaction::Transaction,
};

use streamflow::utils::any_as_u8_slice;

/// The program address to use.
const PROGRAM_ID: &'static str = "ETNwB99fC4HegvqBcHyvPhsiJ9x336NVTeqS11LBSmiP";
/// The Cluster RPC URL
const RPC_ADDR: &'static str = "http://localhost:8899";

// 71G4rRM4DugVRmAwEUtBNaw8xwGKZmSujwjFy37ErphW
/// Alice is our sender, make sure there is funds in the account.
const ALICE_KEY_BYTES: [u8; 64] = [
    97, 93, 122, 16, 225, 220, 239, 230, 206, 134, 241, 223, 228, 135, 202, 29, 7, 124, 108, 250,
    96, 12, 103, 91, 103, 95, 201, 25, 156, 18, 98, 149, 89, 55, 40, 62, 196, 151, 180, 107, 249,
    9, 23, 53, 215, 63, 170, 57, 173, 9, 36, 82, 233, 112, 55, 16, 15, 247, 47, 250, 115, 98, 210,
    129,
];

// H4wPUkepkJgB2FMaRyZWvsSpNUK8exoMonbRgRsipisb
/// Bob is our recipient
const BOB_KEY_BYTES: [u8; 64] = [
    104, 59, 250, 44, 167, 108, 233, 202, 30, 232, 3, 91, 108, 141, 125, 241, 216, 86, 189, 157,
    48, 69, 78, 98, 125, 6, 150, 127, 41, 214, 124, 242, 238, 189, 58, 189, 215, 194, 98, 74, 98,
    184, 196, 38, 158, 174, 51, 135, 76, 147, 74, 61, 214, 178, 94, 233, 190, 216, 78, 115, 83, 39,
    99, 226,
];

#[repr(packed(1))]
/// Structure layout for the native init instruction
struct InitLayout {
    instruction: u8,
    start_time: u64,
    end_time: u64,
    amount: u64,
}

#[repr(packed(1))]
/// Structure layout for the native withdraw instruction
struct WithdrawLayout {
    instruction: u8,
    amount: u64,
}

#[repr(packed(1))]
/// Structure layout for the native cancel instruction
struct CancelLayout {
    instruction: u8,
}

fn usage() {
    println!("usage: strfi [nativeinit|nativewithdraw|nativecancel] [accountAddress]\n");
    println!("accountAddress is needed for withdraw/cancel");
    process::exit(1);
}

fn keypair_from_some(envvar: &str, bytes: &[u8]) -> Result<Keypair, &'static str> {
    match env::var(envvar) {
        // TODO: Read key from file
        Ok(_v) => {
            println!("Loading {} key from file", envvar);
            return Err("foo");
        }
        Err(_) => match Keypair::from_bytes(bytes) {
            Ok(v) => return Ok(v),
            Err(_) => return Err("Could not parse key from bytes"),
        },
    }
}

fn create_keypairs() -> Vec<Keypair> {
    let mut res: Vec<Keypair> = Vec::new();
    let alice = keypair_from_some("ALICE", &ALICE_KEY_BYTES).unwrap();
    let bob = keypair_from_some("BOB", &BOB_KEY_BYTES).unwrap();

    res.push(alice);
    res.push(bob);

    return res;
}

fn create_accountmeta(keys: Vec<(Pubkey, bool)>) -> Vec<AccountMeta> {
    let mut ret: Vec<AccountMeta> = Vec::new();

    for i in keys {
        ret.push(AccountMeta::new(i.0, i.1));
    }

    return ret;
}

fn create_instruction(programid: Pubkey, ix: &[u8], keys: Vec<(Pubkey, bool)>) -> Instruction {
    return Instruction::new_with_bytes(programid, ix, create_accountmeta(keys));
}

fn create_tx(
    rpc: &RpcClient,
    ix: &[Instruction],
    payer: Option<&Pubkey>,
    signers: Vec<&Keypair>,
) -> Transaction {
    let mut tx = Transaction::new_with_payer(&ix, payer);
    let bhq = BlockhashQuery::default();
    match bhq.get_blockhash_and_fee_calculator(&rpc, rpc.commitment()) {
        Err(_) => panic!("Couldn't connect to rpc"),
        Ok(v) => tx.sign(&signers, v.0),
    }

    return tx;
}

fn native_init(rpc: RpcClient) {
    let kps = create_keypairs();
    let pda = Keypair::new();

    println!("ALICE: {}", kps[0].pubkey());
    println!("BOB:   {}", kps[1].pubkey());
    println!("PDA:   {}", pda.pubkey());

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let layout = InitLayout {
        instruction: 0,
        start_time: now as u64 + 15,
        end_time: now as u64 + 615,
        amount: 100000000,
    };

    println!("instruction: {}", { layout.instruction });
    println!("start_time: {}", { layout.start_time });
    println!("end_time: {}", { layout.end_time });
    println!("amount: {}", { layout.amount });

    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();

    let ix = create_instruction(
        program_id,
        unsafe { any_as_u8_slice(&layout) },
        vec![
            (kps[0].pubkey(), true),
            (kps[1].pubkey(), false),
            (pda.pubkey(), true),
            (kps[1].pubkey(), false), // placeholder
            (system_program::ID, false),
        ],
    );

    let tx = create_tx(&rpc, &[ix], Some(&kps[0].pubkey()), vec![&kps[0], &pda]);

    println!(
        "{:#?}",
        rpc.send_and_confirm_transaction_with_spinner(&tx)
            .unwrap()
            .to_string()
    );
}

fn native_withdraw(rpc: RpcClient, accaddr: &str) {
    let kps = create_keypairs();

    println!("ALICE: {}", kps[0].pubkey());
    println!("BOB:   {}", kps[1].pubkey());
    println!("PDA:   {}", accaddr);

    let layout = WithdrawLayout {
        instruction: 1,
        amount: 0, // 0 will withdraw everything that is unlocked
    };

    println!("instruction: {}", { layout.instruction });
    println!("amount: {}", { layout.amount });

    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();
    let lld_pub = Pubkey::from_str("DrFtxPb9F6SxpHHHFiEtSNXE3SZCUNLXMaHS6r8pkoz2").unwrap();
    let pda_pub = Pubkey::from_str(accaddr).unwrap();

    let ix = create_instruction(
        program_id,
        unsafe { any_as_u8_slice(&layout) },
        vec![(kps[1].pubkey(), true), (pda_pub, false), (lld_pub, false)],
    );

    let tx = create_tx(&rpc, &[ix], Some(&kps[1].pubkey()), vec![&kps[1]]);

    println!(
        "{:#?}",
        rpc.send_and_confirm_transaction_with_spinner(&tx)
            .unwrap()
            .to_string()
    );
}
fn native_cancel(rpc: RpcClient, accaddr: &str) {
    let kps = create_keypairs();

    println!("ALICE: {}", kps[0].pubkey());
    println!("BOB:   {}", kps[1].pubkey());
    println!("PDA:   {}", accaddr);

    let layout = CancelLayout { instruction: 2 };

    println!("instruction: {}", { layout.instruction });

    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();
    let lld_pub = Pubkey::from_str("DrFtxPb9F6SxpHHHFiEtSNXE3SZCUNLXMaHS6r8pkoz2").unwrap();
    let pda_pub = Pubkey::from_str(accaddr).unwrap();

    let ix = create_instruction(
        program_id,
        unsafe { any_as_u8_slice(&layout) },
        vec![
            (kps[0].pubkey(), true),
            (kps[1].pubkey(), false),
            (pda_pub, false),
            (lld_pub, false),
        ],
    );

    let tx = create_tx(&rpc, &[ix], Some(&kps[0].pubkey()), vec![&kps[0]]);

    println!(
        "{:#?}",
        rpc.send_and_confirm_transaction_with_spinner(&tx)
            .unwrap()
            .to_string()
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.len() > 3 {
        usage();
    }

    let rpc = RpcClient::new(RPC_ADDR.to_string());

    match args[1].as_str() {
        "nativeinit" => native_init(rpc),
        "nativewithdraw" => native_withdraw(rpc, &args[2].to_string()),
        "nativecancel" => native_cancel(rpc, &args[2].to_string()),
        _ => {
            println!("Invalid instruction");
            usage();
        }
    }
}
