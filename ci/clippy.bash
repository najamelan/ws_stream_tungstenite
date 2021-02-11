#!/usr/bin/bash

# fail fast
#
set -e

# print each command before it's executed
#
set -x

touch **.rs
cargo clippy --tests --examples --benches --all-features
