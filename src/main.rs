#![no_std]  // Отключаем стандартную библиотеку.
#![no_main] // Отключаем main функцию.

#![feature(custom_test_frameworks)]
#![test_runner(enigma_wave::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{BootInfo, entry_point};
use enigma_wave::{allocator, println};

entry_point!(kernel_main);
/// Точка входа в ядро.
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use x86_64::VirtAddr;
    use enigma_wave::memory;

    // println!("Hello, World!");
    enigma_wave::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        memory::BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    // Init heap.
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect(
        "Head initialization failed"
    );

    #[cfg(test)]
    test_main();

    use enigma_wave::task::{Task, executor::Executor, keyboard};

    let mut executor = Executor::new(); // new
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}

/// Функция обработки паники.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);

    enigma_wave::hlt_loop();
}


#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    enigma_wave::test_panic_handler(info);
}

