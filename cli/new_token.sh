#!/bin/sh
set -e

token_addr="$(spl-token create-token \
	| grep 'Creating token' | cut -d' ' -f3)"

token_acct="$(spl-token create-account "$token_addr" \
	| grep 'Creating account' | cut -d' ' -f3)"

spl-token mint "$token_addr" 1100

# Send to Alice from strfi.js
spl-token transfer --fund-recipient \
	"$token_addr" 1000 71G4rRM4DugVRmAwEUtBNaw8xwGKZmSujwjFy37ErphW
