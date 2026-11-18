//! The AppForge interface: sign in to App Store Connect, browse apps, edit App Information, App Review and
//! App Privacy, and see what could get an app rejected.

mod analysis;
mod apps_view;
mod detail;
mod forms;
mod guide_art;
pub mod i18n;
mod listing_ai;
mod login;
mod settings_view;
mod setup;
mod sidebar;
mod theme;
mod view;
mod welcome;
mod widgets;
mod window_state;
mod workspace;

use gpui_kit::{component::Theme, *};
use std::borrow::Cow;

pub use window_state::{initial_window_bounds, MIN_HEIGHT, MIN_WIDTH};
pub use workspace::Workspace;

actions!(appforge, [Quit]);

/// The Geist typeface (SIL Open Font License, see `assets/fonts/OFL.txt`).
const GEIST: [&[u8]; 4] = [
  include_bytes!("../assets/fonts/Geist-Regular.ttf"),
  include_bytes!("../assets/fonts/Geist-Medium.ttf"),
  include_bytes!("../assets/fonts/Geist-SemiBold.ttf"),
  include_bytes!("../assets/fonts/Geist-Bold.ttf"),
];

/// Makes Geist the interface font. Without it the system font stays, so a failure is only logged.
pub fn install_fonts(cx: &mut App) {
  match cx.text_system().add_fonts(GEIST.iter().map(|font| Cow::Borrowed(*font)).collect()) {
    Ok(()) => Theme::update(cx, |theme| theme.font_family = "Geist".into()),
    Err(error) => eprintln!("Could not load the Geist font: {error}"),
  }
}

pub struct Assets;

impl AssetSource for Assets {
  fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
    gpui_kit_assets::AllAssets.load(path)
  }

  fn list(&self, path: &str) -> Result<Vec<SharedString>> {
    gpui_kit_assets::AllAssets.list(path)
  }
}
