#!/usr/bin/env bash

RUSTFLAGS="-Awarnings" cargo run -- --dump "${1:-foc.ctx}"
