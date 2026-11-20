//! The age rating editor: the content questions Apple turns into the age ratings, saved to App Store Connect.

use crate::{
  i18n::tr,
  theme::{colors, FONT_SIZE_SM},
  workspace::Workspace,
};
use appforge_appstore::AgeRatingDeclaration;
use appforge_storage::Language;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    scroll::ScrollableElement as _,
    switch::Switch,
    v_flex, Disableable as _, Selectable as _, Sizable as _, WindowExt as _,
  },
  *,
};

const EDITOR_WIDTH: Pixels = px(640.0);

/// Texts of `AGE_LEVEL_KEYS`, in the same order.
const LEVEL_LABELS: [&str; 12] = [
  "ageQAlcohol",
  "ageQContests",
  "ageQSimulatedGambling",
  "ageQHorror",
  "ageQMature",
  "ageQMedical",
  "ageQProfanity",
  "ageQGraphicSexual",
  "ageQSexual",
  "ageQCartoonViolence",
  "ageQRealisticViolence",
  "ageQSadisticViolence",
];

/// Texts of `AGE_FLAG_KEYS`, in the same order.
const FLAG_LABELS: [&str; 9] = [
  "ageQAdvertising",
  "ageQGambling",
  "ageQHealth",
  "ageQLootBox",
  "ageQMessaging",
  "ageQParental",
  "ageQAgeAssurance",
  "ageQWebAccess",
  "ageQUserContent",
];

const LEVELS: [(&str, &str); 3] = [("NONE", "ageLevelNone"), ("INFREQUENT_OR_MILD", "ageLevelMild"), ("FREQUENT_OR_INTENSE", "ageLevelIntense")];

const AGE_OVERRIDES: [(&str, &str); 6] = [
  ("NONE", "ageAutomatic"),
  ("NINE_PLUS", "age9Plus"),
  ("THIRTEEN_PLUS", "age13Plus"),
  ("SIXTEEN_PLUS", "age16Plus"),
  ("EIGHTEEN_PLUS", "age18Plus"),
  ("UNRATED", "ageUnrated"),
];

const KOREA_OVERRIDES: [(&str, &str); 3] = [("NONE", "ageAutomatic"), ("FIFTEEN_PLUS", "age15Plus"), ("NINETEEN_PLUS", "age19Plus")];

/// The dialog content: a draft of the declaration that is sent only when the user saves.
struct AgeRatingEditor {
  workspace: WeakEntity<Workspace>,
  original: AgeRatingDeclaration,
  draft: AgeRatingDeclaration,
  language: Language,
}

impl AgeRatingEditor {
  fn choices(
    &self,
    id: &'static str,
    options: &'static [(&'static str, &'static str)],
    current: &str,
    set: fn(&mut AgeRatingDeclaration, usize, &str),
    row: usize,
    cx: &mut Context<Self>,
  ) -> impl IntoElement {
    h_flex().gap_1().flex_wrap().children(options.iter().map(|(value, label)| {
      let value = *value;
      Button::new(SharedString::from(format!("{id}-{value}"))).label(tr(self.language, label)).small().outline().selected(current == value).on_click(
        cx.listener(move |this, _, _, cx| {
          set(&mut this.draft, row, value);
          cx.notify();
        }),
      )
    }))
  }

  fn labelled(&self, label: SharedString, body: impl IntoElement, cx: &App) -> Div {
    let c = colors(cx);
    v_flex().gap_1().child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(label)).child(body)
  }
}

impl Render for AgeRatingEditor {
  fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.language;
    let c = colors(cx);
    let changed = self.draft != self.original;
    let levels = (0..LEVEL_LABELS.len())
      .map(|row| {
        let current = self.draft.levels[row].clone();
        let choices = self.choices(LEVEL_LABELS[row], &LEVELS, &current, |draft, row, value| draft.levels[row] = value.to_owned(), row, cx);
        self.labelled(tr(l, LEVEL_LABELS[row]), choices, cx)
      })
      .collect::<Vec<_>>();
    let flags = (0..FLAG_LABELS.len())
      .map(|row| {
        Switch::new(FLAG_LABELS[row]).checked(self.draft.flags[row]).label(tr(l, FLAG_LABELS[row])).on_click(cx.listener(
          move |this, value: &bool, _, cx| {
            this.draft.flags[row] = *value;
            cx.notify();
          },
        ))
      })
      .collect::<Vec<_>>();
    let age_override = self.draft.age_override.clone();
    let korea_override = self.draft.korea_override.clone();
    let age_choices = self.choices("age-override", &AGE_OVERRIDES, &age_override, |draft, _, value| draft.age_override = value.to_owned(), 0, cx);
    let korea_choices =
      self.choices("korea-override", &KOREA_OVERRIDES, &korea_override, |draft, _, value| draft.korea_override = value.to_owned(), 0, cx);
    let workspace = self.workspace.clone();
    let (original, draft) = (self.original.clone(), self.draft.clone());
    v_flex()
      .gap_3()
      .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, "ageRatingEditorHint")))
      .child(
        v_flex()
          .id("age-rating-fields")
          .max_h_96()
          .gap_3()
          .pr_2()
          .overflow_y_scrollbar()
          .children(levels)
          .child(v_flex().gap_2().children(flags))
          .child(self.labelled(tr(l, "ageRaiseRating"), age_choices, cx))
          .child(self.labelled(tr(l, "ageKoreaRating"), korea_choices, cx)),
      )
      .child(
        h_flex()
          .gap_2()
          .justify_end()
          .child(Button::new("cancel-age-rating").outline().label(tr(l, "cancel")).on_click(|_, window, cx| window.close_dialog(cx)))
          .child(Button::new("save-age-rating").primary().label(tr(l, "save")).disabled(!changed).on_click(move |_, window, cx| {
            window.close_dialog(cx);
            let (original, draft) = (original.clone(), draft.clone());
            workspace.update(cx, |this, cx| this.run_save(window, cx, move |client| client.update_age_rating(&draft, &original))).ok();
          })),
      )
  }
}

impl Workspace {
  /// Opens the editor for the declaration of the open app.
  pub(crate) fn open_age_rating_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(original) = self.ready_snapshot().and_then(|snapshot| snapshot.general.age_declaration.clone()) else { return };
    let language = self.settings.language;
    let workspace = cx.weak_entity();
    let editor = cx.new(|_| AgeRatingEditor { workspace, draft: original.clone(), original, language });
    window.open_dialog(cx, move |dialog, _, _| dialog.width(EDITOR_WIDTH).title(tr(language, "editAgeRatings")).child(editor.clone()));
  }
}
