#!/usr/bin/env node

/* Copyright (c) 2021 Ivan Jelincic <parazyd@dyne.org>
 *
 * This file is part of streamflow-program
 * https://github.com/StreamFlow-Finance/streamflow-program
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License version 3
 * as published by the Free Software Foundation.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

// This file serves as a reference on how to use streamflow-program
// with Javascript.
const BufferLayout = require("buffer-layout");
const sol = require("@solana/web3.js");
const spl = require("@solana/spl-token");
const fs = require('fs');

// Cluster and program address to use
// const cluster = "http://localhost:8899";
const cluster = sol.clusterApiUrl("devnet", true);
const programAddr = "2DvvSEde36Ch3B52g9hKWDYbfmJimLpJwVBV9Cknypi4"

// Alice is our sender, make sure there is funds in the account.
// 71G4rRM4DugVRmAwEUtBNaw8xwGKZmSujwjFy37ErphW
let alice;
if (process.env.ALICE !== undefined) {
    alice = sol.Keypair.fromSecretKey(
        Buffer.from(JSON.parse(fs.readFileSync(process.env.ALICE, "utf-8"))));
} else {
    alice = sol.Keypair.fromSecretKey(Buffer.from([97, 93, 122, 16, 225,
    220, 239, 230, 206, 134, 241, 223, 228, 135, 202, 29, 7, 124, 108, 250,
    96, 12, 103, 91, 103, 95, 201, 25, 156, 18, 98, 149, 89, 55, 40, 62, 196,
    151, 180, 107, 249, 9, 23, 53, 215, 63, 170, 57, 173, 9, 36, 82, 233, 112,
    55, 16, 15, 247, 47, 250, 115, 98, 210, 129]));
}
// await connection.requestAirdrop(alice.publicKey, 1000000000);

// Bob is our recipient
// H4wPUkepkJgB2FMaRyZWvsSpNUK8exoMonbRgRsipisb
let bob;
if (process.env.BOB !== undefined) {
    bob = sol.Keypair.fromSecretKey(
        Buffer.from(JSON.parse(fs.readFileSync(process.env.BOB, "utf-8"))));
} else {
    bob = sol.Keypair.fromSecretKey(Buffer.from([104, 59, 250, 44, 167,
    108, 233, 202, 30, 232, 3, 91, 108, 141, 125, 241, 216, 86, 189, 157, 48,
    69, 78, 98, 125, 6, 150, 127, 41, 214, 124, 242, 238, 189, 58, 189, 215,
    194, 98, 74, 98, 184, 196, 38, 158, 174, 51, 135, 76, 147, 74, 61, 214,
    178, 94, 233, 190, 216, 78, 115, 83, 39, 99, 226]));
}

function usage() {
    console.log("usage: strfi.js [init|withdraw|cancel] [accountAddress (needed for withdraw/cancel)]");
    process.exit(1);
}

// This is the structure for the init instruction
const initLayout = BufferLayout.struct([
    BufferLayout.u8("instruction"),
    BufferLayout.blob(8, "starttime"),
    BufferLayout.blob(8, "endtime"),
    BufferLayout.blob(8, "amount"),
]);

// This is the structure for the withdraw instruction
const withdrawLayout = BufferLayout.struct([
    BufferLayout.u8("instruction"),
    BufferLayout.blob(8, "amount"),
]);

// This is the structure for the cancel instruction
const cancelLayout = BufferLayout.struct([
    BufferLayout.u8("instruction"),
]);

async function initStream(connection) {
    // Current time as Unix timestamp
    now = Math.floor(new Date().getTime() / 1000);

    var data = Buffer.alloc(initLayout.span);
    initLayout.encode({
            // 0 means init in the Rust program.
            instruction: 0,
            // Unix timestamp when the stream should start unlocking.
            starttime: new spl.u64(now + 10).toBuffer(),
            // Unix timestamp when the stream should finish and unlock everything.
            endtime: new spl.u64(now + 610).toBuffer(),
            // Lamports to stream
            amount: new spl.u64(100000000).toBuffer(),
        },
        data,
    );

    // pda is a new keypair where the funds are sent, and program metadata
    // is kept and updated by the program.
    const pda = new sol.Keypair();

    console.log("ALICE: %s", alice.publicKey.toBase58());
    console.log("BOB:   %s", bob.publicKey.toBase58());
    console.log("PDA:   %s", pda.publicKey.toBase58());
    console.log("DATA:", data);

    const instruction = new sol.TransactionInstruction({
        keys: [{
            // Alice is the stream sender.
            pubkey: alice.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            // Bob is the stream recipient.
            pubkey: bob.publicKey,
            isSigner: false,
            isWritable: true,
        }, {
            // pda is the account that will be created.
            // It shall contain the locked funds and necessary metadata.
            pubkey: pda.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            // This is the system program public key.
            pubkey: sol.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        }],
        programId: new sol.PublicKey(programAddr),
        data: data,
    });

    // Transaction signed by Alice and the new pda.
    tx = new sol.Transaction().add(instruction);
    return await sol.sendAndConfirmTransaction(connection, tx, [alice, pda]);
}

async function withdrawStream(connection, accountAddr) {
    var data = Buffer.alloc(withdrawLayout.span);
    withdrawLayout.encode({
            // 1 means withdraw in the Rust program.
            instruction: 1,
            // When amount is 0 lamports, then withdraw everything
            // that is unlocked on the stream. Otherwise, arbitrary
            // values are allowed.
            amount: 0,
        },
        data,
    );
    console.log("ALICE: %s", alice.publicKey.toBase58());
    console.log("BOB:   %s", bob.publicKey.toBase58());
    console.log("PDA:   %s", accountAddr);
    console.log("DATA:", data);

    const instruction = new sol.TransactionInstruction({
        keys: [{
            // Bob is the stream recipient.
            pubkey: bob.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            // This is the public key of the account where the funds
            // and metadata are held.
            pubkey: new sol.PublicKey(accountAddr),
            isSigner: false,
            isWritable: true,
        }, {
            // This address is hardcoded in the program, and is supposed
            // to collect the remaining rent when everything is withdrawn
            // from the stream successfully.
            pubkey: new sol.PublicKey("DrFtxPb9F6SxpHHHFiEtSNXE3SZCUNLXMaHS6r8pkoz2"),
            isSigner: false,
            isWritable: true,
        }, {
            // This is the system program public key.
            pubkey: sol.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        }],
        programId: new sol.PublicKey(programAddr),
        data: data,
    });

    // Transaction signed by Bob.
    tx = new sol.Transaction().add(instruction);
    return await sol.sendAndConfirmTransaction(connection, tx, [bob]);
}

async function cancelStream(connection, accountAddr) {
    var data = Buffer.alloc(cancelLayout.span);
    cancelLayout.encode({
            // 2 means cancel in the Rust program.
            instruction: 2,
        },
        data,
    );

    console.log("ALICE: %s", alice.publicKey.toBase58());
    console.log("BOB:   %s", bob.publicKey.toBase58());
    console.log("PDA:   %s", accountAddr);
    console.log("DATA:", data);

    // The transaction instruction contains the public keys used.
    const instruction = new sol.TransactionInstruction({
        keys: [{
            // Alice is our initial stream sender.
            pubkey: alice.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            // Bob is the stream recipient.
            pubkey: bob.publicKey,
            isSigner: false,
            isWritable: true,
        }, {
            // This is the public key of the account where the funds
            // and metadata are held.
            pubkey: new sol.PublicKey(accountAddr),
            isSigner: false,
            isWritable: true,
        }, {
            // This is the system program public key.
            pubkey: sol.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        }],
        programId: new sol.PublicKey(programAddr),
        data: data,
    });

    // Transaction signed by Alice.
    tx = new sol.Transaction().add(instruction);
    return await sol.sendAndConfirmTransaction(connection, tx, [alice]);
}

async function main(args) {
    if (process.argv.length < 3 || process.argv.length > 4) {
        usage();
    }

    const conn = new sol.Connection(cluster);

    switch (args[2]) {
    case "init":
        console.log("TXID:", await initStream(conn));
        break;
    case "withdraw":
        if (args.length != 4) {
            console.log("Missing metadata/funds account address");
            usage();
        }
        console.log("TXID:", await withdrawStream(conn, args[3]));
        break;
    case "cancel":
        if (args.length != 4) {
            console.log("Missing metadata/funds account address");
            usage();
        }
        console.log("TXID:", await cancelStream(conn, args[3]));
        break;
    default:
        usage();
    }
}

main(process.argv).then(() => process.exit(0)).catch(e => console.error(e));
