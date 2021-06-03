#!/bin/sh
set -e

rm -rf test-ledger
tmux new-session -d "solana-test-validator"
sleep 1
tmux split-window -v "solana logs"
tmux a
