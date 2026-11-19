//! Startup: initializes the Kit, opens the window, binds keys and the menu.

use appforge_ui::{install_fonts, Assets, Quit, Workspace};
use gpui_kit::{component::TitleBar, *};

pub fn run() {
  let app = gpui_kit::application().with_assets(Assets);
  app.on_reopen(|cx| {
    if cx.windows().is_empty() {
      open_window(cx);
    }
    cx.activate(true);
  });
  app.run(|cx| {
    gpui_kit::init(cx);
    install_fonts(cx);
    cx.activate(true);
    cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
    cx.set_menus(vec![Menu::new("AppForge").items([MenuItem::action("Quit AppForge", Quit)])]);
    cx.on_action(|_: &Quit, cx| cx.quit());
    open_window(cx);
  });
}

fn open_window(cx: &mut App) {
  let options = WindowOptions {
    window_bounds: Some(appforge_ui::initial_window_bounds(cx)),
    window_min_size: Some(size(px(appforge_ui::MIN_WIDTH), px(appforge_ui::MIN_HEIGHT))),
    titlebar: Some(TitleBar::title_bar_options()),
    ..TitleBar::window_options()
  };
  gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| Workspace::new(window, cx))).expect("Failed to open the AppForge window");
}
