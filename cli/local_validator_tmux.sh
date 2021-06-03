#!/bin/sh
set -e

rm -rf test-ledger
tmux new-session -d "solana-test-validator --limit-ledger-size 999999"
sleep 1
tmux split-window -v "solana logs"
tmux a
