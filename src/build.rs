use bootloader::DiskImageBuilder;
use std::{env, path::PathBuf};

fn main() {
    // set by cargo for the kernel artifact dependency
    let kernel_path = env::var("CARGO_BIN_FILE_ENIGMA_KERNEL_kernel").unwrap();
    let disk_builder = DiskImageBuilder::new(PathBuf::from(kernel_path));

    // specify output paths
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_dir = out_dir.join("enigma-wave_uefi.img");

    // create the disk images
    disk_builder.create_uefi_image(&out_dir).unwrap();

    // pass the disk image paths via environment variables
    println!("cargo:rustc-env=UEFI_IMAGE={}", out_dir.display());
}
