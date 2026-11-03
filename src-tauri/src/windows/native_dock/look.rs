//! The styles of the bar (spec 0005 §3.2): each style's own look, the
//! user's changes to it, and its colors (the blocks of `src/dock/Dock.css`
//! and `src/dock/look.ts` the page's dock read).

use super::theme::{Color, Palette};
use crate::domain::settings::{DockSettings, DockStyle};
use crate::domain::skin::SkinMeta;

/// How the bar is filled: one color, or bands of a vertical gradient (from
/// the top), drawn over a base color.
#[derive(Debug, Clone, PartialEq)]
pub enum Fill {
    Solid(Color),
    Gradient {
        base: Option<Color>,
        stops: Vec<(f32, Color)>,
        /// A second gradient over the first.
        over: Vec<(f32, Color)>,
    },
}

impl Fill {
    /// Whether nothing is drawn (the bar is see-through): a breath of alpha
    /// keeps the mouse on it anyway.
    pub fn is_clear(&self) -> bool {
        matches!(self, Fill::Solid(color) if color.a < 0.01)
    }
}

/// A soft light around the bar: its color and how far it reaches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glow {
    pub color: Color,
    pub reach: f32,
}

/// Everything the bar's paint reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub style: DockStyle,
    pub border: bool,
    /// How round the bar is, 0–1 of a full pill.
    pub corners: f64,
    pub reflections: bool,
    /// The style's own color of the mark under an item, or the accent.
    pub indicator: Option<Color>,
    pub fill: Option<Fill>,
    pub separator: Option<Color>,
    /// The ink of the clock at the end of the bar.
    pub ink: Option<Color>,
    /// The light edge; `Some(transparent)` takes it away.
    pub hi: Option<Color>,
    pub glows: Vec<Glow>,
    /// The bar draws no shadow (the floating styles).
    pub flat: bool,
    /// A gradient outline instead of the hairline (Neon).
    pub outline: Option<(Color, Color)>,
    /// The silver shelf under the tiles (Prateleira Aqua).
    pub shelf: bool,
    /// The bar is the style's skin (spec 0020 §3): its image, not a fill,
    /// an edge or a border.
    pub skin: bool,
}

/// The parts of a style the user can change, as each style has them.
fn own(style: DockStyle) -> (bool, u8, bool) {
    match style {
        DockStyle::Native => (true, 65, false),
        DockStyle::LiquidGlass => (true, 100, false),
        DockStyle::Midnight => (true, 25, false),
        DockStyle::Rocket => (true, 25, false),
        DockStyle::AquaShelf => (false, 0, true),
        DockStyle::Neon => (true, 70, false),
        DockStyle::Paper => (false, 40, false),
        DockStyle::Floating => (false, 40, true),
        // The image is the bar: no border and no corners to change.
        DockStyle::Retro
        | DockStyle::Classic98
        | DockStyle::Phosphor
        | DockStyle::Tactical
        | DockStyle::Medieval
        | DockStyle::Hunter
        | DockStyle::Cyberpunk
        | DockStyle::Arcade
        | DockStyle::Scifi
        | DockStyle::Custom => (false, 0, false),
    }
}

const WHITE: Color = Color::WHITE;

/// The bar in Windows' contrast mode (M6), whatever the style: the
/// palette's colors only, with a border; no skin, glass, glow,
/// reflection or shadow.
pub fn contrast() -> Look {
    Look {
        style: DockStyle::Native,
        border: true,
        corners: 0.65,
        reflections: false,
        indicator: None,
        fill: None,
        separator: None,
        ink: None,
        hi: None,
        glows: Vec::new(),
        flat: true,
        outline: None,
        shelf: false,
        skin: false,
    }
}

