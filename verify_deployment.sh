#!/bin/sh
# This script can be used to verify that the deployed program on the Solana
# network is the same as the latest git tag in this repository.
#
# See verification.md for an explanation.
#
# It is necessary to have Solana command-line tools and a Rust installation
# to run this.

set -e

latest_tag=$(grep '^version ' Cargo.toml | cut -d' ' -f3 | tr -d'"')
git checkout "$latest_tag"
rm -rf target chain_bin.so
cargo build-bpf

solana program -u devnet dump \
	"2DvvSEde36Ch3B52g9hKWDYbfmJimLpJwVBV9Cknypi4" chain_bin.so

size_of_chain_elf="$(wc -c chain_bin.so | cut -d' ' -f1)"
size_of_local_elf="$(wc -c target/deploy/streamflow.so | cut -d' ' -f1)"

size_diff="$(( size_of_chain_elf - size_of_local_elf ))"

dd if=/dev/zero of=target/deploy/streamflow.so bs=1 count=$size_diff \
	oflag=append conv=notrunc

chain_csum="$(sha256sum chain_bin.so | cut -d' ' -f1)"
local_csum="$(sha256sum target/deploy/streamflow.so | cut -d' ' -f1)"

if [ "$chain_csum" != "$local_csum" ]; then
	cat <<EOM

*****************************************************************
ERROR: Checksums do not match!"
$chain_csum  chain_bin.so
$local_csum  target/deploy/streamflow.so
*****************************************************************
EOM
	exit 1
fi

cat <<EOM

*****************************************************************
SUCCESS: Checksums match!
$chain_csum  chain_bin.so
$local_csum  target/deploy/streamflow.so
*****************************************************************
EOM
