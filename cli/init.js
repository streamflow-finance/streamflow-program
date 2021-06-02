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
    // * instruction (0 = init stream)
    // * start_time
    // * end_time
    // * SOL amount

    // Packed as little endian
    const layout = BufferLayout.struct([
        BufferLayout.u8("instruction"),
        BufferLayout.u32("starttime"),
        BufferLayout.u32("endtime"),
        // N.B. JS Number has 53 significant bits, so numbers larger than
        // 2^53 can be misrepresented
        BufferLayout.nu64("amount")
    ]);

    now = Math.floor(new Date().getTime() / 1000);
    console.log(now);

    const data = Buffer.alloc(layout.span);
    layout.encode({
            instruction: 0, // 0: init
            starttime: now,
            endtime: now + 600,
            // amount: Number.MAX_SAFE_INTEGER // limited to 2^53 = 9007199254740992
            amount: 1000000,
        },
        data
    );


    // UInt64 alternative is to remove the "amount" from layout encoding and
    // use the following code:
    // //data.writeBigUInt64LE(BigInt("18446744073709551615"), 9)

    return data;
}

async function main (programAddress) {
    const connection = new sol.Connection("http://localhost:8899");

    // Alice is our sender, make sure there's funds in the account
    const alice = readKeypairFromPath("./alice.json");
    // await connection.requestAirdrop(alice.publicKey, 1000000000);

    // Bob is our recipient
    const bob = readKeypairFromPath("./bob.json");

    // pda is a new keypair where funds are sent, and program metadata
    // is kept and updated by the program
    const pda = new sol.Keypair();

    console.log("ALICE: %s", alice.publicKey.toBase58());
    console.log("BOB: %s", bob.publicKey.toBase58());
    console.log("PDA: %s", pda.publicKey.toBase58());

    var data = await encodeProgramData();
    console.log("DATA:", data);

    const instruction = new sol.TransactionInstruction({
        keys: [{
            pubkey: alice.publicKey,
            isSigner: true,
            isWritable: true
        }, {
            pubkey: bob.publicKey,
            isSigner: false,
            isWritable: true
        }, {
            pubkey: pda.publicKey,
            isSigner: true,
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
        connection, tx, [alice, pda]);

    console.log("TXID: %s", confirmation);
}

if (process.argv.length != 3) {
    console.log("usage: ./init.js ProgramAddress");
    process.exit(1);
}

main(process.argv[2]).then(() => process.exit(0)).catch(err => console.error(err))