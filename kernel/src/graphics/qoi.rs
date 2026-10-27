use super::framebuffer::FrameBufferWriter;
use super::color::Color;

pub static WALLPAPER_QOI: &[u8] = include_bytes!("wallpaper.qoi");

/// Blits the authentic Persona 3 Reload AVENQO OS desktop directly from the embedded QOI asset.
pub fn render_qoi_desktop(writer: &mut FrameBufferWriter) {
    let qoi_data = WALLPAPER_QOI;
    if qoi_data.len() < 14 || &qoi_data[0..4] != b"qoif" {
        return;
    }

    let w = u32::from_be_bytes([qoi_data[4], qoi_data[5], qoi_data[6], qoi_data[7]]) as usize;
    let h = u32::from_be_bytes([qoi_data[8], qoi_data[9], qoi_data[10], qoi_data[11]]) as usize;

    let target_w = writer.width().min(w);
    let target_h = writer.height().min(h);

    let mut history = [[0u8; 4]; 64];
    let mut pr: u8 = 0;
    let mut pg: u8 = 0;
    let mut pb: u8 = 0;
    let pa: u8 = 255;

    let mut cur_x: usize = 0;
    let mut cur_y: usize = 0;
    let total_pixels = w * h;
    let mut px_idx: usize = 0;

    let mut idx = 14;
    let end = qoi_data.len().saturating_sub(8);

    #[inline(always)]
    fn plot_pixel(
        writer: &mut FrameBufferWriter,
        x: &mut usize,
        y: &mut usize,
        w: usize,
        target_w: usize,
        target_h: usize,
        color: Color,
    ) {
        if *x < target_w && *y < target_h {
            writer.set_pixel(*x, *y, color);
        }
        *x += 1;
        if *x == w {
            *x = 0;
            *y += 1;
        }
    }

    while idx < end && px_idx < total_pixels {
        let b1 = qoi_data[idx];
        idx += 1;

        if b1 == 0b11111110 {
            // QOI_OP_RGB
            if idx + 2 < qoi_data.len() {
                pr = qoi_data[idx];
                pg = qoi_data[idx + 1];
                pb = qoi_data[idx + 2];
                idx += 3;
                let h_idx = (pr as usize * 3 + pg as usize * 5 + pb as usize * 7 + pa as usize * 11) % 64;
                history[h_idx] = [pr, pg, pb, pa];

                plot_pixel(writer, &mut cur_x, &mut cur_y, w, target_w, target_h, Color::new(pr, pg, pb));
                px_idx += 1;
            }
        } else {
            match b1 >> 6 {
                0b00 => {
                    // QOI_OP_INDEX
                    let entry = history[(b1 & 0x3F) as usize];
                    pr = entry[0];
                    pg = entry[1];
                    pb = entry[2];

                    plot_pixel(writer, &mut cur_x, &mut cur_y, w, target_w, target_h, Color::new(pr, pg, pb));
                    px_idx += 1;
                }
                0b01 => {
                    // QOI_OP_DIFF
                    let dr = ((b1 >> 4) & 3) as i8 - 2;
                    let dg = ((b1 >> 2) & 3) as i8 - 2;
                    let db = (b1 & 3) as i8 - 2;

                    pr = pr.wrapping_add(dr as u8);
                    pg = pg.wrapping_add(dg as u8);
                    pb = pb.wrapping_add(db as u8);

                    let h_idx = (pr as usize * 3 + pg as usize * 5 + pb as usize * 7 + pa as usize * 11) % 64;
                    history[h_idx] = [pr, pg, pb, pa];

                    plot_pixel(writer, &mut cur_x, &mut cur_y, w, target_w, target_h, Color::new(pr, pg, pb));
                    px_idx += 1;
                }
                0b10 => {
                    // QOI_OP_LUMA
                    if idx < qoi_data.len() {
                        let b2 = qoi_data[idx];
                        idx += 1;

                        let dg = (b1 & 0x3F) as i8 - 32;
                        let dr_dg = ((b2 >> 4) & 0x0F) as i8 - 8;
                        let db_dg = (b2 & 0x0F) as i8 - 8;

                        let dr = dg + dr_dg;
                        let db = dg + db_dg;

                        pr = pr.wrapping_add(dr as u8);
                        pg = pg.wrapping_add(dg as u8);
                        pb = pb.wrapping_add(db as u8);

                        let h_idx = (pr as usize * 3 + pg as usize * 5 + pb as usize * 7 + pa as usize * 11) % 64;
                        history[h_idx] = [pr, pg, pb, pa];

                        plot_pixel(writer, &mut cur_x, &mut cur_y, w, target_w, target_h, Color::new(pr, pg, pb));
                        px_idx += 1;
                    }
                }
                0b11 => {
                    // QOI_OP_RUN
                    let run = ((b1 & 0x3F) + 1) as usize;
                    let color = Color::new(pr, pg, pb);
                    for _ in 0..run {
                        plot_pixel(writer, &mut cur_x, &mut cur_y, w, target_w, target_h, color);
                        px_idx += 1;
                    }
                }
                _ => {}
            }
        }
    }
}
