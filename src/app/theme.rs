//! The chrome palette's tokens, the kit pinned to them, shared surfaces (chips, glass,
//! cards) and the app's icons.
use rex_linux::settings::ChromeTheme;
use rune::{Color, El, Icon, Shadow, Spring, SpringPair, TextStyle, Transition};
use rune_kit::{Kit, PaletteFocus, PaletteSeed};
use std::sync::OnceLock;

/// sRGB `0xRRGGBB` → linear `Color`.
pub fn rgb(value: u32) -> Color {
    let channel = |shift: u32| rune::color::srgb_decode(((value >> shift) & 0xff) as f32 / 255.);
    Color {
        r: channel(16),
        g: channel(8),
        b: channel(0),
        a: 1.,
    }
}

/// `a` over `b` at opacity `t`, mixed in sRGB.
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let channel = |shift: u32| {
        let (x, y) = (((a >> shift) & 0xff) as f32, ((b >> shift) & 0xff) as f32);
        ((x * t + y * (1. - t)).round() as u32) << shift
    };
    channel(16) | channel(8) | channel(0)
}

pub fn white(a: f32) -> Color {
    Color::WHITE.alpha(a)
}
pub fn black(a: f32) -> Color {
    Color::BLACK.alpha(a)
}
pub fn shadow(x: f32, y: f32, blur: f32, color: Color) -> Shadow {
    Shadow {
        x,
        y,
        blur,
        spread: 0.,
        color,
        inset: false,
    }
}
pub fn inset(y: f32, color: Color) -> Shadow {
    Shadow {
        x: 0.,
        y,
        blur: 0.,
        spread: 0.,
        color,
        inset: true,
    }
}

/// One chrome palette: the colour tokens, and the kit pinned to them, built once.
pub struct Palette {
    pub ground: u32,
    /// The card, and the terminal's default background (the server's default black is
    /// replaced). The same colour as the ground, so a single pane and the toolbar read as one
    /// surface.
    pub card: u32,
    pub popover: u32,
    pub ink: u32,
    pub muted: u32,
    pub accent: u32,
    pub accent2: u32,
    pub live: u32,
    pub warning: u32,
    pub danger: u32,
    /// Text on an accent fill (primary buttons).
    pub on_accent: u32,
    pub terminal_selection: u32,
    kit: OnceLock<Kit>,
}

pub static MIDNIGHT: Palette = Palette {
    ground: 0x001528,
    card: 0x001528,
    popover: 0x0d1524,
    ink: 0xe8eef6,
    muted: 0x93a3b8,
    accent: 0x7cc4ff,
    accent2: 0xa78bfa,
    live: 0x5eead4,
    warning: 0xf5d48a,
    danger: 0xff8a8a,
    on_accent: 0x061220,
    terminal_selection: 0x2f4a6e,
    kit: OnceLock::new(),
};

/// Rosé Pine's roles: base, surface, text, subtle, iris, foam, pine, gold, love, highlight med.
pub static ROSE_PINE: Palette = Palette {
    ground: 0x191724,
    card: 0x191724,
    popover: 0x1f1d2e,
    ink: 0xe0def4,
    muted: 0x908caa,
    accent: 0xc4a7e7,
    accent2: 0x9ccfd8,
    live: 0x31748f,
    warning: 0xf6c177,
    danger: 0xeb6f92,
    on_accent: 0x191724,
    terminal_selection: 0x403d52,
    kit: OnceLock::new(),
};

/// Rosé Pine Moon's roles, as for Rosé Pine.
pub static ROSE_PINE_MOON: Palette = Palette {
    ground: 0x232136,
    card: 0x232136,
    popover: 0x2a273f,
    ink: 0xe0def4,
    muted: 0x908caa,
    accent: 0xc4a7e7,
    accent2: 0x9ccfd8,
    live: 0x3e8fb0,
    warning: 0xf6c177,
    danger: 0xeb6f92,
    on_accent: 0x232136,
    terminal_selection: 0x44415a,
    kit: OnceLock::new(),
};

