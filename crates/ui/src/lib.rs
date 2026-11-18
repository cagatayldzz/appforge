//! The AppForge interface: sign in to App Store Connect, browse apps, edit App Information, App Review and
//! App Privacy, and see what could get an app rejected.

mod analysis;
mod detail;
mod forms;
mod guide_art;
pub mod i18n;
mod login;
mod settings_view;
mod setup;
mod sidebar;
mod view;
mod welcome;
mod widgets;
mod workspace;

use gpui_kit::*;
use std::borrow::Cow;

pub use workspace::Workspace;

actions!(appforge, [Quit]);

pub struct Assets;

impl AssetSource for Assets {
  fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
    gpui_kit_assets::AllAssets.load(path)
  }

  fn list(&self, path: &str) -> Result<Vec<SharedString>> {
    gpui_kit_assets::AllAssets.list(path)
  }
}
