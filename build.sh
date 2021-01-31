#!/bin/bash

DEFAULTVERSION="0.0.0"
VERSION="${1:-$DEFAULTVERSION}"
TARGET=x86_64-unknown-linux-musl

rustup target add ${TARGET}
cargo build --release --target=${TARGET} --locked
zip --junk-path /mnt/r/noteexplorer-linux-x64-${VERSION}.zip target/${TARGET}/release/noteexplorer
