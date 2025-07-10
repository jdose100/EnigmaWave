//! Enigma Wave - это маленькая операционная система написанная энтузиастом jdose100.
//! Она является эксперементом над созданием личной ОС. Целью данной системы является создание
//! ядра с удобным API для работы (подобно WinAPI), быстрой скоростью работы и совместимостью с
//! Linux приложениями (которые используют libc, так-как syscalls API сильно отличается).

// Настройка тестов.
#![reexport_test_harness_main = "test_main"]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
// Настройка feature's.
#![feature(default_field_values)]
#![feature(abi_x86_interrupt)]
#![no_std]

extern crate alloc;

pub mod allocator;
pub mod drivers;
pub mod framebuffer;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod task;

pub fn init() {
    gdt::init();
    // x86_64::instructions::interrupts::enable();
}

#[inline]
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

// -- QEMU CODE -- //

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

// ------- TEST ZONE ------- //

#[cfg(test)]
use bootloader_api::{BootInfo, entry_point};

#[cfg(test)]
entry_point!(test_kernel_main);

/// Точка входа для тестов.
#[cfg(test)]
fn test_kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    init();

    test_main();
    hlt_loop();
}

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());

    for test in tests {
        test.run();
    }

    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &core::panic::PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);

    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    test_panic_handler(info);
}