/// Gruvbox Dark: bg, bg1, fg, gray, blue, purple, green, yellow, red, bg2.
pub static GRUVBOX_DARK: Palette = Palette {
    ground: 0x282828,
    card: 0x282828,
    popover: 0x3c3836,
    ink: 0xebdbb2,
    muted: 0xa89984,
    accent: 0x83a598,
    accent2: 0xd3869b,
    live: 0xb8bb26,
    warning: 0xfabd2f,
    danger: 0xfb4934,
    on_accent: 0x282828,
    terminal_selection: 0x504945,
    kit: OnceLock::new(),
};

/// The palette a chrome theme draws in.
pub fn palette_of(chrome: ChromeTheme) -> &'static Palette {
    match chrome {
        ChromeTheme::Midnight => &MIDNIGHT,
        ChromeTheme::RosePine => &ROSE_PINE,
        ChromeTheme::RosePineMoon => &ROSE_PINE_MOON,
        ChromeTheme::GruvboxDark => &GRUVBOX_DARK,
    }
}

thread_local! {
    // Per thread: Rune builds a window's view on its UI thread, and headless runtimes on
    // other threads keep their own. The root view sets it before every build.
    static ACTIVE: std::cell::Cell<&'static Palette> = const { std::cell::Cell::new(&MIDNIGHT) };
}

/// The palette views on this thread draw in.
pub fn palette() -> &'static Palette {
    ACTIVE.get()
}
pub fn set_palette(palette: &'static Palette) {
    ACTIVE.set(palette);
}

pub fn ground() -> u32 {
    palette().ground
}
pub fn card() -> u32 {
    palette().card
}
pub fn popover() -> u32 {
    palette().popover
}
pub fn ink() -> u32 {
    palette().ink
}
pub fn muted() -> u32 {
    palette().muted
}
pub fn accent() -> u32 {
    palette().accent
}
pub fn accent2() -> u32 {
    palette().accent2
}
pub fn warning() -> u32 {
    palette().warning
}
pub fn danger() -> u32 {
    palette().danger
}
pub fn on_accent() -> u32 {
    palette().on_accent
}

/// How much of its tone the `SELECTION` fill is.
pub const SELECTION_FILL: f32 = 0.18;

/// `SELECTION`, the keyboard-selected row of a list, as (fill, outline): a tint of `ACCENT`
/// under a 1 pt outline in it, so selection is never fill alone and never reads as a hover. A
/// destructive row tints in `DANGER` instead. Text on it is `INK`.
pub fn selection(destructive: bool) -> (Color, Color) {
    let tone = rgb(if destructive { danger() } else { accent() });
    (tone.alpha(SELECTION_FILL), tone)
}

/// Terminal search marks, drawn over the cells as the selection is: every match is a tint of
/// `ACCENT` at `SEARCH_MATCH` under a 1 pt outline in it at `SEARCH_OUTLINE`, and the selected
/// one is `ACCENT` at `SEARCH_CURRENT` with its text in `ON_ACCENT`. The tint is light because
/// a match keeps the terminal's own text colour, dim ones included: the outline is what marks
/// it.
pub const SEARCH_MATCH: f32 = 0.18;
pub const SEARCH_OUTLINE: f32 = 0.7;
pub const SEARCH_CURRENT: f32 = 0.8;
pub const SEARCH_RADIUS: f32 = 2.5;

/// A keycap's chip: white at this opacity over its surface, and `KEYCAP_LIT` on a selected row.
pub const KEYCAP_FILL: f32 = 0.07;
pub const KEYCAP_LIT: f32 = 0.12;

/// A keycap's text: halfway from `MUTED` to `INK`, so that 10.5 pt keys meet 4.5:1 on their
/// chip in every palette. `MUTED` alone does not on the lighter popovers.
pub fn keycap_ink() -> u32 {
    mix(ink(), muted(), 0.5)
}

