use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn nasm(file: &Path, arch: &str, out_file: &Path) -> Command {
    let oformat = match arch {
        "x86_64" => ("elf64"),
        "x86" => ("elf32"),
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
        Path::new("asm/tdcall.o"),
    ));
    run_command(nasm(
        Path::new("asm/Tdvmcall.nasm"),
        "x86_64",
        Path::new("asm/tdvmcall.o"),
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
    c.object(Path::new("asm/tdcall.o"));
    c.object(Path::new("asm/tdvmcall.o"));
    let _ = c.cargo_metadata(false);
    c.flag("-Wl,--gc-sections");
    c.compile(
        lib_path
            .file_name()
            .and_then(|f| f.to_str())
            .expect("No filename"),
    );

    // TBD:
    // target\target\debug\build\rust-td-payload-3a4201d474ab9ecf\out\libtdcall.a target\target\debug\deps\libtdcall.a /y
    // and build again.
    //
    println!("cargo:rustc-link-lib=static={}", "tdcall");
}
