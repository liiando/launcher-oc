//! Visual identity for the launcher — "SUMMIT": neon-noir alpine aesthetic.
//!
//! A deep cold-to-black vertical gradient (the light of the summit up top),
//! a glassy central panel with a cyan hairline, and a TikTok-derived duochrome
//! of glacial cyan + hot magenta used sparingly on near-black.

use iced::gradient::Linear;
use iced::widget::{button, container, text_input};
use iced::{theme, Background, Border, Color, Font, Gradient, Radians, Shadow, Theme, Vector};

use std::f32::consts::{FRAC_PI_2, PI};

// ---- palette ---------------------------------------------------------------

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    rgba(r, g, b, 1.0)
}
const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a,
    }
}

pub const BG_TOP: Color = rgb(0x10, 0x1A, 0x30);
pub const BG_BOTTOM: Color = rgb(0x03, 0x05, 0x0B);

pub const CYAN: Color = rgb(0x2F, 0xE8, 0xD6);
pub const CYAN_DEEP: Color = rgb(0x12, 0xA7, 0xC4);
pub const CYAN_BRIGHT: Color = rgb(0x6C, 0xF5, 0xE6);
pub const MAGENTA: Color = rgb(0xFF, 0x2E, 0x63);
pub const MAGENTA_SOFT: Color = rgb(0xFF, 0x5E, 0x8A);

pub const TEXT: Color = rgb(0xEC, 0xF1, 0xFB);
pub const MUTED: Color = rgb(0x86, 0x90, 0xA8);
pub const INK: Color = rgb(0x03, 0x07, 0x10); // dark text on bright fills

// ---- fonts -----------------------------------------------------------------

pub const DISPLAY: Font = Font::with_name("Bebas Neue");
pub const BODY: Font = Font::with_name("Chakra Petch");

// ---- helpers ---------------------------------------------------------------

fn linear(angle: f32, from: Color, to: Color) -> Background {
    Background::Gradient(Gradient::Linear(
        Linear::new(Radians(angle))
            .add_stop(0.0, from)
            .add_stop(1.0, to),
    ))
}

fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

fn border(color: Color, width: f32, radius: f32) -> Border {
    Border {
        color,
        width,
        radius: radius.into(),
    }
}

fn glow(color: Color, blur: f32, dy: f32) -> Shadow {
    Shadow {
        color,
        offset: Vector::new(0.0, dy),
        blur_radius: blur,
    }
}

// ===========================================================================
// Containers
// ===========================================================================

/// Full-window backdrop: cold indigo at the summit fading to black.
struct Root;
impl container::StyleSheet for Root {
    type Style = Theme;
    fn appearance(&self, _: &Theme) -> container::Appearance {
        container::Appearance {
            text_color: None,
            background: Some(linear(PI, BG_TOP, BG_BOTTOM)),
            border: Border::default(),
            shadow: Shadow::default(),
        }
    }
}
pub fn root() -> theme::Container {
    theme::Container::Custom(Box::new(Root))
}

/// The glassy central panel.
struct Card;
impl container::StyleSheet for Card {
    type Style = Theme;
    fn appearance(&self, _: &Theme) -> container::Appearance {
        container::Appearance {
            text_color: Some(TEXT),
            background: Some(Background::Color(rgba(0x0E, 0x14, 0x24, 0.72))),
            border: border(alpha(CYAN, 0.16), 1.0, 22.0),
            shadow: glow(alpha(Color::BLACK, 0.55), 40.0, 16.0),
        }
    }
}
pub fn card() -> theme::Container {
    theme::Container::Custom(Box::new(Card))
}

/// Recessed sub-surface (fingerprint chip, license info block).
struct Inset;
impl container::StyleSheet for Inset {
    type Style = Theme;
    fn appearance(&self, _: &Theme) -> container::Appearance {
        container::Appearance {
            text_color: Some(TEXT),
            background: Some(Background::Color(rgba(0x05, 0x09, 0x12, 0.85))),
            border: border(alpha(MUTED, 0.14), 1.0, 12.0),
            shadow: Shadow::default(),
        }
    }
}
pub fn inset() -> theme::Container {
    theme::Container::Custom(Box::new(Inset))
}

/// Status pill — cyan when ok, magenta when not.
struct Pill(bool);
impl container::StyleSheet for Pill {
    type Style = Theme;
    fn appearance(&self, _: &Theme) -> container::Appearance {
        let accent = if self.0 { CYAN } else { MAGENTA };
        container::Appearance {
            text_color: Some(accent),
            background: Some(Background::Color(alpha(accent, 0.10))),
            border: border(alpha(accent, 0.30), 1.0, 9.0),
            shadow: Shadow::default(),
        }
    }
}
pub fn pill(ok: bool) -> theme::Container {
    theme::Container::Custom(Box::new(Pill(ok)))
}

