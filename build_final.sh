#!/bin/bash

export CC=clang
export AR=llvm-ar

cargo xbuild -p rust-tdshim --target x86_64-unknown-uefi --release
pushd rust-td-payload
cargo xbuild --target x86_64-unknown-uefi --release
popd
cargo run -p rust-td-tool -- target/x86_64-unknown-uefi/release/ResetVector.bin target/x86_64-unknown-uefi/release/rust-tdshim.efi target/x86_64-unknown-uefi/release/rust-td-payload.efi target/x86_64-unknown-uefi/release/final.bin