pub const TOOLBAR: f32 = 52.;
pub const GUTTER: f32 = 8.;
pub const MARGIN: f32 = 10.;
pub const CARD_RADIUS: f32 = 12.;
pub const CARD_HEADER: f32 = 30.;

/// Springs for chrome: quick and calm.
pub const SNAPPY: Spring = Spring {
    duration: 0.28,
    bounce: 0.0,
};
pub const POP: Spring = Spring {
    duration: 0.32,
    bounce: 0.12,
};
/// For things that must not move (terminal content).
pub const STILL: Spring = Spring {
    duration: 0.001,
    bounce: 0.,
};

/// A sliding highlight's own entrance and exit: none, so one highlight shows even when its
/// matched partner isn't mounted (Reduce Motion, a row scrolled in).
pub fn no_presence() -> Transition {
    Transition::identity().springs(SpringPair::new(STILL, STILL))
}

/// A popover's entrance from its anchor: a little scale and a fade on a gentle spring.
pub fn pop_in() -> Transition {
    Transition::scale(0.96).springs(SpringPair::new(POP, SNAPPY))
}

/// The active palette's kit.
pub fn kit() -> &'static Kit {
    let p = palette();
    p.kit.get_or_init(|| {
        let mut k = Kit::from_palette_focus(
            PaletteSeed {
                bg: rgb(p.ground),
                fg: rgb(p.ink),
                accent: rgb(p.accent),
            },
            PaletteFocus::Custom(rgb(p.accent)),
        );
        let c = &mut k.c;
        (c.page, c.bg_base, c.surface_base) = (rgb(p.ground), rgb(p.ground), rgb(p.ground));
        c.bg_deep = rgb(p.card);
        for slot in [
            &mut c.layer_01,
            &mut c.layer_02,
            &mut c.surface_raised,
            &mut c.surface_raised_base,
            &mut c.surface_float,
        ] {
            *slot = rgb(p.popover);
        }
        (c.hover, c.surface_hover, c.surface_raised_hover) =
            (white(0.07), white(0.07), white(0.07));
        c.pressed = white(0.11);
        (c.border_base, c.border_muted, c.border_weak) = (white(0.12), white(0.08), white(0.06));
        (c.border_strong, c.border_hover) = (white(0.2), white(0.2));
        c.border_focus = rgb(p.accent);
        (c.text_base, c.text_strong, c.icon_base) = (rgb(p.ink), rgb(p.ink), rgb(p.ink));
        for slot in [
            &mut c.text_muted,
            &mut c.text_faint,
            &mut c.text_weak,
            &mut c.icon_muted,
            &mut c.icon_faint,
            &mut c.icon_weaker,
        ] {
            *slot = rgb(p.muted);
        }
        (c.text_accent, c.icon_accent) = (rgb(p.accent), rgb(p.accent));
        c.success.fg = rgb(p.live);
        c.warning.fg = rgb(p.warning);
        c.danger.fg = rgb(p.danger);
        k.radius.sm = 8.;
        k.radius.md = 10.;
        k
    })
}

pub fn ui(size: f32) -> TextStyle {
    TextStyle::new(size, 450.)
}
pub fn medium(size: f32) -> TextStyle {
    TextStyle::new(size, 560.)
}
pub fn strong(size: f32) -> TextStyle {
    TextStyle::new(size, 620.)
}
/// Small capitals for group labels.
pub fn label_caps() -> TextStyle {
    TextStyle::new(10.5, 650.).tracking(0.6)
}
/// Code-like values in chrome (paths, commits, endpoints, ids, commands): the terminal's font
/// stack, which ends in the bundled IBM Plex Mono.
pub fn mono(size: f32) -> TextStyle {
    TextStyle::new(size, 450.).mono()
}

/// A glassy pill every control in the bar sits on.
pub fn chip(e: El) -> El {
    e.fill(white(0.055))
        .border_gradient(1., white(0.13), white(0.04))
        .radius(10.)
        .continuous_corners()
}

