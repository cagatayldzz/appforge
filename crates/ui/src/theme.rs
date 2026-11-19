//! AppForge's light and dark colors and fonts, switched with the system appearance.
//! The palettes use semantic colors from GitHub Primer (https://github.com/primer/primitives).

use gpui_kit::{
  component::{ActiveTheme as _, Theme},
  *,
};

/// The size of the text the interface draws itself. The kit's own parts (icons) are sized in `rem`,
/// which stays at the theme's 16 px.
pub(crate) const FONT_SIZE: Pixels = px(14.);

/// Secondary text, such as a bundle id under an app's name.
pub(crate) const FONT_SIZE_SM: Pixels = px(12.);

/// The interface font (bundled, see `install_fonts`).
const SANS_FONT: &str = "Geist";

/// The colors of one appearance.
#[derive(Clone, Copy)]
pub(crate) struct Colors {
  /// The main area and the window.
  pub background: Hsla,
  pub text: Hsla,
  pub text_secondary: Hsla,
  /// The sidebar.
  pub surface: Hsla,
  /// A slightly raised surface: the status bar, the tab strip.
  pub surface_alt: Hsla,
  pub border: Hsla,
  /// The outline of a field or a checkbox: stronger than `border`, which only separates areas.
  pub control_border: Hsla,
  /// The row that is chosen.
  pub selected: Hsla,
  pub hover: Hsla,
  /// The title band and the activity bar.
  pub rail: Hsla,
  pub rail_hover: Hsla,
  pub rail_item_hover: Hsla,
  pub rail_text: Hsla,
  /// The accent for selection controls and the line over the shown tab.
  pub accent: Hsla,
  pub accent_hover: Hsla,
  /// Text and icons drawn on the accent color.
  pub on_accent: Hsla,
  pub primary_button: Hsla,
  pub primary_button_hover: Hsla,
  pub primary_button_active: Hsla,
  pub link: Hsla,
  pub link_hover: Hsla,
  /// Marks for done, failed, pending and informational states.
  pub success: Hsla,
  pub danger: Hsla,
  pub warning: Hsla,
  pub on_warning: Hsla,
  pub info: Hsla,
}

fn hex(value: u32) -> Hsla {
  rgb(value).into()
}

/// The colors an app's avatar can take.
const AVATAR_COLORS: [u32; 8] = [0x4d8dff, 0x8b7bff, 0xf066b0, 0xee5a5a, 0xf0883e, 0xd9a62e, 0x3fb68b, 0x2fb5c9];

/// A color for an app's avatar, always the same for the same id.
pub(crate) fn avatar_color(id: &str) -> Hsla {
  let sum = id.bytes().fold(0usize, |sum, byte| sum.wrapping_mul(31).wrapping_add(byte as usize));
  hex(AVATAR_COLORS[sum % AVATAR_COLORS.len()])
}

impl Colors {
  fn dark() -> Self {
    Self {
      background: hex(0x0d1117),
      text: hex(0xf0f6fc),
      text_secondary: hex(0x9198a1),
      surface: hex(0x0d1117),
      surface_alt: hex(0x151b23),
      border: hex(0x252c35),
      control_border: hex(0x3d444d),
      selected: hex(0x172b45),
      hover: hex(0x151b23),
      rail: hex(0x010409),
      rail_hover: hex(0x151b23),
      rail_item_hover: hex(0x212830),
      rail_text: hex(0x9198a1),
      accent: hex(0x1f6feb),
      accent_hover: hex(0x1c62d2),
      on_accent: hex(0xffffff),
      primary_button: hex(0x238636),
      primary_button_hover: hex(0x29903b),
      primary_button_active: hex(0x216e39),
      link: hex(0x4493f8),
      link_hover: hex(0x79b8ff),
      success: hex(0x3fb950),
      danger: hex(0xf85149),
      warning: hex(0xd29922),
      on_warning: hex(0x0d1117),
      info: hex(0x4493f8),
    }
  }

