#![feature(abi_x86_interrupt)]
#![no_main]
#![no_std]

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use core::panic::PanicInfo;

use enigma_wave::{exit_qemu, hlt_loop, serial_print, serial_println, QemuExitCode};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    serial_print!("stack_overflow::stack_overflow...\t");

    enigma_wave::gdt::init();
    init_test_idt();

    // Вызываем переполнение стека.
    stack_overflow();

    panic!("Execution continued after stack overflow");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    enigma_wave::test_panic_handler(info);
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow();
    volatile::Volatile::new(0).read();
}

// Init InterruptDescriptorTable.
lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_fn(test_double_fault_handler)
                .set_stack_index(enigma_wave::gdt::DOUBLE_FAULT_IST_INDEXT);
        }

        idt
    };
}

extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64
) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);

    hlt_loop();
}

pub fn init_test_idt() {
    TEST_IDT.load();
}

