use crate::graphics::color::Color;
use crate::graphics::with_framebuffer;
use crate::serial_println;
use spin::Mutex;
use x86_64::instructions::port::Port;

const MAX_LINES: usize = 24;
const MAX_LINE_CHARS: usize = 110;
const MAX_INPUT: usize = 64;
const PROMPT_STR: &str = "sees@avenqo-os:~# ";

// Geometry inside the S.E.E.S. Command Matrix Window
// Positioned in center to preserve top 11 desktop icons and bottom taskbar dock!
const WIN_X: usize = 140;
const WIN_Y: usize = 145;
const WIN_W: usize = 1000;
const WIN_H: usize = 515;

const CONTENT_X: usize = WIN_X + 24;
const CONTENT_TOP_Y: usize = WIN_Y + 48;
const LINE_HEIGHT: usize = 18;

#[derive(Clone, Copy)]
pub struct Line {
    bytes: [u8; MAX_LINE_CHARS],
    len: usize,
    color: Color,
}

impl Line {
    pub const fn empty() -> Self {
        Self {
            bytes: [0; MAX_LINE_CHARS],
            len: 0,
            color: Color::P3_TEXT_WHITE,
        }
    }

    pub fn new(text: &str, color: Color) -> Self {
        let mut line = Self::empty();
        let len = text.len().min(MAX_LINE_CHARS);
        line.bytes[..len].copy_from_slice(&text.as_bytes()[..len]);
        line.len = len;
        line.color = color;
        line
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

pub struct Shell {
    lines: [Line; MAX_LINES],
    line_count: usize,
    input_buf: [u8; MAX_INPUT],
    input_len: usize,
    cursor_visible: bool,
    ticks: u64,
    dark_hour: bool,
    pub window_visible: bool,
}

impl Shell {
    pub const fn new() -> Self {
        Self {
            lines: [Line::empty(); MAX_LINES],
            line_count: 0,
            input_buf: [0; MAX_INPUT],
            input_len: 0,
            cursor_visible: true,
            ticks: 0,
            dark_hour: false,
            window_visible: false, // Clean desktop on boot; opens on first keystroke!
        }
    }

    pub fn push_line(&mut self, text: &str, color: Color) {
        serial_println!("[SHELL] {}", text);

        if self.line_count < MAX_LINES {
            self.lines[self.line_count] = Line::new(text, color);
            self.line_count += 1;
        } else {
            // Scroll up by 1 line
            for i in 1..MAX_LINES {
                self.lines[i - 1] = self.lines[i];
            }
            self.lines[MAX_LINES - 1] = Line::new(text, color);
        }
    }

    pub fn on_char(&mut self, c: char) {
        if !self.window_visible {
            self.window_visible = true;
            self.redraw_all();
            if c == '\n' || c == '\r' || c == ' ' {
                return;
            }
        }

        match c {
            '\n' => {
                // 1. Commit the prompt line into the history
                let mut full_cmd_line = [0u8; MAX_LINE_CHARS];
                let prompt_b = PROMPT_STR.as_bytes();
                let p_len = prompt_b.len();
                full_cmd_line[..p_len].copy_from_slice(prompt_b);
                let in_len = self.input_len.min(MAX_LINE_CHARS - p_len);
                full_cmd_line[p_len..p_len + in_len].copy_from_slice(&self.input_buf[..in_len]);
                let total_len = p_len + in_len;

                let line_str = core::str::from_utf8(&full_cmd_line[..total_len]).unwrap_or("");
                self.push_line(line_str, Color::P3_YELLOW);

                // 2. Extract command string
                let mut cmd_copy = [0u8; MAX_INPUT];
                cmd_copy[..self.input_len].copy_from_slice(&self.input_buf[..self.input_len]);
                let cmd_str = match core::str::from_utf8(&cmd_copy[..self.input_len]) {
                    Ok(s) => s.trim(),
                    Err(_) => "",
                };

                // 3. Execute command and append its outputs below
                self.execute_command(cmd_str);

                // 4. Reset input buffer for the next command
                self.input_len = 0;
                self.cursor_visible = true;

                // 5. Redraw the full terminal lines smoothly if window is still open
                if self.window_visible {
                    self.redraw_all();
                }
            }
            '\x08' => {
                // Backspace
                if self.input_len > 0 {
                    self.erase_cursor();
                    self.input_len -= 1;
                    let cur_x = CONTENT_X + (PROMPT_STR.len() + self.input_len) * 8;
                    let cur_y = CONTENT_TOP_Y + (self.line_count * LINE_HEIGHT);
                    with_framebuffer(|w| {
                        w.fill_rect(cur_x, cur_y, 8, 16, Color::P3_BG_DARK);
                    });
                    self.draw_cursor();
                }
            }
            _ => {
                // Printable character
                if self.input_len < MAX_INPUT && c.is_ascii() && !c.is_ascii_control() {
                    self.erase_cursor();
                    let cur_x = CONTENT_X + (PROMPT_STR.len() + self.input_len) * 8;
                    let cur_y = CONTENT_TOP_Y + (self.line_count * LINE_HEIGHT);

                    self.input_buf[self.input_len] = c as u8;
                    self.input_len += 1;

                    with_framebuffer(|w| {
                        w.draw_char(cur_x, cur_y, c, Color::P3_TEXT_WHITE, None, 1);
                    });

                    self.draw_cursor();
                }
            }
        }
    }

    fn erase_cursor(&self) {
        if !self.window_visible {
            return;
        }
        let cur_x = CONTENT_X + (PROMPT_STR.len() + self.input_len) * 8;
        let cur_y = CONTENT_TOP_Y + (self.line_count * LINE_HEIGHT);
        with_framebuffer(|w| {
            w.fill_rect(cur_x, cur_y, 8, 16, Color::P3_BG_DARK);
        });
    }

    fn draw_cursor(&self) {
        if !self.window_visible {
            return;
        }
        let cur_x = CONTENT_X + (PROMPT_STR.len() + self.input_len) * 8;
        let cur_y = CONTENT_TOP_Y + (self.line_count * LINE_HEIGHT);
        let c_color = if self.cursor_visible {
            if self.dark_hour {
                Color::DH_GREEN_VIVID
            } else {
                Color::P3_YELLOW
            }
        } else {
            Color::P3_BG_DARK
        };
        with_framebuffer(|w| {
            w.fill_rect(cur_x, cur_y, 8, 16, c_color);
        });
    }

    pub fn on_tick(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
        if !self.window_visible {
            return;
        }
        if self.ticks % 10 == 0 {
            self.cursor_visible = !self.cursor_visible;
            self.draw_cursor();
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        match cmd {
            "" => {}
            "help" => {
                self.push_line("S.E.E.S. COMMAND MATRIX DIRECTIVES:", Color::P3_BORDER_CYAN);
                self.push_line("  help        - Display command directives", Color::P3_TEXT_WHITE);
                self.push_line("  status      - S.E.E.S. neural system & security health", Color::P3_TEXT_WHITE);
                self.push_line("  squad       - View active S.E.E.S. operatives roster", Color::P3_CYAN_LIGHT);
                self.push_line("  compendium  - Inspect Velvet Room Persona entries", Color::P3_YELLOW);
                self.push_line("  lunar       - Display Dark Hour lunar cycle status", Color::P3_TEXT_WHITE);
                self.push_line("  darkhour    - Toggle Tartarus Dark Hour state", Color::P3_RED);
                self.push_line("  hide / min  - Minimize terminal back to pure desktop", Color::P3_CYAN_LIGHT);
                self.push_line("  neofetch    - Display AVENQO OS architecture & branding", Color::P3_TEXT_WHITE);
                self.push_line("  clear       - Clear matrix window", Color::P3_TEXT_WHITE);
                self.push_line("  echo <msg>  - Echo text to console", Color::P3_TEXT_WHITE);
                self.push_line("  reboot      - Reboot system via 8042 controller", Color::P3_TEXT_WHITE);
            }
            "status" => {
                self.push_line("AVENQO OS // NEURAL DESKTOP [ONLINE]", Color::P3_BORDER_CYAN);
                self.push_line("  KERNEL      : AvenOS Core v3.04 (x86_64 Long Mode, Ring 0)", Color::P3_TEXT_WHITE);
                self.push_line("  THEME       : Persona 3 Reload (Gekkoukan High Neural Grid)", Color::P3_CYAN_LIGHT);
                self.push_line("  SECURITY    : SMEP=ACTIVE | SMAP=ACTIVE | NXE=ACTIVE", Color::P3_YELLOW);
                self.push_line("  INTERRUPTS  : IDT Loaded | 8259 PIC Remapped | PIT Timer Live", Color::P3_BORDER_CYAN);
                self.push_line("  FRAMEBUFFER : 1280x720 True Color (BGR 24-bit)", Color::P3_TEXT_WHITE);
            }
            "squad" | "sees" => {
                self.push_line("// S.E.E.S. OPERATIVE DEPLOYMENT ROSTER //", Color::P3_BORDER_CYAN);
                self.push_line("  [01] MAKOTO YUKI     // LEADER     // LVL 75 // PERSONA: ORPHEUS", Color::P3_TEXT_WHITE);
                self.push_line("  [02] YUKARI TAKEBA   // RECON      // LVL 72 // PERSONA: IO", Color::P3_TEXT_WHITE);
                self.push_line("  [03] JUNPEI IORI     // VANGUARD   // LVL 71 // PERSONA: HERMES", Color::P3_TEXT_WHITE);
                self.push_line("  [04] AKIHIKO SANADA  // ASSAULT    // LVL 74 // PERSONA: POLYDEUCES", Color::P3_TEXT_WHITE);
                self.push_line("  [05] MITSURU KIRIJO  // COMMAND    // LVL 75 // PERSONA: PENTHESILEA", Color::P3_RED);
                self.push_line("  [06] FUUKA YAMAGISHI // NAVIGATOR  // LVL 70 // PERSONA: LUCIA", Color::P3_CYAN_LIGHT);
                self.push_line("  [07] AIGIS           // SUPPRESSION// LVL 73 // PERSONA: PALLADION", Color::P3_YELLOW);
            }
            "compendium" => {
                self.push_line("// VELVET ROOM PERSONA COMPENDIUM //", Color::P3_YELLOW);
                self.push_line("  - [FOOL]      ORPHEUS    // LVL 1  // Agi, Bash, Tarunda", Color::P3_TEXT_WHITE);
                self.push_line("  - [DEATH]     THANATOS   // LVL 64 // Brave Blade, Megidolaon", Color::P3_RED);
                self.push_line("  - [JUDGEMENT] MESSIAH    // LVL 90 // God's Hand, Salvation", Color::P3_CYAN_LIGHT);
                self.push_line("  - [MAGICIAN]  JACK FROST // LVL 8  // Bufu, Mabufu, Rakunda", Color::P3_BORDER_CYAN);
            }
            "lunar" => {
                self.push_line("// TARTARUS LUNAR APPARITION CYCLE //", Color::P3_YELLOW);
                self.push_line("  CURRENT DATE  : OCT 23 [FRI]", Color::P3_TEXT_WHITE);
                self.push_line("  MOON PHASE    : WAXING GIBBOUS (88% ILLUMINATION)", Color::P3_CYAN_LIGHT);
                self.push_line("  COUNTDOWN     : 4 DAYS REMAINING UNTIL FULL MOON SHADOW INVASION", Color::P3_RED);
            }
            "darkhour" => {
                self.dark_hour = !self.dark_hour;
                if self.dark_hour {
                    self.push_line("[DARK HOUR DETECTED] Time has stopped. Tartarus awakens.", Color::DH_GREEN_VIVID);
                    self.push_line(">> Green Transmutation protocol active.", Color::DH_CRIMSON);
                } else {
                    self.push_line("[DARK HOUR CONCLUDED] Normal daylight timeline restored.", Color::P3_CYAN_LIGHT);
                }
            }
            "hide" | "min" | "close" => {
                self.window_visible = false;
                with_framebuffer(|writer| {
                    crate::graphics::render_qoi_desktop(writer);
                });
            }
            "neofetch" | "about" => {
                self.push_line(r"  /\   AVENQO OS // 'MEMENTO MORI'", Color::P3_BORDER_CYAN);
                self.push_line(r" /  \  Inspired by Persona 3 Reload Neural Desktop", Color::P3_TEXT_WHITE);
                self.push_line(r"/____\ OS: AVENQO OS (P3R Themed x86_64 Kernel)", Color::P3_CYAN_LIGHT);
                self.push_line("       Kernel: Rust no_std 64-bit | IDT | Paging", Color::P3_TEXT_WHITE);
                self.push_line("       UI: 1280x720 Linear Framebuffer true-color", Color::P3_YELLOW);
                self.push_line("       Author: Sidharth & Antigravity Pair-Programming", Color::P3_BORDER_CYAN);
            }
            "clear" => {
                self.line_count = 0;
            }
            "echo" => {
                let rest: &str = cmd.strip_prefix("echo").unwrap_or("").trim();
                self.push_line(rest, Color::P3_TEXT_WHITE);
            }
            "panic" => {
                panic!("User triggered manual test panic from Persona shell!");
            }
            "reboot" => {
                self.push_line("Initiating CPU reset via 8042 controller...", Color::P3_RED);
                unsafe {
                    let mut port = Port::<u8>::new(0x64);
                    port.write(0xFE);
                }
            }
            unknown => {
                let mut buf = [0u8; 90];
                let prefix = b"Unknown directive: '";
                let p_len = prefix.len();
                buf[..p_len].copy_from_slice(prefix);
                let u_len = unknown.len().min(90 - p_len - 1);
                buf[p_len..p_len + u_len].copy_from_slice(&unknown.as_bytes()[..u_len]);
                buf[p_len + u_len] = b'\'';
                if let Ok(s) = core::str::from_utf8(&buf[..p_len + u_len + 1]) {
                    self.push_line(s, Color::P3_RED);
                }
            }
        }
    }

    /// Redraws the terminal contents cleanly inside the S.E.E.S. Command Matrix Window
    pub fn redraw_all(&self) {
        if !self.window_visible {
            return;
        }

        with_framebuffer(|writer| {
            // 1. Draw Window Body & 2px Cyan Border
            writer.fill_rect(WIN_X, WIN_Y, WIN_W, WIN_H, Color::P3_BG_DARK);
            writer.draw_rect_outline(WIN_X, WIN_Y, WIN_W, WIN_H, 2, Color::P3_BORDER_CYAN);

            // 2. Draw Window Header Titlebar (h = 34)
            writer.fill_rect(WIN_X + 2, WIN_Y + 2, WIN_W - 4, 32, Color::P3_BLUE_DEEP);
            writer.fill_rect(WIN_X + 2, WIN_Y + 34, WIN_W - 4, 2, Color::P3_BORDER_CYAN);

            // 3. Header Title Text & Controls
            writer.draw_string(WIN_X + 16, WIN_Y + 12, "// 01: S.E.E.S. COMMAND MATRIX // v3.04", Color::P3_CYAN_LIGHT, None, 1);
            writer.draw_badge(WIN_X + WIN_W - 74, WIN_Y + 8, "-", Color::P3_BLUE_PRIMARY, Color::P3_CYAN_LIGHT);
            writer.draw_badge(WIN_X + WIN_W - 40, WIN_Y + 8, "X", Color::P3_RED, Color::P3_TEXT_WHITE);

            // 4. Draw committed lines inside window
            let mut y = CONTENT_TOP_Y;
            for i in 0..self.line_count {
                let line = &self.lines[i];
                writer.draw_string(CONTENT_X, y, line.as_str(), line.color, None, 1);
                y += LINE_HEIGHT;
            }

            // 5. Draw active prompt line below history
            let prompt_y = CONTENT_TOP_Y + (self.line_count * LINE_HEIGHT);
            let prompt_color = if self.dark_hour {
                Color::DH_GREEN_VIVID
            } else {
                Color::P3_BORDER_CYAN
            };
            writer.draw_string(CONTENT_X, prompt_y, PROMPT_STR, prompt_color, None, 1);

            let input_x = CONTENT_X + PROMPT_STR.len() * 8;
            if let Ok(inp) = core::str::from_utf8(&self.input_buf[..self.input_len]) {
                writer.draw_string(input_x, prompt_y, inp, Color::P3_TEXT_WHITE, None, 1);
            }

            // 6. Draw cursor
            let cur_x = input_x + self.input_len * 8;
            let c_color = if self.cursor_visible {
                if self.dark_hour {
                    Color::DH_GREEN_VIVID
                } else {
                    Color::P3_YELLOW
                }
            } else {
                Color::P3_BG_DARK
            };
            writer.fill_rect(cur_x, prompt_y, 8, 16, c_color);
        });
    }
}

pub static SHELL: Mutex<Shell> = Mutex::new(Shell::new());

pub fn init() {
    let mut shell = SHELL.lock();
    shell.push_line("[ 0.000000 ] [ OK ] 64-bit Long Mode CPU Handshake complete.", Color::P3_BORDER_CYAN);
    shell.push_line("[ 0.000104 ] [ OK ] UART 16550 Serial Port probed and linked.", Color::P3_BORDER_CYAN);
    shell.push_line("[ 0.000218 ] [ OK ] Linear Framebuffer initialized: 1280x720 True Color.", Color::P3_BORDER_CYAN);
    shell.push_line("[ 0.000342 ] [ OK ] Hardware Security: SMEP, SMAP, NXE ACTIVE.", Color::P3_YELLOW);
    shell.push_line("[ 0.000495 ] [ OK ] GDT & TSS Interrupt Stack Table loaded.", Color::P3_BORDER_CYAN);
    shell.push_line("[ 0.000620 ] [ OK ] IDT loaded with 8259 PIC remapped.", Color::P3_BORDER_CYAN);
    shell.push_line("[ 0.000780 ] [ OK ] PS/2 Keyboard & PIT Timer interrupts LIVE.", Color::P3_BORDER_CYAN);
    shell.push_line(">> Type 'help' to explore S.E.E.S. commands, or 'squad' for team roster!", Color::P3_YELLOW);
    // Note: window_visible starts false to display the clean desktop on boot!
    // Typing any key opens the window immediately.
}