  fn light() -> Self {
    Self {
      background: hex(0xffffff),
      text: hex(0x1f2328),
      text_secondary: hex(0x59636e),
      surface: hex(0xffffff),
      surface_alt: hex(0xf6f8fa),
      border: hex(0xe2e7ec),
      control_border: hex(0x818b98),
      selected: hex(0xddf4ff),
      hover: hex(0xf6f8fa),
      rail: hex(0xf6f8fa),
      rail_hover: hex(0xeaeef2),
      rail_item_hover: hex(0xd1d9e0),
      rail_text: hex(0x59636e),
      accent: hex(0x0969da),
      accent_hover: hex(0x0759bd),
      on_accent: hex(0xffffff),
      primary_button: hex(0x1f883d),
      primary_button_hover: hex(0x1a7f37),
      primary_button_active: hex(0x116329),
      link: hex(0x0969da),
      link_hover: hex(0x0550ae),
      success: hex(0x1a7f37),
      danger: hex(0xd1242f),
      warning: hex(0x9a6700),
      on_warning: hex(0xffffff),
      info: hex(0x0969da),
    }
  }
}

/// The colors for the appearance that is showing.
pub(crate) fn colors(cx: &App) -> Colors {
  if cx.theme().is_dark() {
    Colors::dark()
  } else {
    Colors::light()
  }
}

/// Follows the system's appearance and applies AppForge's colors and font to the Kit theme.
pub(crate) fn apply_theme(window: &mut Window, cx: &mut App) {
  Theme::sync_system_appearance(Some(window), cx);
  let c = colors(cx);
  Theme::update(cx, |theme| {
    theme.font_family = SANS_FONT.into();
    // Dialogs and notifications are rounded with `radius_lg`; fields, buttons and menu items with `radius`.
    theme.radius = px(6.);
    theme.radius_lg = px(10.);
    let colors = &mut theme.colors;
    colors.background = c.background;
    colors.foreground = c.text;
    colors.sidebar = c.surface;
    colors.sidebar_foreground = c.text;
    colors.sidebar_border = c.border;
    colors.sidebar_accent = c.selected;
    colors.sidebar_accent_foreground = c.text;
    colors.sidebar_primary = c.accent;
    colors.sidebar_primary_foreground = c.on_accent;
    colors.border = c.border;
    colors.input = c.control_border;
    colors.title_bar = c.rail;
    colors.title_bar_border = c.border;
    colors.status_bar = c.surface_alt;
    colors.status_bar_border = c.border;
    colors.group_box = c.surface;
    colors.group_box_foreground = c.text;
    colors.tab = c.surface;
    colors.tab_bar = c.surface;
    colors.tab_foreground = c.text_secondary;
    colors.tab_active_foreground = c.text;
    colors.switch = c.rail_item_hover;
    colors.tab_active = c.selected;
    colors.popover = c.surface;
    colors.popover_foreground = c.text;
    colors.muted = c.surface_alt;
    colors.muted_foreground = c.text_secondary;
    colors.secondary = c.surface_alt;
    colors.secondary_foreground = c.text;
    colors.accent = c.rail_hover;
    colors.accent_foreground = c.text;
    colors.primary = c.accent;
    colors.primary_hover = c.accent_hover;
    colors.primary_active = c.accent;
    colors.primary_foreground = c.on_accent;
    colors.button_primary = c.primary_button;
    colors.button_primary_hover = c.primary_button_hover;
    colors.button_primary_active = c.primary_button_active;
    colors.button_primary_foreground = c.on_accent;
    colors.button_secondary = c.surface_alt;
    colors.button_secondary_hover = c.rail_item_hover;
    colors.button_secondary_active = c.selected;
    colors.button_secondary_foreground = c.text;
    colors.ring = c.link;
    colors.caret = c.text;
    colors.link = c.link;
    colors.link_hover = c.link_hover;
    colors.link_active = c.link_hover;
    colors.list = c.surface;
    colors.list_even = c.surface;
    colors.list_hover = c.hover;
    colors.list_active = c.selected;
    colors.list_head = c.surface_alt;
    colors.list_active_border = c.link;
    colors.scrollbar_thumb = c.text_secondary.opacity(0.6);
    colors.scrollbar_thumb_hover = c.text_secondary;
    colors.success = c.success;
    colors.danger = c.danger;
    colors.warning = c.warning;
    colors.warning_foreground = c.on_warning;
    colors.info = c.info;
    colors.selection = c.accent.opacity(0.35);
  });
  window.refresh();
}
