#!/usr/bin/env node

const sol = require("@solana/web3.js");
const BufferLayout = require("buffer-layout");
const fs = require("fs");

function readKeypairFromPath (path) {
    const data = JSON.parse(fs.readFileSync(path, "utf-8"))
    return sol.Keypair.fromSecretKey(Buffer.from(data))
}

async function encodeProgramData () {
    // Initialize a struct containing:
    // * instruction (2 = withdraw)

    // Packed as little endian
    const layout = BufferLayout.struct([
        BufferLayout.u8("instruction"),
    ]);

    const data = Buffer.alloc(layout.span);
    layout.encode({
            instruction: 2, // 2: cancel
        },
        data
    );

    return data;
}

async function main (programAddress, accountAddress) {
    const connection = new sol.Connection("http://localhost:8899");

    // Alice is our sender who did the initialization.
    const alice = readKeypairFromPath("./alice.json");

    console.log("ALICE: %s", alice.publicKey.toBase58());

    var data = await encodeProgramData();
    console.log(data);

    const instruction = new sol.TransactionInstruction({
        keys: [{
            pubkey: alice.publicKey,
            isSigner: true,
            isWritable: true
        }, {
            pubkey: new sol.PublicKey(accountAddress),
            isSigner: false,
            isWritable: true
        }, {
            pubkey: sol.SystemProgram.programId,
            isSigner: false,
            isWritable: false
        }],
        programId: new sol.PublicKey(programAddress),
        data: data,
    });

    tx = new sol.Transaction().add(instruction);

    const confirmation = await sol.sendAndConfirmTransaction(
        connection, tx, [alice]);

    console.log("TXID: %s", confirmation);
}

if (process.argv.length != 4) {
    console.log("usage: ./cancel.js ProgramAddress AccountAddress");
    process.exit(1);
}

main(process.argv[2], process.argv[3]).then(() => process.exit(0)).catch(err => console.error(err))