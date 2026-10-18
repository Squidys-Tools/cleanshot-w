//! The design tokens from `mockups/snipsy-ui-redesign/design-spec.md` §2,
//! expressed for gpui. Every surface renders through a `Theme` so dark and
//! light schemes stay in lockstep; the editor canvas backdrop is always
//! dark-neutral in both schemes (the screenshot must pop).

use gpui::{App, BoxShadow, Global, Hsla, px, rgb, rgba};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scheme {
    #[default]
    Dark,
    Light,
}

impl Scheme {
    pub fn toggled(self) -> Self {
        match self {
            Scheme::Dark => Scheme::Light,
            Scheme::Light => Scheme::Dark,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Scheme::Dark => "Dark",
            Scheme::Light => "Light",
        }
    }
}

use crate::settings;

impl Scheme {}

/// Corner radii (spec §2.3), in px.
pub const R_PANEL: f32 = 10.0;
pub const R_PILL: f32 = 16.0;
pub const R_BTN: f32 = 8.0;
pub const R_INPUT: f32 = 6.0;
pub const R_THUMB: f32 = 6.0;

#[derive(Debug, Clone)]
pub struct Theme {
    /// Window shell background.
    pub bg_window: Hsla,
    /// Panels, rail, cards.
    pub bg_surface: Hsla,
    /// Inputs and secondary buttons.
    pub bg_elevated: Hsla,
    /// Editor canvas backdrop; dark-neutral in both schemes.
    pub bg_canvas: Hsla,
    /// Floating pills (toolbar, selection strip, quick-access card).
    pub bg_toolbar: Hsla,
    pub text_1: Hsla,
    pub text_2: Hsla,
    pub text_3: Hsla,
    pub accent: Hsla,
    /// Active-tool background.
    pub accent_soft: Hsla,
    pub hairline: Hsla,
    pub hairline_strong: Hsla,
    pub danger: Hsla,
    pub shadow_floating: Vec<BoxShadow>,
    pub shadow_panel: Vec<BoxShadow>,
}

impl Theme {
    pub fn for_scheme(scheme: Scheme) -> Self {
        match scheme {
            Scheme::Dark => Self::dark(),
            Scheme::Light => Self::light(),
        }
    }

    pub fn dark() -> Self {
        Self {
            bg_window: rgb(0x1f1f23).into(),
            bg_surface: rgb(0x26262b).into(),
            bg_elevated: rgb(0x2e2e34).into(),
            bg_canvas: rgb(0x17171a).into(),
            bg_toolbar: rgba(0x222227fa).into(),
            text_1: rgb(0xf2f2f4).into(),
            text_2: rgb(0xa0a0aa).into(),
            text_3: rgb(0x6e6e78).into(),
            accent: rgb(0x4c8dff).into(),
            accent_soft: rgba(0x4c8dff29).into(),
            hairline: gpui::white().alpha(0.08),
            hairline_strong: gpui::white().alpha(0.14),
            danger: rgb(0xe5484d).into(),
            shadow_floating: floating_shadows(0.45, 0.35),
            shadow_panel: panel_shadows(0.40, 0.30),
        }
    }

    pub fn light() -> Self {
        Self {
            bg_window: rgb(0xf6f6f8).into(),
            bg_surface: gpui::white(),
            bg_elevated: rgb(0xf0f0f3).into(),
            bg_canvas: rgb(0xe8e8ec).into(),
            bg_toolbar: rgba(0xfffffffa).into(),
            text_1: rgb(0x1d1d21).into(),
            text_2: rgb(0x6e6e78).into(),
            text_3: rgb(0xa0a0aa).into(),
            accent: rgb(0x2f6fed).into(),
            accent_soft: rgba(0x2f6fed1f).into(),
            hairline: gpui::black().alpha(0.08),
            hairline_strong: gpui::black().alpha(0.14),
            danger: rgb(0xc94a3c).into(),
            shadow_floating: floating_shadows(0.16, 0.08),
            shadow_panel: panel_shadows(0.12, 0.08),
        }
    }

    /// The theme for the app's current scheme.
    pub fn current(cx: &App) -> Self {
        Self::for_scheme(
            cx.try_global::<ThemeSetting>()
                .map_or(Scheme::default(), |setting| setting.scheme),
        )
    }
}

fn panel_shadows(large: f32, tight: f32) -> Vec<BoxShadow> {
    vec![
        BoxShadow::new(px(0.), px(8.), gpui::black().alpha(large)).blur_radius(px(28.)),
        BoxShadow::new(px(0.), px(1.), gpui::black().alpha(tight)).blur_radius(px(4.)),
    ]
}

fn floating_shadows(large: f32, tight: f32) -> Vec<BoxShadow> {
    vec![
        BoxShadow::new(px(0.), px(12.), gpui::black().alpha(large)).blur_radius(px(40.)),
        BoxShadow::new(px(0.), px(2.), gpui::black().alpha(tight)).blur_radius(px(8.)),
    ]
}
/// The app's scheme, shared by every window. Defaults to dark; the
/// preferences panel toggles and persists it.
#[derive(Default)]
pub struct ThemeSetting {
    pub scheme: Scheme,
}

impl Global for ThemeSetting {}

/// Apply the scheme the user has persisted in settings.
pub fn init_from_settings(cx: &mut App) {
    let theme = settings::current_settings().theme;
    cx.set_global(ThemeSetting { scheme: theme });
}

/// Switch the scheme for every surface and persist the choice.
pub fn set_scheme(cx: &mut App, scheme: Scheme) {
    cx.set_global(ThemeSetting { scheme });
}
