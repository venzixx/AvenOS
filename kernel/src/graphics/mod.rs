pub mod color;
pub mod framebuffer;
pub mod dashboard;
pub mod qoi;

pub use color::Color;
pub use framebuffer::FrameBufferWriter;
pub use dashboard::render_persona_hud;
pub use qoi::render_qoi_desktop;

use bootloader_api::info::FrameBuffer;
use spin::Mutex;

/// Global synchronized framebuffer instance.
/// Guarantees mutual exclusion and prevents data races across threads / ISRs.
pub static FRAMEBUFFER: Mutex<Option<FrameBufferWriter>> = Mutex::new(None);

pub fn init(framebuffer: &'static mut FrameBuffer) {
    let info = framebuffer.info();
    let writer = FrameBufferWriter::new(framebuffer.buffer_mut(), info);
    x86_64::instructions::interrupts::without_interrupts(|| {
        *FRAMEBUFFER.lock() = Some(writer);
    });
}

/// Safely execute a closure with the synchronized FrameBufferWriter.
/// Disables interrupts while holding the lock to prevent ISR deadlocks.
pub fn with_framebuffer<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut FrameBufferWriter) -> R,
{
    x86_64::instructions::interrupts::without_interrupts(|| {
        FRAMEBUFFER.lock().as_mut().map(f)
    })
}

/// Emergency force-unlock for panic handling to render crash screen without deadlocking.
///
/// # Safety
/// Only call from kernel panic handler.
pub unsafe fn force_unlock() {
    FRAMEBUFFER.force_unlock();
}

/// Renders a Persona-style Critical Kernel Failure screen.
pub fn draw_panic_screen(info: &core::panic::PanicInfo) {
    if let Some(ref mut writer) = *FRAMEBUFFER.lock() {
        let w = writer.width();
        let h = writer.height();

        // High contrast Crimson & Black background
        writer.fill_rect(0, 0, w, h, Color::CRIMSON_DARK);
        writer.draw_slanted_panel(0, h / 4, w, h / 2, -100, Color::PHANTOM_BLACK);

        // Warning banner
        writer.draw_slanted_panel(40, h / 4 + 20, 480, 40, -15, Color::CRIMSON_RED);
        writer.draw_string(55, h / 4 + 30, "// THE SHOW IS OVER //", Color::CHALK_WHITE, None, 2);

        writer.draw_badge(540, h / 4 + 25, "CRITICAL KERNEL PANIC", Color::STAR_YELLOW, Color::PHANTOM_BLACK);

        // Panic details
        let mut y = h / 4 + 80;
        writer.draw_string(60, y, "A fatal exception halted AvenOS execution:", Color::CHALK_WHITE, None, 1);
        y += 24;

        if let Some(location) = info.location() {
            writer.draw_string(60, y, "LOCATION : ", Color::MUTED_GRAY, None, 1);
            writer.draw_string(150, y, location.file(), Color::CYAN_ACCENT, None, 1);
            y += 20;
        }

        writer.draw_string(60, y, "HALT     : CPU halted in safe Ring 0 loop.", Color::MUTED_GRAY, None, 1);
    }
}
