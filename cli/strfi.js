#!/usr/bin/env node

const sol = require("@solana/web3.js");
const BufferLayout = require("buffer-layout");

// Alice is our sender, make sure there is funds in the account
const alice = sol.Keypair.fromSecretKey(Buffer.from([97, 93, 122, 16, 225, 220, 239, 230, 206, 134, 241, 223, 228, 135, 202, 29, 7, 124, 108, 250, 96, 12, 103, 91, 103, 95, 201, 25, 156, 18, 98, 149, 89, 55, 40, 62, 196, 151, 180, 107, 249, 9, 23, 53, 215, 63, 170, 57, 173, 9, 36, 82, 233, 112, 55, 16, 15, 247, 47, 250, 115, 98, 210, 129]));
// await connection.requestAirdrop(alice.publicKey, 1000000000);

// Bob is our recipient
const bob = sol.Keypair.fromSecretKey(Buffer.from([104, 59, 250, 44, 167, 108, 233, 202, 30, 232, 3, 91, 108, 141, 125, 241, 216, 86, 189, 157, 48, 69, 78, 98, 125, 6, 150, 127, 41, 214, 124, 242, 238, 189, 58, 189, 215, 194, 98, 74, 98, 184, 196, 38, 158, 174, 51, 135, 76, 147, 74, 61, 214, 178, 94, 233, 190, 216, 78, 115, 83, 39, 99, 226]));

// This is the structure for the init instruction
const initLayout = BufferLayout.struct([
    BufferLayout.u8("instruction"),
    BufferLayout.u32("starttime"),
    BufferLayout.u32("endtime"),
    // N.B. JS Number has 53 significant bits, so numbers harger than
    // 2^53 can be misrepresented
    BufferLayout.nu64("amount"),
]);

// This is the structure for the withdraw instruction
const withdrawLayout = BufferLayout.struct([
    BufferLayout.u8("instruction"),
    BufferLayout.nu64("amount"),
]);

// This is the structure for the cancel instruction
const cancelLayout = BufferLayout.struct([
    BufferLayout.u8("instruction"),
]);

async function cancelStream(connection, programAddress, accountAddress) {
    var data = Buffer.alloc(cancelLayout.span);
    cancelLayout.encode({
            instruction: 2,
        },
        data,
    );

    console.log("DATA:", data);

    const instruction = new sol.TransactionInstruction({
        keys: [{
            pubkey: alice.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            pubkey: new sol.PublicKey(accountAddress),
            isSigner: false,
            isWritable: true,
        }, {
            pubkey: sol.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        }],
        programId: new sol.PublicKey(programAddress),
        data: data,
    });

    tx = new sol.Transaction().add(instruction);
    return await sol.sendAndConfirmTransaction(connection, tx, [alice]);
}

async function withdrawStream(connection, programAddress, accountAddress) {
    var data = Buffer.alloc(withdrawLayout.span);
    withdrawLayout.encode({
            instruction: 1,
            amount: 160000,
        },
        data,
    );

    console.log("DATA:", data);

    const instruction = new sol.TransactionInstruction({
        keys: [{
            pubkey: bob.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            pubkey: new sol.PublicKey(accountAddress),
            isSigner: false,
            isWritable: true,
        }, {
            pubkey: sol.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        }],
        programId: new sol.PublicKey(programAddress),
        data: data,
    });

    tx = new sol.Transaction().add(instruction);
    return await sol.sendAndConfirmTransaction(connection, tx, [bob]);
}

async function initStream(connection, programAddress) {
    // pda is a new keypair where funds are sent, and program metadata
    // is kept and updated by the program
    const pda = new sol.Keypair();
    console.log("PDA:   %s", pda.publicKey.toBase58());

    now = Math.floor(new Date().getTime() / 1000);

    var data = Buffer.alloc(initLayout.span);
    initLayout.encode({
            instruction: 0,
            starttime: now,
            endtime: now + 600,
            amount: 10000000
        },
        data,
    );

    console.log("DATA:", data);

    const instruction = new sol.TransactionInstruction({
        keys: [{
            pubkey: alice.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            pubkey: bob.publicKey,
            isSigner: false,
            isWritable: true,
        }, {
            pubkey: pda.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            pubkey: sol.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        }],
        programId: new sol.PublicKey(programAddress),
        data: data,
    });

    tx = new sol.Transaction().add(instruction);
    return await sol.sendAndConfirmTransaction(connection, tx, [alice, pda]);

}

async function main(ix, programAddress, accountAddress) {
    const conn = new sol.Connection("http://localhost:8899");
    console.log("ALICE: %s", alice.publicKey.toBase58());
    console.log("BOB:   %s", bob.publicKey.toBase58());

    if (ix == "init") {
        confirmation = await initStream(conn, programAddress);
    } else if (ix == "withdraw" && accountAddress != "") {
        confirmation = await withdrawStream(conn, programAddress, accountAddress);
    } else if (ix == "cancel" && accountAddress != "") {
        confirmation = await cancelStream(conn, programAddress, accountAddress);
    } else {
        usage();
    }

    console.log("TXID:  %s", confirmation);
}

function usage() {
    console.log("usage: strfi.js [init|withdraw|cancel] [programAddress] [accountAddress]");
    console.log("ex:");
    console.log("strfi.js init     CQg4Kcd285oMPzEHubEHny3ncqt9omWmNXmCTttnfVbp");
    console.log("strfi.js withdraw CQg4Kcd285oMPzEHubEHny3ncqt9omWmNXmCTttnfVbp BDeKFWwL7zsFHKphFEHjEmcpk9twKgUgepytyA44Ta6e");
    console.log("strfi.js cancel   CQg4Kcd285oMPzEHubEHny3ncqt9omWmNXmCTttnfVbp BDeKFWwL7zsFHKphFEHjEmcpk9twKgUgepytyA44Ta6e");
    process.exit(1);
}

if (process.argv.length < 4 && process.argv.length > 5) {
    usage();
}

var accAddr = "";
if (process.argv.length == 5) {
    accAddr = process.argv[4];
}

main(process.argv[2], process.argv[3], accAddr).then(
    () => process.exit(0)).catch(err => console.error(err));