impl Look {
    /// The look of the bar for `settings`. `custom` is the folder skin
    /// `custom_skin` names, as read (spec 0020, Q14); with
    /// [`DockStyle::Custom`] and none, the bar is Nativo's.
    pub fn of(settings: &DockSettings, custom: Option<&SkinMeta>) -> Look {
        let style = match (settings.style, custom) {
            (DockStyle::Custom, None) => DockStyle::Native,
            (style, _) => style,
        };
        // A folder skin's own colors, when its `skin.json` gives them.
        let color_of = |pick: fn(&SkinMeta) -> Option<&String>| {
            custom.and_then(pick).and_then(|hex| Color::hex(hex))
        };
        let custom_ink = color_of(|meta| meta.ink.as_ref());
        let custom_indicator = color_of(|meta| meta.indicator.as_ref());
        let tweaks = settings.tweaks.get(&style);
        let (border, corners, reflections) = own(style);
        let base = Look {
            style,
            border: tweaks.and_then(|t| t.border).unwrap_or(border),
            corners: f64::from(tweaks.and_then(|t| t.corners).unwrap_or(corners).min(100)) / 100.0,
            reflections: tweaks.and_then(|t| t.reflections).unwrap_or(reflections),
            indicator: None,
            fill: None,
            separator: None,
            ink: None,
            hi: None,
            glows: Vec::new(),
            flat: false,
            outline: None,
            shelf: false,
            skin: false,
        };
        match style {
            DockStyle::Native => base,
            DockStyle::LiquidGlass => Look {
                fill: Some(Fill::Gradient {
                    base: Some(Color::rgba(32, 38, 56, 0.5)),
                    stops: vec![(0.0, WHITE.with_alpha(0.24)), (1.0, WHITE.with_alpha(0.07))],
                    over: Vec::new(),
                }),
                separator: Some(WHITE.with_alpha(0.28)),
                ink: Some(WHITE),
                glows: vec![Glow {
                    color: WHITE.with_alpha(0.18),
                    reach: 6.0,
                }],
                indicator: Some(WHITE),
                ..base
            },
            DockStyle::Midnight => Look {
                fill: Some(Fill::Solid(Color::rgba(8, 10, 16, 0.93))),
                separator: Some(WHITE.with_alpha(0.2)),
                ink: Some(WHITE),
                indicator: Some(WHITE),
                ..base
            },
            DockStyle::Rocket => Look {
                fill: Some(Fill::Solid(Color::rgba(9, 14, 32, 0.95))),
                separator: Some(Color::rgba(120, 160, 255, 0.3)),
                ink: Some(WHITE),
                glows: vec![Glow {
                    color: Color::rgba(80, 130, 255, 0.6),
                    reach: 7.0,
                }],
                indicator: Some(Color::rgb(0x8f, 0xb0, 0xff)),
                ..base
            },
            DockStyle::Neon => Look {
                fill: Some(Fill::Solid(Color::rgba(6, 6, 12, 0.96))),
                separator: Some(WHITE.with_alpha(0.2)),
                ink: Some(WHITE),
                glows: vec![
                    Glow {
                        color: Color::rgba(0, 229, 255, 0.55),
                        reach: 6.0,
                    },
                    Glow {
                        color: Color::rgba(255, 43, 214, 0.35),
                        reach: 7.0,
                    },
                ],
                outline: base
                    .border
                    .then_some((Color::rgb(0x00, 0xe5, 0xff), Color::rgb(0xff, 0x2b, 0xd6))),
                indicator: Some(Color::rgb(0x00, 0xe5, 0xff)),
                ..base
            },
            DockStyle::Paper => Look {
                fill: Some(Fill::Solid(Color::rgb(247, 247, 243))),
                separator: Some(Color::rgba(0, 0, 0, 0.14)),
                ink: Some(Color::rgb(0x1d, 0x1e, 0x22)),
                hi: Some(Color::CLEAR),
                indicator: Some(Color::rgb(0x2c, 0x3a, 0x5a)),
                ..base
            },
            DockStyle::Floating => Look {
                fill: Some(Fill::Solid(Color::CLEAR)),
                separator: Some(WHITE.with_alpha(0.25)),
                ink: Some(WHITE),
                hi: Some(Color::CLEAR),
                flat: true,
                indicator: Some(WHITE),
                ..base
            },
            DockStyle::AquaShelf => Look {
                fill: Some(Fill::Solid(Color::CLEAR)),
                separator: Some(WHITE.with_alpha(0.3)),
                ink: Some(WHITE),
                hi: Some(Color::CLEAR),
                flat: true,
                shelf: true,
                indicator: Some(Color::rgb(0x2c, 0x3a, 0x5a)),
                ..base
            },
            DockStyle::Retro => Look {
                skin: true,
                border: false,
                separator: Some(Color::rgb(0x5f, 0x57, 0x4f)),
                ink: Some(Color::rgb(0xff, 0xf1, 0xe8)),
                indicator: Some(Color::rgb(0xff, 0x00, 0x4d)),
                ..base
            },
            DockStyle::Classic98 => Look {
                skin: true,
                border: false,
                separator: Some(Color::rgb(0x80, 0x80, 0x80)),
                ink: Some(Color::rgb(0x00, 0x00, 0x00)),
                indicator: Some(Color::rgb(0x00, 0x00, 0x80)),
                ..base
            },
            DockStyle::Phosphor => Look {
                skin: true,
                border: false,
                separator: Some(Color::rgb(0x1f, 0x7a, 0x3a)),
                ink: Some(Color::rgb(0x3c, 0xff, 0x6e)),
                indicator: Some(Color::rgb(0x3c, 0xff, 0x6e)),
                ..base
            },
            DockStyle::Tactical => Look {
                skin: true,
                border: false,
                separator: Some(Color::rgba(0xd9, 0xc9, 0xa3, 0.3)),
                ink: Some(Color::rgb(0xd9, 0xc9, 0xa3)),
                indicator: Some(Color::rgb(0xff, 0x8a, 0x1f)),
                ..base
            },
            DockStyle::Medieval => Look {
                skin: true,
                border: false,
                separator: Some(Color::rgb(0x6b, 0x4a, 0x2e)),
                ink: Some(Color::rgb(0xe8, 0xd9, 0xb0)),
                indicator: Some(Color::rgb(0xe8, 0xb8, 0x4a)),
                ..base
            },
            DockStyle::Hunter => Look {
                skin: true,
                border: false,
                separator: Some(Color::rgba(0xe8, 0xd2, 0xa6, 0.4)),
                ink: Some(Color::rgb(0xe8, 0xd2, 0xa6)),
                indicator: Some(Color::rgb(0xf2, 0xa3, 0x3a)),
                ..base
            },
            DockStyle::Cyberpunk => Look {
                skin: true,
                border: false,
                separator: Some(Color::rgba(0xff, 0x2b, 0xd6, 0.35)),
                ink: Some(Color::rgb(0x2e, 0xf2, 0xff)),
                indicator: Some(Color::rgb(0xff, 0x2b, 0xd6)),
                ..base
            },
            DockStyle::Arcade => Look {
                skin: true,
                border: false,
                separator: Some(Color::rgb(0x5c, 0x61, 0x78)),
                // White, not the yellow of the marks: the date crosses the stripes.
                ink: Some(Color::rgb(0xf2, 0xf2, 0xf7)),
                indicator: Some(Color::rgb(0xff, 0xd2, 0x3f)),
                ..base
            },
            DockStyle::Scifi => Look {
                skin: true,
                border: false,
                separator: Some(Color::rgba(0x4f, 0xd8, 0xff, 0.3)),
                ink: Some(Color::rgb(0x9f, 0xb2, 0xcc)),
                indicator: Some(Color::rgb(0x4f, 0xd8, 0xff)),
                ..base
            },
            // The separators in the skin's ink, faint; the dock's own
            // colors where the skin gives none.
            DockStyle::Custom => Look {
                skin: true,
                border: false,
                separator: custom_ink.map(|ink| ink.with_alpha(0.35)),
                ink: custom_ink,
                indicator: custom_indicator,
                ..base
            },
        }
    }

