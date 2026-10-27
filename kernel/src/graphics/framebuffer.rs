use bootloader_api::info::{FrameBufferInfo, PixelFormat};
use super::color::Color;

pub struct FrameBufferWriter {
    buffer: &'static mut [u8],
    info: FrameBufferInfo,
}

impl FrameBufferWriter {
    pub fn new(buffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        Self { buffer, info }
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.info.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.info.height
    }

    #[inline]
    pub fn bytes_per_pixel(&self) -> usize {
        self.info.bytes_per_pixel
    }

    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x >= self.info.width || y >= self.info.height || self.info.bytes_per_pixel == 0 {
            return;
        }

        let pixel_offset = y * self.info.stride + x;
        let byte_offset = match pixel_offset.checked_mul(self.info.bytes_per_pixel) {
            Some(offset) => offset,
            None => return, // Integer overflow protection
        };

        if byte_offset.saturating_add(self.info.bytes_per_pixel) > self.buffer.len() {
            return;
        }

        match self.info.pixel_format {
            PixelFormat::Rgb => {
                self.buffer[byte_offset] = color.r;
                self.buffer[byte_offset + 1] = color.g;
                self.buffer[byte_offset + 2] = color.b;
            }
            PixelFormat::Bgr => {
                self.buffer[byte_offset] = color.b;
                self.buffer[byte_offset + 1] = color.g;
                self.buffer[byte_offset + 2] = color.r;
            }
            PixelFormat::U8 => {
                let gray = ((color.r as u16 + color.g as u16 + color.b as u16) / 3) as u8;
                self.buffer[byte_offset] = gray;
            }
            _ => {
                self.buffer[byte_offset] = color.b;
                self.buffer[byte_offset + 1] = color.g;
                self.buffer[byte_offset + 2] = color.r;
            }
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        if x >= self.info.width || y >= self.info.height || self.info.bytes_per_pixel == 0 {
            return Color::new(0, 0, 0);
        }
        let pixel_offset = y * self.info.stride + x;
        let byte_offset = pixel_offset * self.info.bytes_per_pixel;
        if byte_offset + self.info.bytes_per_pixel > self.buffer.len() {
            return Color::new(0, 0, 0);
        }
        match self.info.pixel_format {
            PixelFormat::Rgb => Color::new(
                self.buffer[byte_offset],
                self.buffer[byte_offset + 1],
                self.buffer[byte_offset + 2],
            ),
            _ => Color::new(
                self.buffer[byte_offset + 2],
                self.buffer[byte_offset + 1],
                self.buffer[byte_offset],
            ),
        }
    }

    /// Fast buffer clearing using bulk chunk filling
    pub fn clear(&mut self, color: Color) {
        let bpp = self.info.bytes_per_pixel;
        if bpp == 3 {
            let px = match self.info.pixel_format {
                PixelFormat::Rgb => [color.r, color.g, color.b],
                _ => [color.b, color.g, color.r],
            };
            for chunk in self.buffer.chunks_exact_mut(3) {
                chunk.copy_from_slice(&px);
            }
        } else if bpp == 4 {
            let px = match self.info.pixel_format {
                PixelFormat::Rgb => [color.r, color.g, color.b, 0],
                _ => [color.b, color.g, color.r, 0],
            };
            for chunk in self.buffer.chunks_exact_mut(4) {
                chunk.copy_from_slice(&px);
            }
        } else {
            for y in 0..self.info.height {
                for x in 0..self.info.width {
                    self.set_pixel(x, y, color);
                }
            }
        }
    }

    /// Fast horizontal rectangle fill using row slice chunks
    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        if x >= self.info.width || y >= self.info.height || w == 0 || h == 0 || self.info.bytes_per_pixel == 0 {
            return;
        }
        let max_x = (x.saturating_add(w)).min(self.info.width);
        let max_y = (y.saturating_add(h)).min(self.info.height);
        let bpp = self.info.bytes_per_pixel;
        let stride = self.info.stride;

        let px_bgr = [color.b, color.g, color.r];
        let px_rgb = [color.r, color.g, color.b];
        let use_rgb = matches!(self.info.pixel_format, PixelFormat::Rgb);

