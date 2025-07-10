//! Данный бинарный модуль содержит исходный код ядра и точку входа в ядро, а
//! также является сердцем данной системы.

#![no_std] // Отключаем стандартную библиотеку.
#![no_main] // Отключаем main функцию.

// Настройка тестов.
#![feature(custom_test_frameworks)]
#![test_runner(enigma_kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader_api::{BootInfo, BootloaderConfig, config::Mapping, entry_point};
use enigma_kernel::{allocator, println, serial_println};
extern crate alloc;

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);
static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);

    config
};

/// Точка входа в ядро.
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    use bootloader_api::info::Optional;
    use enigma_kernel::task::{Task, executor::Executor, keyboard};
    use enigma_kernel::{drivers::apic, memory};
    use x86_64::VirtAddr;

    // Проверка на наличие фреймбуфера.
    if let Optional::None = &boot_info.framebuffer {
        print_to_vga(b"Framebuffer not found, system stopped!");
        enigma_kernel::hlt_loop();
    }

    // Инициализация фреймбуфера.
    let framebuffer = boot_info.framebuffer.take().unwrap();
    let framebuffer_info = framebuffer.info();

    enigma_kernel::framebuffer::init(framebuffer.into_buffer(), framebuffer_info);

    // Инициализация ядра.
    enigma_kernel::init();

    // Инициализация paging.
    let phys_mem_offset = VirtAddr::new(
        boot_info
            .physical_memory_offset
            .take()
            .expect("Failed to find physical memory offset"),
    );

    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_regions) };

    // Инициализация кучи ядра.
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Head initialization failed");

    // Инициализация APIC контроллера.
    unsafe {
        let rsdp: Option<u64> = boot_info.rsdp_addr.take();
        apic::init(
            rsdp.expect("") as usize,
            phys_mem_offset,
            &mut mapper,
            &mut frame_allocator,
        );
    }

    // Запуск тестов (если требуется).
    #[cfg(test)]
    test_main();

    // Запуск ассинхронных служб ядра.
    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypresses()));

    executor.run();
}

#[inline]
/// Данная функция печатает текст в VGA.
fn print_to_vga(s: &[u8]) {
    let vga_buffer = 0xb8000 as *mut u8;

    for (i, &byte) in s.iter().enumerate() {
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xC;
        }
    }
}

/// Функция обработки паники.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    serial_println!("{}", info);
    println!("{}", info);

    enigma_kernel::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    enigma_kernel::test_panic_handler(info);
}
