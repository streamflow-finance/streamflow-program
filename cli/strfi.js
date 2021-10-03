#!/usr/bin/env node
 // This file serves as a reference on how to use streamflow-program with
// Javascript.
const BufferLayout = require("buffer-layout");
const sol = require("@solana/web3.js");
const spl = require("@solana/spl-token");
const fs = require("fs");

const cluster = "http://localhost:8899";
const programAddr = "EZZVHC2vtXsP2CZ5r8HM25PHcWj6Et1joqsZ6zxY6GRG";
const tokenMint = "89LDQH8DPKsNopU1nPpkodHzFyH4dvCBEMXcWuFqSeLX";

function usage() {
    console.log("usage: strfi.js [initnative|withdrawnative|cancelnative] [accountAddress (needed for withdraw/cancel)]");
    console.log("usage: strfi.js [inittoken|withdrawtoken|canceltoken] [metadataAddress] [escrowAddress]");
    process.exit(1);
}

// 71G4rRM4DugVRmAwEUtBNaw8xwGKZmSujwjFy37ErphW
let alice = sol.Keypair.fromSecretKey(Buffer.from([97, 93, 122, 16, 225,
220, 239, 230, 206, 134, 241, 223, 228, 135, 202, 29, 7, 124, 108, 250,
96, 12, 103, 91, 103, 95, 201, 25, 156, 18, 98, 149, 89, 55, 40, 62, 196,
151, 180, 107, 249, 9, 23, 53, 215, 63, 170, 57, 173, 9, 36, 82, 233, 112,
55, 16, 15, 247, 47, 250, 115, 98, 210, 129]));

// H4wPUkepkJgB2FMaRyZWvsSpNUK8exoMonbRgRsipisb
let bob = sol.Keypair.fromSecretKey(Buffer.from([104, 59, 250, 44, 167,
108, 233, 202, 30, 232, 3, 91, 108, 141, 125, 241, 216, 86, 189, 157, 48,
69, 78, 98, 125, 6, 150, 127, 41, 214, 124, 242, 238, 189, 58, 189, 215,
194, 98, 74, 98, 184, 196, 38, 158, 174, 51, 135, 76, 147, 74, 61, 214,
178, 94, 233, 190, 216, 78, 115, 83, 39, 99, 226]));

