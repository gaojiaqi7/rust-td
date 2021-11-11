### Build Cloud-Hypervisor
An example using bzImage for TD-Shim is available in:
https://github.com/gaojiaqi7/cloud-hypervisor/tree/tdx-tdshim
```
cargo build --features "fwdebug,tdx"
```
Binary can be found at target/debug/cloud-hypervisor

### Build TDX Linux kernel
```
https://github.com/intel/tdx/tree/guest
```

### Build TD-Shim Image
```
cargo xbuild -p rust-tdshim --target x86_64-unknown-uefi --release
pushd rust-td-payload
cargo xbuild --target target.json --release
popd
cargo run -p rust-td-tool --features "boot-kernel" -- target/x86_64-unknown-uefi/release/ResetVector.bin target/x86_64-unknown-uefi/release/rust-tdshim.efi target/target//release/rust-td-payload target/x86_64-unknown-uefi/release/final.bin
```

## Run
```
BIOS=final.bin
GUEST_IMG=./td-guest-centos8.4.qcow2
KERNEL=./bzImage
./cloud-hypervisor -v --tdx firmware=${BIOS} --memory size=2G --cpus boot=1 --kernel ${KERNEL} --disk path=${GUEST_IMG} --cmdline "console=hvc0 root=/dev/vda3 rw"
```
