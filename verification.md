Verifying the deployed program
==============================

To confirm the deployed program corresponds to this source code, it
is possible to dump the ELF from the Solana chain, and compare it to
the binary built from this very source code.

The sequence for doing this on a Linux box with installed Solana
command-line tools should be something like the following. 


Clone and build the program from source
---------------------------------------

```
% git clone https://github.com/streamflow-finance/streamflow-program
% cd streamflow-program
% git checkout v0.1.1  # Check the version in Cargo.toml
% cargo build-bpf
```

Dump the program from the Solana chain
--------------------------------------

```
% solana program -u devnet dump 2DvvSEde36Ch3B52g9hKWDYbfmJimLpJwVBV9Cknypi4 onchain.elf
```

Verify that the two binaries are the same
-----------------------------------------

The "issue" here is that the on-chain programs have trailing zeroes to
allow further program upgrades. This means in theory, the deployed
binary is different than the local one because of these. To mitigate
this, we can append enough zeroes to our locally-compiled program in
order to get the same result.

We first find the byte sizes of our binaries:

```
% size_of_chain_elf="$(wc -c onchain.elf)"
% size_of_local_elf="$(wc -c target/deploy/streamflow.so)"
```

Having this, we can calculate how many zeroes are missing from our
local binary. In this case, let's say there are 186808 extra bytes in
the on-chain binary.

```
% echo "$size_of_chain_elf - $size_of_local_elf" | bc
186808
```

We should now be able to add enough zeroes to our local ELF and
get the same checksum like the deployed program:

```
% dd if=/dev/zero of=./target/deploy/streamflow.so bs=1 count=186808 oflag=append conv=notrunc
% sha256sum onchain.elf target/deploy/streamflow.so
0233c4b1b464fa0134a18c8b7564272dcb892c98002a8f1a56fdce9bbb21f646  onchain.elf
0233c4b1b464fa0134a18c8b7564272dcb892c98002a8f1a56fdce9bbb21f646  target/deploy/streamflow.so
```

Looks like it's the exact same program! :)
