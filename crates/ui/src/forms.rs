//! The text fields of the app screens and helpers to read and fill them.

use appforge_appstore::{AppSnapshot, ReviewDetail};
use gpui_kit::{
  component::input::{InputState, TextareaState},
  *,
};

pub struct Forms {
  pub name: Entity<InputState>,
  pub subtitle: Entity<InputState>,
  pub description: Entity<TextareaState>,
  pub keywords: Entity<InputState>,
  pub promotional: Entity<TextareaState>,
  pub support_url: Entity<InputState>,
  pub marketing_url: Entity<InputState>,
  pub whats_new: Entity<TextareaState>,
  pub privacy_choices: Entity<InputState>,
  pub privacy_url: Entity<InputState>,
  pub first_name: Entity<InputState>,
  pub last_name: Entity<InputState>,
  pub phone: Entity<InputState>,
  pub email: Entity<InputState>,
  pub demo_name: Entity<InputState>,
  pub demo_password: Entity<InputState>,
  pub notes: Entity<TextareaState>,
  pub data_types: Entity<InputState>,
  pub sdks: Entity<InputState>,
  /// The "Sign-in required" switch of App Review.
  pub demo_required: bool,
}

fn input<T: 'static>(window: &mut Window, cx: &mut Context<T>, placeholder: &'static str) -> Entity<InputState> {
  cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
}

fn area<T: 'static>(window: &mut Window, cx: &mut Context<T>, placeholder: &'static str, rows: usize) -> Entity<TextareaState> {
  cx.new(|cx| TextareaState::new(window, cx).rows(rows).placeholder(placeholder))
}

pub fn set_input(input: &Entity<InputState>, value: &str, window: &mut Window, cx: &mut App) {
  input.update(cx, |input, cx| input.set_value(value.to_owned(), window, cx));
}

pub fn set_area(area: &Entity<TextareaState>, value: &str, window: &mut Window, cx: &mut App) {
  area.update(cx, |area, cx| area.set_value(value.to_owned(), window, cx));
}

pub fn read_input(input: &Entity<InputState>, cx: &App) -> String {
  input.read(cx).value().trim().to_owned()
}

pub fn read_area(area: &Entity<TextareaState>, cx: &App) -> String {
  area.read(cx).value().trim().to_owned()
}

impl Forms {
  pub fn new<T: 'static>(window: &mut Window, cx: &mut Context<T>) -> Self {
    Self {
      name: input(window, cx, "App name"),
      subtitle: input(window, cx, "A short line under the name"),
      description: area(window, cx, "What the app does and who it is for", 16),
      keywords: input(window, cx, "planner,todo,calendar"),
      promotional: area(window, cx, "Shown above the description; can change without a new version", 5),
      support_url: input(window, cx, "https://example.com/support"),
      marketing_url: input(window, cx, "https://example.com"),
      whats_new: area(window, cx, "What changed in this version", 8),
      privacy_choices: input(window, cx, "https://example.com/privacy-choices"),
      privacy_url: input(window, cx, "https://example.com/privacy"),
      first_name: input(window, cx, "First name"),
      last_name: input(window, cx, "Last name"),
      phone: input(window, cx, "+1 555 0100"),
      email: input(window, cx, "name@example.com"),
      demo_name: input(window, cx, "Demo account user name"),
      demo_password: cx.new(|cx| InputState::new(window, cx).placeholder("Demo account password").masked(true)),
      notes: area(window, cx, "How to reach every feature, hardware needed, in-app purchases, regional behaviour", 12),
      data_types: input(window, cx, "email, purchases, crash logs"),
      sdks: input(window, cx, "Firebase, AdMob"),
      demo_required: false,
    }
  }

  /// Fills the App Information and App Privacy fields with `locale` of `snapshot`.
  pub fn fill_locale(&mut self, snapshot: &AppSnapshot, locale: &str, window: &mut Window, cx: &mut App) {
    let info = snapshot.info_localizations.iter().find(|item| item.locale == locale);
    set_input(&self.name, info.map_or("", |item| &item.name), window, cx);
    set_input(&self.subtitle, info.map_or("", |item| &item.subtitle), window, cx);
    set_input(&self.privacy_choices, info.map_or("", |item| &item.privacy_choices_url), window, cx);
    set_input(&self.privacy_url, info.map_or("", |item| &item.privacy_policy_url), window, cx);
    let listing = snapshot.version_localizations.iter().find(|item| item.locale == locale);
    set_area(&self.description, listing.map_or("", |item| &item.description), window, cx);
    set_input(&self.keywords, listing.map_or("", |item| &item.keywords), window, cx);
    set_area(&self.promotional, listing.map_or("", |item| &item.promotional_text), window, cx);
    set_input(&self.support_url, listing.map_or("", |item| &item.support_url), window, cx);
    set_input(&self.marketing_url, listing.map_or("", |item| &item.marketing_url), window, cx);
    set_area(&self.whats_new, listing.map_or("", |item| &item.whats_new), window, cx);
  }

  pub fn fill_review(&mut self, review: Option<&ReviewDetail>, window: &mut Window, cx: &mut App) {
    let review = review.cloned().unwrap_or_default();
    set_input(&self.first_name, &review.contact_first_name, window, cx);
    set_input(&self.last_name, &review.contact_last_name, window, cx);
    set_input(&self.phone, &review.contact_phone, window, cx);
    set_input(&self.email, &review.contact_email, window, cx);
    set_input(&self.demo_name, &review.demo_account_name, window, cx);
    set_input(&self.demo_password, &review.demo_account_password, window, cx);
    set_area(&self.notes, &review.notes, window, cx);
    self.demo_required = review.demo_account_required;
  }
}