// This is the structure for the init instruction
const initLayout = BufferLayout.struct([
    BufferLayout.u8("instruction"),
    BufferLayout.blob(8, "starttime"),
    BufferLayout.blob(8, "endtime"),
    BufferLayout.blob(8, "amount"),
    BufferLayout.blob(8, "period"),
    BufferLayout.blob(8, "cliff"),
    BufferLayout.blob(8, "cliff_amount"),
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

async function initNativeStream(connection) {
    // Current time as Unix timestamp
    now = Math.floor(new Date().getTime() / 1000);

    var data = Buffer.alloc(initLayout.span);
    initLayout.encode({
            // 0 means init in the rust program
            instruction: 0,
            // Unix timestamp when the stream should start unlocking
            starttime: new spl.u64(now + 10).toBuffer(),
            // Unix timestamp when the stream should finish and unlock everything
            endtime: new spl.u64(now + 610).toBuffer(),
            // Lamports to stream
            amount: new spl.u64(100000000).toBuffer(),
            // Time step per which vesting occurs
            period: new spl.u64(1).toBuffer(),
            // Vesting contract cliff_timestamp
            cliff: new spl.u64(0).toBuffer(),
            // Amount unlocked at cliff timestamp
            cliff_amount: new spl.u64(0).toBuffer(),
        },
        data,
    );

    const escrow = new sol.Keypair();

    console.log("ALICE: %s", alice.publicKey.toBase58());
    console.log("BOB:   %s", bob.publicKey.toBase58());
    console.log("ESCRW: %s", escrow.publicKey.toBase58());
    console.log("DATA:", data);

    const instruction = new sol.TransactionInstruction({
        keys: [{
            // Alice is the stream sender
            pubkey: alice.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            // Bob is the stream recipient
            pubkey: bob.publicKey,
            isSigner: false,
            isWritable: true,
        }, {
            // Escrow account with metadata and funds
            pubkey: escrow.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            // System program
            pubkey: sol.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        }],
        programId: new sol.PublicKey(programAddr),
        data: data,
    });

    // Transaction signed by Alice and the escrow account
    tx = new sol.Transaction().add(instruction);
    return await sol.sendAndConfirmTransaction(connection, tx, [alice, escrow]);
}

async function withdrawNativeStream(connection, accountAddr) {
    var data = Buffer.alloc(withdrawLayout.span);
    withdrawLayout.encode({
            // 1 means withdraw in the rust program
            instruction: 1,
            // When amount is 0, withdraw all available
            // Otherwise, arbitrary values are allowed
            amount: new spl.u64(0).toBuffer(),
        },
        data,
    );

    console.log("ALICE: %s", alice.publicKey.toBase58());
    console.log("BOB:   %s", bob.publicKey.toBase58());
    console.log("ESCRW: %s", accountAddr);
    console.log("DATA:", data);

    const instruction = new sol.TransactionInstruction({
        keys: [{
            // Alice is the stream initializer
            pubkey: alice.publicKey,
            isSigner: false,
            isWritable: true,
        }, {
            // Bob is the stream recipient
            pubkey: bob.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            // Escrow account
            pubkey: new sol.PublicKey(accountAddr),
            isSigner: false,
            isWritable: true,
        }],
        programId: new sol.PublicKey(programAddr),
        data: data,
    });

    // Transaction signed by Bob
    tx = new sol.Transaction().add(instruction);
    return await sol.sendAndConfirmTransaction(connection, tx, [bob]);
}

async function cancelNativeStream(connection, accountAddr) {
    var data = Buffer.alloc(cancelLayout.span);
    cancelLayout.encode({
            // 2 means cancel in the rust program
            instruction: 2,
        },
        data,
    );

    console.log("ALICE: %s", alice.publicKey.toBase58());
    console.log("BOB:   %s", bob.publicKey.toBase58());
    console.log("ESCRW: %s", accountAddr);
    console.log("DATA:", data);

    const instruction = new sol.TransactionInstruction({
        keys: [{
            // Alice is the stream initializer
            pubkey: alice.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            // Bob is the stream recipient
            pubkey: bob.publicKey,
            isSigner: false,
            isWritable: true,
        }, {
            // The escrow account
            pubkey: new sol.PublicKey(accountAddr),
            isSigner: false,
            isWritable: true,
        }],
        programId: new sol.PublicKey(programAddr),
        data: data,
    });

    // Transaction signed by Alice
    tx = new sol.Transaction().add(instruction);
    return await sol.sendAndConfirmTransaction(connection, tx, [alice]);
}

async function initTokenStream(connection) {
    // Current time as Unix timestamp
    now = Math.floor(new Date().getTime() / 1000);

    var data = Buffer.alloc(initLayout.span);
    initLayout.encode({
            // 3 means spl token init in the rust program
            instruction: 3,
            // Unix timestamp when the stream should start unlocking
            starttime: new spl.u64(now + 10).toBuffer(),
            // Unix timestamp when the stream should finish and unlock everything
            endtime: new spl.u64(now + 610).toBuffer(),
            // Tokens*decimals to stream
            amount: new spl.u64(10000000000).toBuffer(),
            // Time step per which vesting occurs
            period: new spl.u64(1).toBuffer(),
            // Vesting contract cliff_timestamp
            cliff: new spl.u64(0).toBuffer(),
            // Amount unlocked at cliff timestamp
            cliff_amount: new spl.u64(0).toBuffer(),
        },
        data,
    );

    const metadata = new sol.Keypair();

    let [escrow, number] = await sol.PublicKey.findProgramAddress(
        [metadata.publicKey.toBuffer()], new sol.PublicKey(programAddr));

    const alice_tokens = await spl.Token.getAssociatedTokenAddress(
        spl.ASSOCIATED_TOKEN_PROGRAM_ID,
        spl.TOKEN_PROGRAM_ID,
        new sol.PublicKey(tokenMint),
        alice.publicKey,
    );

    const bob_tokens = await spl.Token.getAssociatedTokenAddress(
        spl.ASSOCIATED_TOKEN_PROGRAM_ID,
        spl.TOKEN_PROGRAM_ID,
        new sol.PublicKey(tokenMint),
        bob.publicKey,
    );

    console.log("ALICE: %s", alice.publicKey.toBase58());
    console.log("BOB:   %s", bob.publicKey.toBase58());
    console.log("META:  %s", metadata.publicKey.toBase58());
    console.log("ESCRW: %s", escrow);
    console.log("MINT:  %s", tokenMint);
    console.log("DATA:", data);

    const instruction = new sol.TransactionInstruction({
        keys: [{
            // Alice is the stream sender
            pubkey: alice.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            // Alice associated token account
            pubkey: alice_tokens,
            isSigner: false,
            isWritable: true,
        }, {
            // Bob is the stream recipient
            pubkey: bob.publicKey,
            isSigner: false,
            isWritable: true,
        }, {
            // Bob associated token account
            pubkey: bob_tokens,
            isSigner: false,
            isWritable: true,
        }, {
            // Metadata account with data
            pubkey: metadata.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            // Escrow account with funds
            pubkey: escrow,
            isSigner: false,
            isWritable: true,
        }, {
            // Token mint
            pubkey: new sol.PublicKey(tokenMint),
            isSigner: false,
            isWritable: false,
        }, {
            // Sysvar rent
            pubkey: sol.SYSVAR_RENT_PUBKEY,
            isSigner: false,
            isWritable: false,
        }, {
            // timelock program
            pubkey: new sol.PublicKey(programAddr),
            isSigner: false,
            isWritable: false,
        }, {
            // token program
            pubkey: spl.TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
        }, {
            // associated token program
            pubkey: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
        }, {
            // System program
            pubkey: sol.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        }],
        programId: new sol.PublicKey(programAddr),
        data: data,
    });

    // Transaction signed by Alice and the escrow account
    tx = new sol.Transaction().add(instruction);
    return await sol.sendAndConfirmTransaction(connection, tx, [alice, metadata]);
}

async function withdrawTokenStream(connection, metadataAddr) {
    var data = Buffer.alloc(withdrawLayout.span);
    withdrawLayout.encode({
            // 4 means withdraw spl token in the rust program
            instruction: 4,
            // When amount is 0, withdraw all available
            // Otherwise, arbitrary values are allowed
            amount: new spl.u64(0).toBuffer(),
        },
        data,
    );

    const alice_tokens = await spl.Token.getAssociatedTokenAddress(
        spl.ASSOCIATED_TOKEN_PROGRAM_ID,
        spl.TOKEN_PROGRAM_ID,
        new sol.PublicKey(tokenMint),
        alice.publicKey,
    );

    const bob_tokens = await spl.Token.getAssociatedTokenAddress(
        spl.ASSOCIATED_TOKEN_PROGRAM_ID,
        spl.TOKEN_PROGRAM_ID,
        new sol.PublicKey(tokenMint),
        bob.publicKey,
    );

    let metadata_pubkey = new sol.PublicKey(metadataAddr);
    let [escrow, number] = await sol.PublicKey.findProgramAddress(
        [metadata_pubkey.toBuffer()], new sol.PublicKey(programAddr));

    const instruction = new sol.TransactionInstruction({
        keys: [{
            // Alice is the stream initializer
            pubkey: alice.publicKey,
            isSigner: false,
            isWritable: true,
        }, {
            // Alice associated token account
            pubkey: alice_tokens,
            isSigner: false,
            isWritable: true,
        }, {
            // Bob is the stream recipient
            pubkey: bob.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            // Bob associated token account
            pubkey: bob_tokens,
            isSigner: false,
            isWritable: true,
        }, {
            // Metadata account with data
            pubkey: metadata_pubkey,
            isSigner: false,
            isWritable: true,
        }, {
            // Escrow account with funds
            pubkey: escrow,
            isSigner: false,
            isWritable: true,
        }, {
            // Mint account
            pubkey: new sol.PublicKey(tokenMint),
            isSigner: false,
            isWritable: false,
        }, {
            // Sysvar rent
            pubkey: sol.SYSVAR_RENT_PUBKEY,
            isSigner: false,
            isWritable: false,
        }, {
            // timelock program
            pubkey: new sol.PublicKey(programAddr),
            isSigner: false,
            isWritable: true,
        }, {
            // token program
            pubkey: spl.TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
        }, {
            // System program
            pubkey: sol.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        }],
        programId: new sol.PublicKey(programAddr),
        data: data,
    });

    // Transaction signed by Bob
    tx = new sol.Transaction().add(instruction);
    return await sol.sendAndConfirmTransaction(connection, tx, [bob]);
}

async function cancelTokenStream(connection, metadataAddr) {
    var data = Buffer.alloc(withdrawLayout.span);
    withdrawLayout.encode({
            // 4 means withdraw spl token in the rust program
            instruction: 5,
        },
        data,
    );

    const alice_tokens = await spl.Token.getAssociatedTokenAddress(
        spl.ASSOCIATED_TOKEN_PROGRAM_ID,
        spl.TOKEN_PROGRAM_ID,
        new sol.PublicKey(tokenMint),
        alice.publicKey,
    );

    const bob_tokens = await spl.Token.getAssociatedTokenAddress(
        spl.ASSOCIATED_TOKEN_PROGRAM_ID,
        spl.TOKEN_PROGRAM_ID,
        new sol.PublicKey(tokenMint),
        bob.publicKey,
    );

    let metadata_pubkey = new sol.PublicKey(metadataAddr);
    let [escrow, number] = await sol.PublicKey.findProgramAddress(
        [metadata_pubkey.toBuffer()], new sol.PublicKey(programAddr));

    const instruction = new sol.TransactionInstruction({
        keys: [{
            // Alice is the stream initializer
            pubkey: alice.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            // Alice associated token account
            pubkey: alice_tokens,
            isSigner: false,
            isWritable: true,
        }, {
            // Bob is the stream recipient
            pubkey: bob.publicKey,
            isSigner: false,
            isWritable: true,
        }, {
            // Bob associated token account
            pubkey: bob_tokens,
            isSigner: false,
            isWritable: true,
        }, {
            // Metadata account with data
            pubkey: metadata_pubkey,
            isSigner: false,
            isWritable: true,
        }, {
            // Escrow account with funds
            pubkey: escrow,
            isSigner: false,
            isWritable: true,
        }, {
            // Mint account
            pubkey: new sol.PublicKey(tokenMint),
            isSigner: false,
            isWritable: false,
        }, {
            // timelock program
            pubkey: new sol.PublicKey(programAddr),
            isSigner: false,
            isWritable: true,
        }, {
            // token program
            pubkey: spl.TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
        }, {
            // System program
            pubkey: sol.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        }],
        programId: new sol.PublicKey(programAddr),
        data: data,
    });

    // Transaction signed by Alice
    tx = new sol.Transaction().add(instruction);
    return await sol.sendAndConfirmTransaction(connection, tx, [alice]);
}


async function main(args) {
    if (process.argv.length < 3 || process.argv.length > 4) {
        usage();
    }

    const conn = new sol.Connection(cluster);

    switch (args[2]) {
    case "initnative":
        console.log("TXID:", await initNativeStream(conn));
        break;
    case "withdrawnative":
        if (args.length != 4) {
            console.log("Missing metadata/escrow account address");
            usage();
        }
        console.log("TXID:", await withdrawNativeStream(conn, args[3]));
        break;
    case "cancelnative":
        if (args.length != 4) {
            console.log("Missing metadata/escrow account address");
            usage();
        }
        console.log("TXID:", await cancelNativeStream(conn, args[3]));
        break;
    case "inittoken":
        console.log("TXID:", await initTokenStream(conn));
        break;
    case "withdrawtoken":
        if (args.length != 4) {
            console.log("Missing metadata account address");
            usage();
        }
        console.log("TXID:", await withdrawTokenStream(conn, args[3]));
        break;
    case "canceltoken":
        if (args.length != 4) {
            console.log("Missing metadata account address");
            usage();
        }
        console.log("TXID:", await cancelTokenStream(conn, args[3]));
        break;
    default:
        usage();
    }
}

main(process.argv).then(() => process.exit(0)).catch(e => console.error(e));