/// A popover or dialog surface: the frosted popover colour with a lit rim and a deep shadow.
pub fn glass(e: El, radius: f32, blur: bool) -> El {
    let e = e
        .fill(rgb(popover()).alpha(0.95))
        .border_gradient(1., white(0.16), white(0.05))
        .radius(radius)
        .continuous_corners()
        .box_shadow(shadow(0., 24., 60., black(0.6)))
        .box_shadow(shadow(0., 2., 6., black(0.35)));
    if blur { e.backdrop_blur(22.) } else { e }
}

macro_rules! icon {
    ($name:ident, $id:literal, $($el:literal),+) => {
        pub static $name: Icon = Icon {
            name: $id,
            view: 20.0,
            body: concat!($("<", $el, r#" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#),+),
        };
    };
}
icon!(
    TERMINAL,
    "rex-terminal",
    r#"path d="m4 6 4 4-4 4""#,
    r#"path d="M10.5 14.5h5.5""#
);
icon!(CHEVRON, "rex-chevron", r#"path d="m7 8.5 3 3 3-3""#);
// The find bar's steps: wider than the disclosure chevron, to sit beside the close cross.
icon!(STEP_UP, "rex-step-up", r#"path d="m6 12 4-4 4 4""#);
icon!(STEP_DOWN, "rex-step-down", r#"path d="m6 8 4 4 4-4""#);
icon!(RIGHT, "rex-right", r#"path d="m8.5 6.5 3.5 3.5-3.5 3.5""#);
icon!(LEFT, "rex-left", r#"path d="m11.5 6.5-3.5 3.5 3.5 3.5""#);
icon!(CHECK, "rex-check", r#"path d="m5.5 10.5 3 3 6-7""#);
icon!(
    PLUS,
    "rex-plus",
    r#"path d="M10 4.5v11""#,
    r#"path d="M4.5 10h11""#
);
icon!(
    CLOSE,
    "rex-close",
    r#"path d="m6.5 6.5 7 7""#,
    r#"path d="m13.5 6.5-7 7""#
);
icon!(
    PENCIL,
    "rex-pencil",
    r#"path d="M12.5 5.5 14.5 7.5 8 14l-2.75.75L6 12z""#
);
icon!(
    SEARCH,
    "rex-search",
    r#"circle cx="9" cy="9" r="5""#,
    r#"path d="m13 13 3.5 3.5""#
);
icon!(
    COMMAND,
    "rex-command",
    r#"path d="M12.5 5v10a2.5 2.5 0 1 0 2.5-2.5H5a2.5 2.5 0 1 0 2.5 2.5V5a2.5 2.5 0 1 0-2.5 2.5h10a2.5 2.5 0 1 0-2.5-2.5""#
);
// A gear: eight teeth around a hub. Sliders read as view options, not Settings.
icon!(
    SETTINGS,
    "rex-settings",
    r#"path d="M8.69 4.55L9.02 2.56A7.5 7.5 0 0 1 10.98 2.56L11.31 4.55A5.6 5.6 0 0 1 12.93 5.23L14.57 4.05A7.5 7.5 0 0 1 15.95 5.43L14.77 7.07A5.6 5.6 0 0 1 15.45 8.69L17.44 9.02A7.5 7.5 0 0 1 17.44 10.98L15.45 11.31A5.6 5.6 0 0 1 14.77 12.93L15.95 14.57A7.5 7.5 0 0 1 14.57 15.95L12.93 14.77A5.6 5.6 0 0 1 11.31 15.45L10.98 17.44A7.5 7.5 0 0 1 9.02 17.44L8.69 15.45A5.6 5.6 0 0 1 7.07 14.77L5.43 15.95A7.5 7.5 0 0 1 4.05 14.57L5.23 12.93A5.6 5.6 0 0 1 4.55 11.31L2.56 10.98A7.5 7.5 0 0 1 2.56 9.02L4.55 8.69A5.6 5.6 0 0 1 5.23 7.07L4.05 5.43A7.5 7.5 0 0 1 5.43 4.05L7.07 5.23A5.6 5.6 0 0 1 8.69 4.55Z""#,
    r#"circle cx="10" cy="10" r="2.4""#
);
icon!(
    MONITOR,
    "rex-monitor",
    r#"rect x="3" y="4" width="14" height="9.5" rx="2""#,
    r#"path d="M10 13.5v3""#,
    r#"path d="M7 16.5h6""#
);
icon!(
    SERVER,
    "rex-server",
    r#"rect x="3.5" y="4" width="13" height="5" rx="1.5""#,
    r#"rect x="3.5" y="11" width="13" height="5" rx="1.5""#,
    r#"path d="M6.5 6.5h.01""#,
    r#"path d="M6.5 13.5h.01""#
);
// Add host's groups: the machines of a tailnet, and the ones that are away.
icon!(
    NETWORK,
    "rex-network",
    r#"circle cx="10" cy="5" r="2""#,
    r#"circle cx="4.75" cy="14.5" r="2""#,
    r#"circle cx="15.25" cy="14.5" r="2""#,
    r#"path d="m9 6.75-3.25 6""#,
    r#"path d="m11 6.75 3.25 6""#,
    r#"path d="M6.75 14.5h6.5""#
);
icon!(
    MOON,
    "rex-moon",
    r#"path d="M16 11.75A6.5 6.5 0 1 1 8.25 4 5.25 5.25 0 0 0 16 11.75Z""#
);
icon!(
    SIDEBAR,
    "rex-sidebar",
    r#"rect x="3" y="4" width="14" height="12" rx="3""#,
    r#"path d="M8 4v12""#
);
// The pane header's actions: a pane cut in two side by side, cut in two one over the other,
// grown to its tab's corners, and brought back from them.
icon!(
    SPLIT_RIGHT,
    "rex-split-right",
    r#"rect x="3" y="4.5" width="14" height="11" rx="2""#,
    r#"path d="M10 4.5v11""#
);
icon!(
    SPLIT_DOWN,
    "rex-split-down",
    r#"rect x="3" y="4.5" width="14" height="11" rx="2""#,
    r#"path d="M3 10h14""#
);
icon!(
    ZOOM,
    "rex-zoom",
    r#"path d="M11.5 4.5h4v4""#,
    r#"path d="m15.5 4.5-4.25 4.25""#,
    r#"path d="M8.5 15.5h-4v-4""#,
    r#"path d="m4.5 15.5 4.25-4.25""#
);
icon!(
    RESTORE,
    "rex-restore",
    r#"path d="M11.5 4.5v4h4""#,
    r#"path d="m11.5 8.5 4-4""#,
    r#"path d="M8.5 15.5v-4h-4""#,
    r#"path d="m8.5 11.5-4 4""#
);
icon!(MINUS, "rex-minus", r#"path d="M6 10h8""#);
// A popup button's badge: the value opens a list.
icon!(
    UPDOWN,
    "rex-updown",
    r#"path d="m7 8 3-3 3 3""#,
    r#"path d="m7 12 3 3 3-3""#
);
// Pane density: cards with gaps, or panes that share their borders.
icon!(
    PANES_APART,
    "rex-panes-apart",
    r#"rect x="3.5" y="4.5" width="5.5" height="11" rx="1.5""#,
    r#"rect x="11" y="4.5" width="5.5" height="11" rx="1.5""#
);
icon!(
    PANES_JOINED,
    "rex-panes-joined",
    r#"rect x="3.5" y="4.5" width="13" height="11" rx="1.5""#,
    r#"path d="M10 4.5v11""#
);
// Tab position: a strip along the top (the sidebar position uses `SIDEBAR`).
icon!(
    TABS_TOP,
    "rex-tabs-top",
    r#"rect x="3" y="4" width="14" height="12" rx="3""#,
    r#"path d="M3 8h14""#
);
// A muted tab: a bell with a stroke through it.
icon!(
    BELL_OFF,
    "rex-bell-off",
    r#"path d="M7.6 5.2A4 4 0 0 1 14 8.5c0 1.6.3 2.8.8 3.7""#,
    r#"path d="M6.2 7.2A4 4 0 0 0 6 8.5c0 2.6-.9 3.9-1.75 4.75h9.25""#,
    r#"path d="M8.75 16h2.5""#,
    r#"path d="m4 4 12 12""#
);

// Programs a tab can show instead of the terminal glyph (`activity::program_icon`).
icon!(
    CLAUDE,
    "rex-claude",
    r#"path d="M10 3.5v13""#,
    r#"path d="M3.5 10h13""#,
    r#"path d="m5.4 5.4 9.2 9.2""#,
    r#"path d="m14.6 5.4-9.2 9.2""#
);
icon!(
    CODEX,
    "rex-codex",
    r#"path d="M10 2.75 16.25 6.4v7.2L10 17.25 3.75 13.6V6.4Z""#,
    r#"path d="m8 8 2 2-2 2""#,
    r#"path d="M11 12.5h1.5""#
);
icon!(
    VIM,
    "rex-vim",
    r#"path d="m5 5 5 10.5L15 5""#,
    r#"path d="M3.5 5h4""#,
    r#"path d="M12.5 5h4""#
);
icon!(
    GIT,
    "rex-git",
    r#"circle cx="6.5" cy="5" r="1.75""#,
    r#"circle cx="6.5" cy="15" r="1.75""#,
    r#"circle cx="13.5" cy="7" r="1.75""#,
    r#"path d="M6.5 6.75v6.5""#,
    r#"path d="M13.5 8.75c0 3-7 2.25-7 4.5""#
);
icon!(
    CRATE,
    "rex-crate",
    r#"path d="M10 3 16.25 6.5v7L10 17l-6.25-3.5v-7Z""#,
    r#"path d="M3.75 6.5 10 10l6.25-3.5""#,
    r#"path d="M10 10v7""#
);
icon!(
    KEY,
    "rex-key",
    r#"circle cx="7" cy="13" r="3""#,
    r#"path d="m9.2 10.8 6.3-6.3""#,
    r#"path d="m13.5 6.5 2 2""#
);

/// Solid marks drawn in `currentColor`, for logos that outlines make unrecognisable.
macro_rules! mark {
    ($name:ident, $id:literal, $body:literal) => {
        pub static $name: Icon = Icon {
            name: $id,
            view: 20.0,
            body: $body,
        };
    };
}
// Pi: a rounded tile with a block P and dot cut out of it.
mark!(
    PI,
    "rex-pi",
    r#"<path fill="currentColor" fill-rule="evenodd" d="M5.5 2.5h9a3 3 0 0 1 3 3v9a3 3 0 0 1-3 3h-9a3 3 0 0 1-3-3v-9a3 3 0 0 1 3-3ZM6 5.5h7v5.5H9v3.5H6ZM9 7.5h1.75v1.5H9ZM13 12h2v2.5h-2Z"/>"#
);
// OpenCode: a heavy block O whose lower counter is shaded.
mark!(
    OPENCODE,
    "rex-opencode",
    r#"<path fill="currentColor" fill-rule="evenodd" d="M4 2.5h12v15H4ZM7.5 6v8h5V6Z"/><path fill="currentColor" fill-opacity=".45" d="M7.5 10h5v4h-5Z"/>"#
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midnight_keeps_its_token_values() {
        let p = &MIDNIGHT;
        assert_eq!(
            [
                p.ground,
                p.card,
                p.popover,
                p.ink,
                p.muted,
                p.accent,
                p.accent2,
                p.live,
                p.warning,
                p.danger,
                p.on_accent,
                p.terminal_selection,
            ],
            [
                0x001528, 0x001528, 0x0d1524, 0xe8eef6, 0x93a3b8, 0x7cc4ff, 0xa78bfa, 0x5eead4,
                0xf5d48a, 0xff8a8a, 0x061220, 0x2f4a6e,
            ]
        );
        assert_eq!(kit().c.page, rgb(0x001528));
        assert_eq!(kit().c.text_muted, rgb(0x93a3b8));
    }

    /// WCAG 2 contrast ratio of two sRGB colours.
    fn contrast(a: u32, b: u32) -> f64 {
        let luminance = |c: u32| {
            let channel = |shift: u32| {
                let v = ((c >> shift) & 0xff) as f64 / 255.;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
        };
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// Hue in degrees.
    fn hue(c: u32) -> f64 {
        let [r, g, b] = [16, 8, 0].map(|shift| ((c >> shift) & 0xff) as f64 / 255.);
        let (max, min) = (r.max(g).max(b), r.min(g).min(b));
        let d = max - min;
        let h = if d == 0. {
            0.
        } else if max == r {
            ((g - b) / d).rem_euclid(6.)
        } else if max == g {
            (b - r) / d + 2.
        } else {
            (r - g) / d + 4.
        };
        h * 60.
    }

    #[test]
    fn every_palette_is_legible_and_pairs_with_its_terminal_theme() {
        assert!((contrast(0xffffff, 0x000000) - 21.).abs() < 1e-9);
        for chrome in ChromeTheme::ALL {
            let p = palette_of(chrome);
            let name = chrome.name();
            assert!(contrast(p.muted, p.card) >= 4.5, "{name}: muted on card");
            assert!(contrast(p.ink, p.popover) >= 4.5, "{name}: ink on popover");
            // Keycaps: on a popover's rows, and on the sidebar's search chip (itself 8% white
            // over the card).
            let keys = mix(p.ink, p.muted, 0.5);
            for (surface, under, lift) in [("popover", p.popover, 0.), ("sidebar", p.card, 0.08)] {
                let chip = mix(0xffffff, under, KEYCAP_FILL + lift);
                assert!(
                    contrast(keys, chip) >= 4.5,
                    "{name}: keycaps on the {surface}"
                );
            }
            // A selected row keeps its label and its keycaps in ink on either tint, and its
            // outline stands clear of the popover around it.
            for (kind, tone) in [("selection", p.accent), ("destructive selection", p.danger)] {
                let fill = mix(tone, p.popover, SELECTION_FILL);
                assert!(contrast(p.ink, fill) >= 4.5, "{name}: ink on the {kind}");
                let chip = mix(0xffffff, fill, KEYCAP_LIT);
                assert!(
                    contrast(p.ink, chip) >= 4.5,
                    "{name}: keycaps on the {kind}"
                );
                assert!(contrast(tone, p.popover) >= 3., "{name}: {kind} outline");
            }
            // Trouble always carries a word; Gruvbox's brightest red is 4.3:1 on its ground.
            for (word, color) in [("warning", p.warning), ("danger", p.danger)] {
                for surface in [p.card, p.popover] {
                    assert!(contrast(color, surface) >= 3., "{name}: {word}");
                }
            }
            let gap = (hue(p.warning) - hue(p.danger)).abs();
            assert!(
                gap.min(360. - gap) >= 30.,
                "{name}: warning and danger differ in hue"
            );
            assert_eq!(p.card, p.ground, "{name}: one surface");
            let paired = rex_linux::terminal_theme::paired(chrome);
            assert_eq!(
                (paired.theme)().background,
                Some(p.ground),
                "{name}: the paired theme sits on the ground"
            );
            set_palette(p);
            assert_eq!(kit().c.page, rgb(p.ground), "{name}: its own kit");
            assert_eq!(kit().c.text_base, rgb(p.ink));
        }
        set_palette(&MIDNIGHT);
        assert_eq!(kit().c.page, rgb(0x001528), "Midnight's kit is unchanged");
    }
}