    /// The bar's fill in this palette.
    pub fn fill(&self, palette: &Palette) -> Fill {
        self.fill.clone().unwrap_or(Fill::Solid(palette.surface))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::settings::StyleTweaks;

    fn with_style(style: DockStyle) -> DockSettings {
        DockSettings {
            style,
            ..DockSettings::default()
        }
    }

    #[test]
    fn the_contrast_look_draws_no_picture_of_its_own() {
        let look = contrast();
        assert!(look.border && !look.skin && !look.reflections && !look.shelf);
        assert!(look.glows.is_empty() && look.outline.is_none() && look.fill.is_none());
    }

    #[test]
    fn each_style_has_its_own_look() {
        let native = Look::of(&with_style(DockStyle::Native), None);
        assert!(native.border);
        assert!((native.corners - 0.65).abs() < 1e-9);
        assert!(!native.reflections);
        assert_eq!(native.fill, None);

        let shelf = Look::of(&with_style(DockStyle::AquaShelf), None);
        assert!(!shelf.border && shelf.reflections && shelf.shelf && shelf.flat);
        assert!(
            shelf
                .fill(&Palette::of(false, Default::default()))
                .is_clear()
        );

        let neon = Look::of(&with_style(DockStyle::Neon), None);
        assert!(neon.outline.is_some());
        assert_eq!(neon.glows.len(), 2);
    }

    #[test]
    fn the_users_changes_go_over_the_style() {
        let mut settings = with_style(DockStyle::Neon);
        settings.tweaks.insert(
            DockStyle::Neon,
            StyleTweaks {
                border: Some(false),
                corners: Some(10),
                reflections: Some(true),
            },
        );

        let look = Look::of(&settings, None);

        assert!(!look.border && look.reflections);
        assert!((look.corners - 0.1).abs() < 1e-9);
        // Without the border the gradient outline goes too.
        assert_eq!(look.outline, None);
    }

    fn folder_meta(ink: Option<&str>, indicator: Option<&str>) -> SkinMeta {
        serde_json::from_value(serde_json::json!({
            "name": "Minha", "image": "skin.png", "width": 8, "height": 8,
            "slice": {"top": 0, "right": 2, "bottom": 0, "left": 2},
            "middle": "repeat", "pixelated": false,
            "ink": ink, "indicator": indicator,
        }))
        .unwrap()
    }

    #[test]
    fn a_folder_skin_is_the_bar_in_its_own_colors() {
        let mut settings = with_style(DockStyle::Custom);
        settings.custom_skin = Some("Minha".into());
        // A tweak of the border can't put one on an image.
        settings.tweaks.insert(
            DockStyle::Custom,
            StyleTweaks {
                border: Some(true),
                ..StyleTweaks::default()
            },
        );
        let meta = folder_meta(Some("#102030"), Some("#ff8000"));

        let look = Look::of(&settings, Some(&meta));

        assert_eq!(look.style, DockStyle::Custom);
        assert!(look.skin && !look.border && !look.reflections);
        assert_eq!(look.ink, Some(Color::rgb(0x10, 0x20, 0x30)));
        assert_eq!(look.indicator, Some(Color::rgb(0xff, 0x80, 0x00)));
        assert_eq!(
            look.separator,
            Some(Color::rgb(0x10, 0x20, 0x30).with_alpha(0.35))
        );

        // Colors it does not give, or gives wrong, are the dock's own.
        let plain = Look::of(&settings, Some(&folder_meta(None, Some("orange"))));
        assert!(plain.skin);
        assert_eq!(
            (plain.ink, plain.indicator, plain.separator),
            (None, None, None)
        );
    }

    #[test]
    fn a_folder_skin_that_can_t_be_read_paints_the_bar_as_nativo() {
        let mut settings = with_style(DockStyle::Custom);
        settings.custom_skin = Some("Sumiu".into());

        let look = Look::of(&settings, None);

        assert_eq!(look, Look::of(&with_style(DockStyle::Native), None));
        assert!(!look.skin);
    }
}
