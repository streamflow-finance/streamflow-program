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
    // * instruction (1 = withdraw)
    // * SOL amount

    // Packed as little endian
    const layout = BufferLayout.struct([
        BufferLayout.u8("instruction"),
        // N.B. JS Number has 53 significant bits, so numbers larger than
        // 2^53 can be misrepresented
        BufferLayout.nu64("amount")
    ]);

    const data = Buffer.alloc(layout.span);
    layout.encode({
            instruction: 1, // 1: withdraw
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

async function main (programAddress, accountAddress) {
    const connection = new sol.Connection("http://localhost:8899");

    // Bob is our recipient
    const bob = readKeypairFromPath("./bob.json");

    console.log("BOB: %s", bob.publicKey.toBase58());

    var data = await encodeProgramData();
    console.log(data);

    const instruction = new sol.TransactionInstruction({
        keys: [{
            pubkey: bob.publicKey,
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
        connection, tx, [bob]);

    console.log("TXID: %s", confirmation);
}

if (process.argv.length != 4) {
    console.log("usage: ./withdraw.js ProgramAddress AccountAddress");
    process.exit(1);
}

main(process.argv[2], process.argv[3]).then(() => process.exit(0)).catch(err => console.error(err))