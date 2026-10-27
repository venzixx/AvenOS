#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};

pub mod serial;
pub mod graphics;
pub mod security;
pub mod gdt;
pub mod interrupts;
pub mod shell;

use graphics::{render_persona_hud, with_framebuffer};

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(bootloader_api::config::Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // 1. Initialize COM1 Serial Port with hardware probing (0x3F8)
    serial::init();

    serial_println!("");
    serial_println!("=======================================================");
    serial_println!("  // AVEN.OS // 64-BIT HARDENED KERNEL (RING 0)");
    serial_println!("  \"WAKE UP, GET UP, GET OUT THERE\"");
    serial_println!("=======================================================");
    serial_println!("[OK] Hardware probe succeeded: UART 16550 COM1 linked.");
    serial_println!("[OK] Running in x86_64 Long Mode (Ring 0 supervisor).");

    // 2. Enable Hardware Security Mitigations (SMEP, SMAP, NXE)
    let _security_status = security::init();

    // 3. Initialize GDT & TSS (Interrupt Stack Table for safe stack overflow recovery)
    gdt::init();
    serial_println!("[OK] GDT & TSS initialized with Double Fault IST stack.");

    // 4. Initialize IDT & Remap 8259 PIC Interrupts
    interrupts::init_idt();
    serial_println!("[OK] IDT loaded & 8259 PIC remapped (Timer=IRQ0, Keyboard=IRQ1).");

    // 5. Initialize Thread-Safe Synchronized Framebuffer
    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        let info = framebuffer.info();
        serial_println!(
            "[OK] Linear Framebuffer active: {}x{} (format: {:?}, stride: {})",
            info.width,
            info.height,
            info.pixel_format,
            info.stride
        );

        // Store into synchronized global mutex
        graphics::init(framebuffer);

        // 6. Render Persona UI Base Dashboard
        with_framebuffer(|writer| {
            render_persona_hud(writer, &boot_info.memory_regions);
        });

        // 7. Initialize Interactive Persona Console & Shell
        shell::init();
        serial_println!("[OK] Interactive Persona Shell online and listening for input.");
    } else {
        serial_println!("[WARN] No graphical framebuffer provided by bootloader!");
    }

    // 9. Enable CPU Interrupts safely after all hardware and UI structures are ready
    x86_64::instructions::interrupts::enable();
    serial_println!("[OK] Hardware Interrupts ENABLED.");
    serial_println!("[OK] Phase 2 Milestone ACHIEVED: CPU Interrupts & Keyboard LIVE.");

    // Loop indefinitely listening for interrupts
    hlt_loop();
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 1. Force-unlock serial mutex to prevent deadlock if panic occurred inside lock
    unsafe {
        serial::force_unlock();
    }

    serial_println!("");
    serial_println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    serial_println!("  CRITICAL KERNEL PANIC // SYSTEM HALTED");
    serial_println!("  {}", info);
    serial_println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");

    // 2. Force-unlock framebuffer mutex and render Persona panic screen
    unsafe {
        graphics::force_unlock();
    }
    graphics::draw_panic_screen(info);

    hlt_loop();
}
