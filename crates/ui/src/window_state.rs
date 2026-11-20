//! Opening the window the way it was left: its size, its place and whether it was maximized or full screen.

use appforge_storage::{data_dir, Settings, WindowMode, WindowState};
use gpui_kit::*;

/// The size of a window that has no saved state.
const DEFAULT_WIDTH: f32 = 1100.;
const DEFAULT_HEIGHT: f32 = 760.;

/// The smallest size the window can be dragged to.
pub const MIN_WIDTH: f32 = 800.;
pub const MIN_HEIGHT: f32 = 550.;

/// Where and how big the window opens: as it was left last time. The size always comes back; the place too, unless it is
/// no longer on a screen (a display was unplugged), in which case the window opens in the middle of the main display.
pub fn initial_window_bounds(cx: &App) -> WindowBounds {
  let default = WindowBounds::Windowed(Bounds::centered(None, size(px(DEFAULT_WIDTH), px(DEFAULT_HEIGHT)), cx));
  let Some(saved) = Settings::load(&data_dir().join("settings.json")).ok().and_then(|settings| settings.window) else { return default };
  let mut bounds = Bounds::new(
    point(px(saved.x as f32), px(saved.y as f32)),
    size(px((saved.width as f32).max(MIN_WIDTH)), px((saved.height as f32).max(MIN_HEIGHT))),
  );
  if !cx.displays().iter().any(|display| display.bounds().contains(&bounds.center())) {
    bounds = Bounds::centered(None, bounds.size, cx);
  }
  match saved.mode {
    WindowMode::Windowed => WindowBounds::Windowed(bounds),
    WindowMode::Maximized => WindowBounds::Maximized(bounds),
    WindowMode::Fullscreen => WindowBounds::Fullscreen(bounds),
  }
}

/// The window as it is now.
pub(crate) fn state_of(window: &Window) -> WindowState {
  let (mode, bounds) = match window.window_bounds() {
    WindowBounds::Windowed(bounds) => (WindowMode::Windowed, bounds),
    WindowBounds::Maximized(bounds) => (WindowMode::Maximized, bounds),
    WindowBounds::Fullscreen(bounds) => (WindowMode::Fullscreen, bounds),
  };
  let points = |value: Pixels| f32::from(value).round() as i32;
  WindowState { mode, x: points(bounds.origin.x), y: points(bounds.origin.y), width: points(bounds.size.width), height: points(bounds.size.height) }
}
