use std::path::PathBuf;
use std::process::Command;

fn get_cargo_path() -> PathBuf {
    if let Ok(cargo) = std::env::var("CARGO") {
        return PathBuf::from(cargo);
    }
    if let Ok(home) = std::env::var("HOME") {
        let cargo_in_home = PathBuf::from(home).join(".cargo/bin/cargo");
        if cargo_in_home.exists() {
            return cargo_in_home;
        }
    }
    PathBuf::from("cargo")
}

fn main() {
    let cargo = get_cargo_path();
    
    // 1. Build the kernel
    println!("\x1b[1;31m[AvenOS]\x1b[0m Compiling 64-bit kernel (x86_64-unknown-none)...");
    let status = Command::new(&cargo)
        .arg("build")
        .arg("--package")
        .arg("kernel")
        .arg("--target")
        .arg("x86_64-unknown-none")
        .status();

    match status {
        Ok(s) if s.success() => (),
        _ => {
            eprintln!("\x1b[1;33m[Warning]\x1b[0m Could not re-compile kernel via cargo runner, checking existing binary...");
        }
    }

    // 2. Locate kernel ELF and create BIOS bootable disk image
    let kernel_path = PathBuf::from("target/x86_64-unknown-none/debug/kernel");
    if !kernel_path.exists() {
        eprintln!("\x1b[1;31m[Error]\x1b[0m Kernel binary not found at {}", kernel_path.display());
        std::process::exit(1);
    }

    let disk_path = PathBuf::from("target/aven_os_bios.img");

    println!("\x1b[1;31m[AvenOS]\x1b[0m Packaging bootable disk image...");
    let bios = bootloader::BiosBoot::new(&kernel_path);
    bios.create_disk_image(&disk_path)
        .expect("Failed to create BIOS disk image");

    println!("\x1b[1;32m[AvenOS]\x1b[0m Image ready: {}", disk_path.display());

    // 3. Launch QEMU with modern CPU features enabled (SMEP, SMAP, NX)
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--build-only") {
        println!("\x1b[1;32m[AvenOS]\x1b[0m Build-only mode requested. Disk image is ready.");
        return;
    }
    let no_display = args.iter().any(|arg| arg == "--no-display");

    println!("\x1b[1;31m[AvenOS]\x1b[0m Launching QEMU emulator with hardened CPU flags...");
    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.arg("-drive").arg(format!("format=raw,file={}", disk_path.display()));
    qemu.arg("-cpu").arg("max");
    qemu.arg("-serial").arg("stdio");

    if no_display {
        qemu.arg("-display").arg("none");
    }

    let mut child = qemu.spawn().expect("Failed to start QEMU");
    let _ = child.wait();
}