        for cy in y..max_y {
            let row_start = (cy * stride + x) * bpp;
            let row_end = (cy * stride + max_x) * bpp;

            if row_end <= self.buffer.len() && row_start < row_end {
                let row_slice = &mut self.buffer[row_start..row_end];
                if bpp == 3 {
                    let px = if use_rgb { &px_rgb } else { &px_bgr };
                    for chunk in row_slice.chunks_exact_mut(3) {
                        chunk.copy_from_slice(px);
                    }
                } else if bpp == 4 {
                    let b0 = if use_rgb { color.r } else { color.b };
                    let b2 = if use_rgb { color.b } else { color.r };
                    for chunk in row_slice.chunks_exact_mut(4) {
                        chunk[0] = b0;
                        chunk[1] = color.g;
                        chunk[2] = b2;
                    }
                } else {
                    for cx in x..max_x {
                        self.set_pixel(cx, cy, color);
                    }
                }
            }
        }
    }

    pub fn draw_rect_outline(&mut self, x: usize, y: usize, w: usize, h: usize, thickness: usize, color: Color) {
        self.fill_rect(x, y, w, thickness, color);
        if h > thickness {
            self.fill_rect(x, y + h - thickness, w, thickness, color);
        }
        self.fill_rect(x, y, thickness, h, color);
        if w > thickness {
            self.fill_rect(x + w - thickness, y, thickness, h, color);
        }
    }

    /// Persona Slanted Panel: draws an angled parallelogram with bounds checking.
    pub fn draw_slanted_panel(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        slant_dx: isize,
        color: Color,
    ) {
        if h == 0 || w == 0 {
            return;
        }
        for row in 0..h {
            let offset = (row as isize * slant_dx) / (h as isize);
            let raw_start = x as isize + offset;
            if raw_start < 0 {
                let clipped = (-raw_start) as usize;
                if clipped < w {
                    self.fill_rect(0, y + row, w - clipped, 1, color);
                }
            } else {
                let start_x = raw_start as usize;
                if start_x < self.info.width {
                    let clamped_w = w.min(self.info.width - start_x);
                    self.fill_rect(start_x, y + row, clamped_w, 1, color);
                }
            }
        }
    }

    /// Persona Slanted Badge: an angled badge with text inside
    pub fn draw_badge(
        &mut self,
        x: usize,
        y: usize,
        text: &str,
        bg_color: Color,
        text_color: Color,
    ) {
        let pad_x = 12;
        let pad_y = 6;
        let text_w = text.len() * 8;
        let total_w = text_w + pad_x * 2;
        let total_h = 8 + pad_y * 2;

        self.draw_slanted_panel(x, y, total_w, total_h, -8, bg_color);
        self.draw_string(x + pad_x, y + pad_y, text, text_color, None, 1);
    }

    pub fn draw_char(&mut self, x: usize, y: usize, c: char, fg: Color, bg: Option<Color>, scale: usize) {
        use font8x8::UnicodeFonts;

        let glyph = font8x8::BASIC_FONTS.get(c).unwrap_or([0; 8]);

        for (row, &byte) in glyph.iter().enumerate() {
            for col in 0..8 {
                let bit = (byte >> col) & 1;
                let px_color = if bit == 1 {
                    Some(fg)
                } else {
                    bg
                };

                if let Some(color_val) = px_color {
                    if scale == 1 {
                        self.set_pixel(x + col, y + row, color_val);
                    } else {
                        self.fill_rect(x + col * scale, y + row * scale, scale, scale, color_val);
                    }
                }
            }
        }
    }

    pub fn draw_string(
        &mut self,
        x: usize,
        y: usize,
        text: &str,
        fg: Color,
        bg: Option<Color>,
        scale: usize,
    ) {
        let mut cur_x = x;
        let mut cur_y = y;

        for c in text.chars() {
            if c == '\n' {
                cur_x = x;
                cur_y += 8 * scale + 4;
                continue;
            }
            self.draw_char(cur_x, cur_y, c, fg, bg, scale);
            cur_x += 8 * scale;
        }
    }
}
