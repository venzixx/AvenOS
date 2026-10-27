#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    // =========================================================================
    // AVENQO OS // Persona 3 Reload Authentic Palette
    // =========================================================================
    pub const P3_BG_OCEAN: Color = Color::new(0, 12, 30);          // #000c1e
    pub const P3_BG_DARK: Color = Color::new(0, 8, 20);            // #000814
    pub const P3_BLUE_PRIMARY: Color = Color::new(0, 119, 255);    // #0077ff
    pub const P3_BLUE_VIVID: Color = Color::new(0, 210, 255);      // #00d2ff
    pub const P3_BLUE_DEEP: Color = Color::new(0, 31, 77);         // #001f4d
    pub const P3_BLUE_GLASS: Color = Color::new(0, 20, 52);        // Window card glass
    pub const P3_CYAN_LIGHT: Color = Color::new(119, 254, 252);    // #77fefc
    pub const P3_CYAN_GLOW: Color = Color::new(22, 207, 251);      // #16cffb
    pub const P3_YELLOW: Color = Color::new(255, 215, 0);          // #ffd700
    pub const P3_YELLOW_NEON: Color = Color::new(255, 242, 0);     // #fff200
    pub const P3_RED: Color = Color::new(255, 43, 86);             // #ff2b56
    pub const P3_TEXT_WHITE: Color = Color::new(255, 255, 255);
    pub const P3_TEXT_MUTED: Color = Color::new(140, 180, 215);
    pub const P3_BORDER_CYAN: Color = Color::new(0, 210, 255);
    pub const P3_BORDER_DARK: Color = Color::new(0, 60, 120);

    // Dark Hour Tartarus Theme Palette
    pub const DH_BG_DARK: Color = Color::new(2, 18, 8);            // #021208
    pub const DH_GREEN_VIVID: Color = Color::new(0, 255, 136);     // #00ff88
    pub const DH_GREEN_DEEP: Color = Color::new(5, 38, 20);        // #052614
    pub const DH_CRIMSON: Color = Color::new(255, 43, 86);         // #ff2b56

    // Persona 5 / Classic Accents
    pub const PHANTOM_BLACK: Color = Self::P3_BG_OCEAN;
    pub const CHARCOAL_DARK: Color = Self::P3_BLUE_DEEP;
    pub const CHARCOAL_LIGHT: Color = Self::P3_BORDER_DARK;
    pub const CRIMSON_RED: Color = Self::P3_RED;
    pub const CRIMSON_BRIGHT: Color = Color::new(255, 60, 100);
    pub const CRIMSON_DARK: Color = Color::new(140, 15, 35);
    pub const CHALK_WHITE: Color = Self::P3_TEXT_WHITE;
    pub const MUTED_GRAY: Color = Self::P3_TEXT_MUTED;
    pub const DARK_GRAY: Color = Color::new(60, 85, 115);
    pub const STAR_YELLOW: Color = Self::P3_YELLOW;
    pub const CYAN_ACCENT: Color = Self::P3_CYAN_LIGHT;
}
