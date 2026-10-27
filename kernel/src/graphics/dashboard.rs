use super::framebuffer::FrameBufferWriter;
use super::qoi::render_qoi_desktop;
use bootloader_api::info::MemoryRegions;

pub fn render_persona_hud(writer: &mut FrameBufferWriter, _memory_regions: &MemoryRegions) {
    // Render the authentic Persona 3 Reload desktop wallpaper, top HUD, 11 horizontal desktop icons, and bottom dock
    render_qoi_desktop(writer);
}
