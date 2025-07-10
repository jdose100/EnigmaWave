use std::process::{self, Command};
use std::{env, fs};

fn main() {
    let current_exe = env::current_exe().unwrap();
    let uefi_target = current_exe.with_file_name("EnigmaWave-uefi.img");

    println!(
        "\t\x1b[0;34m UEFI disk image at '{}' \x1b[0m",
        &uefi_target.display()
    );
    fs::copy(env!("UEFI_IMAGE"), &uefi_target).unwrap();

    if cfg!(feature = "qemu") {
        let mut qemu = Command::new("qemu-system-x86_64");

        qemu.arg("-drive");
        qemu.arg(format!("format=raw,file={}", env!("UEFI_IMAGE")));
        qemu.arg("-bios").arg(ovmf_prebuilt::ovmf_pure_efi());
        // qemu.arg("-serial").arg("stdio");

        let exit_status = qemu.status().unwrap();
        process::exit(exit_status.code().unwrap_or(-1));
    }
}
