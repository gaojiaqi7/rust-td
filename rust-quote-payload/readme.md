## Build Rust Td

### Tools

1. Install [RUST](https://www.rust-lang.org/)

please use nightly-2020-11-09.

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
pushd rust-quote-payload
cargo mbuild --release
popd
cargo run -p rust-td-tool -- target/x86_64-unknown-uefi/release/ResetVector.bin target/x86_64-unknown-uefi/release/rust-tdshim.efi target/x86_64-unknown-uefi/release/rust-quote-payload.efi target/x86_64-unknown-uefi/release/final.bin
```

### Build Elf format payload
```
pushd rust-quote-payload
cargo xbuild --target target.json --release
popd
cargo run -p rust-td-tool -- target/x86_64-unknown-uefi/release/ResetVector.bin target/x86_64-unknown-uefi/release/rust-tdshim.efi target/target/release/rust-quote-payload target/x86_64-unknown-uefi/release/final.bin
```


## How to run

```
QEMU=/usr/libexec/qemu-kvm
BIOS=[final.bin location]/final.bin
now=$(date +"%m%d_%H%M")
LOGFILE=stdout.${now}.log
MEMORY_SIZE=2M
$QEMU \
  -accel kvm \
  -name process=rust-td,debug-threads=on \
  -smp 1,sockets=1 \
  -object tdx-guest,id=tdx,debug=on \
  -machine q35,kvm-type=tdx,pic=no,kernel_irqchip=split,confidential-guest-support=tdx \
  -no-hpet \
  -cpu host,pmu=off,-kvm-steal-time \
  -device loader,file=$BIOS,id=fd0 \
  -device vhost-vsock-pci,id=vhost-vsock-pci1,guest-cid=33 \
  -m $MEMORY_SIZE -nographic -vga none -nic none \
  -serial mon:stdio
```
