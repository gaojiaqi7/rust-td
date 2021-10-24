# rust-td

A demo for pure rust based td-shim.

It is derived from https://github.com/jyao1/edk2-staging/tree/TdShim/TdShimPkg.

## tdx-tdcall

tdx-tdcall impl two ways:

you can edit Cargo.toml -> features -> default to enable or disable. See

1. tdx call

```
default = []
```

2. tdx emulate

```
default = ["use_tdx_emulation"]
```

## How to build

### Tools

1. Install [RUST](https://www.rust-lang.org/)

please use nightly-2021-08-20.

1.1. Intall xbuild

```
cargo install cargo-xbuild
```

Please reinstall cargo-xbuild, after you update the rust toolchain.

2. Install [NASM](https://www.nasm.us/)

Please make sure nasm can be found in PATH.

3. Install LLVM

Please make sure clang can be found in PATH.

Set env:

```
set CC=clang
set AR=llvm-ar
```

### Build TdShim
```
cargo xbuild -p rust-tdshim --target x86_64-unknown-uefi --release
```

### Build PE format payload
```
pushd rust-td-payload
cargo xbuild --target x86_64-unknown-uefi --release
popd
cargo run -p rust-td-tool -- target/x86_64-unknown-uefi/release/ResetVector.bin target/x86_64-unknown-uefi/release/rust-tdshim.efi target/x86_64-unknown-uefi/release/rust-td-payload.efi target/x86_64-unknown-uefi/release/final.bin
```

### Build Elf format payload
```
pushd rust-td-payload
cargo xbuild --target target.json --release
popd
cargo run -p rust-td-tool -- target/x86_64-unknown-uefi/release/ResetVector.bin target/x86_64-unknown-uefi/release/rust-tdshim.efi target/target//release/rust-td-payload target/x86_64-unknown-uefi/release/final.bin
```

## Run
REF: https://github.com/tianocore/edk2-staging/tree/TDVF

```
./launch-rust-td.sh
```

## Code Contributions

1.  install [pre-commit](https://pre-commit.com/#install)
2.  run ```pre-commit install```
3.  when you run ```git commit```, pre-commit will do check-code things.

## Known limitation
This package is only the sample code to show the concept. It does not have a full validation such as robustness functional test and fuzzing test. It does not meet the production quality yet. Any codes including the API definition, the libary and the drivers are subject to change.