/// Thin luminous divider (cyan → magenta).
struct Divider;
impl container::StyleSheet for Divider {
    type Style = Theme;
    fn appearance(&self, _: &Theme) -> container::Appearance {
        container::Appearance {
            text_color: None,
            background: Some(Background::Gradient(Gradient::Linear(
                Linear::new(Radians(FRAC_PI_2))
                    .add_stop(0.0, alpha(CYAN, 0.0))
                    .add_stop(0.25, alpha(CYAN, 0.6))
                    .add_stop(0.75, alpha(MAGENTA, 0.5))
                    .add_stop(1.0, alpha(MAGENTA, 0.0)),
            ))),
            border: Border::default(),
            shadow: Shadow::default(),
        }
    }
}
pub fn divider() -> theme::Container {
    theme::Container::Custom(Box::new(Divider))
}

// ===========================================================================
// Text inputs
// ===========================================================================

struct Field;
impl text_input::StyleSheet for Field {
    type Style = Theme;

    fn active(&self, _: &Theme) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(rgba(0x05, 0x09, 0x12, 0.9)),
            border: border(alpha(MUTED, 0.22), 1.0, 11.0),
            icon_color: MUTED,
        }
    }
    fn focused(&self, _: &Theme) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(rgba(0x06, 0x0C, 0x16, 0.95)),
            border: border(CYAN, 1.5, 11.0),
            icon_color: CYAN,
        }
    }
    fn hovered(&self, _: &Theme) -> text_input::Appearance {
        text_input::Appearance {
            border: border(alpha(CYAN, 0.45), 1.0, 11.0),
            ..self.active(&Theme::Dark)
        }
    }
    fn disabled(&self, _: &Theme) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(rgba(0x05, 0x09, 0x12, 0.6)),
            border: border(alpha(MUTED, 0.12), 1.0, 11.0),
            icon_color: MUTED,
        }
    }
    fn placeholder_color(&self, _: &Theme) -> Color {
        alpha(MUTED, 0.55)
    }
    fn value_color(&self, _: &Theme) -> Color {
        TEXT
    }
    fn disabled_color(&self, _: &Theme) -> Color {
        MUTED
    }
    fn selection_color(&self, _: &Theme) -> Color {
        alpha(CYAN, 0.30)
    }
}
pub fn field() -> theme::TextInput {
    theme::TextInput::Custom(Box::new(Field))
}

// ===========================================================================
// Buttons
// ===========================================================================

/// Primary action — glacial cyan, dark ink label, cyan glow.
struct Activate;
impl button::StyleSheet for Activate {
    type Style = Theme;
    fn active(&self, _: &Theme) -> button::Appearance {
        button::Appearance {
            background: Some(linear(FRAC_PI_2, CYAN, CYAN_DEEP)),
            text_color: INK,
            border: border(Color::TRANSPARENT, 0.0, 12.0),
            shadow: glow(alpha(CYAN, 0.35), 18.0, 6.0),
            shadow_offset: Vector::new(0.0, 1.0),
        }
    }
    fn hovered(&self, _: &Theme) -> button::Appearance {
        button::Appearance {
            background: Some(linear(FRAC_PI_2, CYAN_BRIGHT, CYAN)),
            shadow: glow(alpha(CYAN, 0.5), 26.0, 8.0),
            ..self.active(&Theme::Dark)
        }
    }
    fn disabled(&self, _: &Theme) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(alpha(CYAN, 0.14))),
            text_color: alpha(TEXT, 0.55),
            border: border(alpha(CYAN, 0.25), 1.0, 12.0),
            shadow: Shadow::default(),
            shadow_offset: Vector::new(0.0, 0.0),
        }
    }
}
pub fn activate() -> theme::Button {
    theme::Button::Custom(Box::new(Activate))
}

/// Hero action — magenta ignition with glow, only enabled once the license is
/// valid. Renders its `disabled` (ghost) appearance automatically when no
/// `on_press` is wired.
struct Launch;
impl button::StyleSheet for Launch {
    type Style = Theme;
    fn active(&self, _: &Theme) -> button::Appearance {
        button::Appearance {
            background: Some(linear(FRAC_PI_2, MAGENTA, MAGENTA_SOFT)),
            text_color: Color::WHITE,
            border: border(Color::TRANSPARENT, 0.0, 13.0),
            shadow: glow(alpha(MAGENTA, 0.45), 26.0, 8.0),
            shadow_offset: Vector::new(0.0, 2.0),
        }
    }
    fn hovered(&self, _: &Theme) -> button::Appearance {
        button::Appearance {
            background: Some(linear(FRAC_PI_2, MAGENTA_SOFT, MAGENTA)),
            shadow: glow(alpha(MAGENTA, 0.6), 34.0, 10.0),
            ..self.active(&Theme::Dark)
        }
    }
    fn disabled(&self, _: &Theme) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(rgba(0x12, 0x16, 0x22, 0.7))),
            text_color: alpha(MUTED, 0.7),
            border: border(alpha(MUTED, 0.16), 1.0, 13.0),
            shadow: Shadow::default(),
            shadow_offset: Vector::new(0.0, 0.0),
        }
    }
}
pub fn launch() -> theme::Button {
    theme::Button::Custom(Box::new(Launch))
}
