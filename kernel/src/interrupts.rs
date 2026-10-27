use crate::gdt;
use crate::serial_println;
use pic8259::ChainedPics;
use spin::{LazyLock, Mutex};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe {
    ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET)
});

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard = PIC_1_OFFSET + 1,
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

static IDT: LazyLock<InterruptDescriptorTable> = LazyLock::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    unsafe {
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
    }
    idt.page_fault.set_handler_fn(page_fault_handler);
    idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);

    idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
    idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);

    idt
});

pub fn init_idt() {
    IDT.load();
    unsafe {
        PICS.lock().initialize();
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    serial_println!("[EXCEPTION] BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("[FATAL EXCEPTION] DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    serial_println!("[EXCEPTION] PAGE FAULT");
    serial_println!("Accessed Address: {:?}", Cr2::read());
    serial_println!("Error Code: {:?}", error_code);
    serial_println!("{:#?}", stack_frame);
    panic!("Page Fault occurred!");
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    serial_println!("[EXCEPTION] GENERAL PROTECTION FAULT (code: {})", error_code);
    serial_println!("{:#?}", stack_frame);
    panic!("General Protection Fault occurred!");
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Only call on_tick if we can try_lock or without blocking
    if let Some(mut shell) = crate::shell::SHELL.try_lock() {
        shell.on_tick();
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, EventDecoder, HandleControl, ScancodeSet, ScancodeSet1};
    use x86_64::instructions::port::Port;

    static SCANCODE_SET: Mutex<ScancodeSet1> = Mutex::new(ScancodeSet1::new());
    static EVENT_DECODER: LazyLock<Mutex<EventDecoder<layouts::Us104Key>>> =
        LazyLock::new(|| Mutex::new(EventDecoder::new(layouts::Us104Key, HandleControl::Ignore)));

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    let mut scancodes = SCANCODE_SET.lock();
    let mut decoder = EVENT_DECODER.lock();

    if let Ok(Some(key_event)) = scancodes.advance_state(scancode) {
        if let Some(key) = decoder.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => {
                    crate::shell::SHELL.lock().on_char(character);
                }
                DecodedKey::RawKey(_raw) => {}
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
