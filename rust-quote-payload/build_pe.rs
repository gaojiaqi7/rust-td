use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn nasm(file: &Path, arch: &str, out_file: &Path) -> Command {
    let oformat = match arch {
        "x86_64" => ("win64"),
        "x86" => ("win32"),
        _ => panic!("unsupported arch: {}", arch),
    };
    let mut c = Command::new("nasm");
    let _ = c
        .arg("-o")
        .arg(out_file.to_str().expect("Invalid path"))
        .arg("-f")
        .arg(oformat)
        .arg(file);
    c
}

fn run_command(mut cmd: Command) {
    eprintln!("running {:?}", cmd);
    let status = cmd.status().unwrap_or_else(|e| {
        panic!("failed to execute [{:?}]: {}", cmd, e);
    });
    if !status.success() {
        panic!("execution failed");
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    run_command(nasm(
        Path::new("asm/Tdcall.nasm"),
        "x86_64",
        Path::new("asm/tdcall.obj"),
    ));
    run_command(nasm(
        Path::new("asm/Tdvmcall.nasm"),
        "x86_64",
        Path::new("asm/tdvmcall.obj"),
    ));

    println!(
        "cargo:rerun-if-changed={}",
        Path::new("asm/Tdcall.nasm").to_str().unwrap()
    );
    println!(
        "cargo:rerun-if-changed={}",
        Path::new("asm/Tdvmcall.nasm").to_str().unwrap()
    );

    let lib_path = PathBuf::from("tdcall");
    let mut c = cc::Build::new();
    c.object(Path::new("asm/tdcall.obj"));
    c.object(Path::new("asm/tdvmcall.obj"));
    let _ = c.cargo_metadata(false);
    c.flag("-Wl,--gc-sections");
    c.compile(
        lib_path
            .file_name()
            .and_then(|f| f.to_str())
            .expect("No filename"),
    );

    // TBD:
    // copy target\x86_64-unknown-uefi\debug\build\rust-tdshim-9e492b1606971e11\out\libtdcall.a target\x86_64-unknown-uefi\debug\deps\libtdcall.lib /y
    // and build again.
    //
    println!("cargo:rustc-link-lib=static={}", "libtdcall");
}
