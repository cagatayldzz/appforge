//! Localized product page media, App Information summaries and App Store Connect links.

use crate::{
  i18n::{tr, trf},
  localization::locale_name,
  theme::{colors, FONT_SIZE_SM},
  widgets::{card, page_section, pair, section},
  workspace::{Load, Workspace},
};
use appforge_appstore::{App as StoreApp, ProductAsset};
use gpui_kit::{
  component::{
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    spinner::Spinner,
    v_flex, Disableable as _, Icon, IconName, Sizable as _, WindowExt as _,
  },
  *,
};
use std::sync::Arc;

const MEDIA_PREVIEW_WIDTH: Pixels = px(560.0);

struct Media {
  asset: ProductAsset,
  image: Option<Arc<Image>>,
}

#[derive(Default)]
pub(crate) struct MediaLoad {
  assets: Load<Vec<Media>>,
  pub(crate) task: Option<Task<()>>,
}

pub(crate) fn store_link(id: &'static str, label: SharedString, url: String) -> Button {
  Button::new(id).label(label).icon(Icon::new(IconName::ExternalLink)).small().outline().on_click(move |_, _, cx| cx.open_url(&url))
}

pub(crate) fn store_info(app: &StoreApp) -> String {
  format!("https://appstoreconnect.apple.com/apps/{}/distribution/info", app.id)
}

pub(crate) fn store_version(app: &StoreApp) -> String {
  format!("https://appstoreconnect.apple.com/apps/{}/distribution/ios/version/inflight", app.id)
}

fn display_name(value: &str) -> String {
  value
    .trim_start_matches("GAMES_")
    .split('_')
    .map(|part| {
      let part = match part {
        "IPAD" => return "iPad".to_owned(),
        "IPHONE" => return "iPhone".to_owned(),
        _ => part,
      };
      let mut letters = part.chars();
      letters.next().map_or_else(String::new, |first| format!("{}{}", first.to_uppercase(), letters.as_str().to_lowercase()))
    })
    .collect::<Vec<_>>()
    .join(" ")
}

/// A label over its value (or a dash), the look of every display-only detail.
pub(crate) fn info_field(l: appforge_storage::Language, c: &crate::theme::Colors, key: &'static str, value: &str) -> Div {
  v_flex().gap_0p5().child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, key))).child(div().child(if value.is_empty() {
    "—".into()
  } else {
    SharedString::from(value.to_owned())
  }))
}

impl Workspace {
  pub(crate) fn product_localization(&self) -> Option<&str> {
    self.ready_snapshot()?.version_localizations.iter().find(|item| item.locale == self.locale).map(|item| item.id.as_str())
  }

  pub(crate) fn load_product_assets(&mut self, refresh: bool, window: &mut Window, cx: &mut Context<Self>) {
    let (Some(client), Some(app), Some(locale)) = (self.client.clone(), self.selected.clone(), self.product_localization().map(str::to_owned)) else {
      return;
    };
    let Some(data) = self.app_data.get_mut(&app) else { return };
    let entry = data.product_assets.entry(locale.clone()).or_default();
    if entry.task.is_some() || (!refresh && !matches!(entry.assets, Load::Idle)) {
      return;
    }
    entry.assets = Load::Loading;
    let notice = crate::workspace::refresh::Refresh::ProductMedia;
    let notice_id = format!("{app}-{locale}");
    if refresh {
      notice.start(&notice_id, self.settings.language, window, cx);
    }
    let target = locale.clone();
    entry.task = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx
        .background_spawn(async move {
          client.product_assets(&target).map(|assets| {
            assets
              .chunks(6)
              .flat_map(|chunk| {
                std::thread::scope(|scope| {
                  let tasks: Vec<_> = chunk
                    .iter()
                    .map(|asset| {
                      scope.spawn(|| {
                        let image = asset
                          .thumbnail
                          .as_deref()
                          .and_then(|url| client.download_product_thumbnail(url).ok())
                          .map(|bytes| Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes)));
                        Media { asset: asset.clone(), image }
                      })
                    })
                    .collect();
                  tasks.into_iter().filter_map(|task| task.join().ok()).collect::<Vec<_>>()
                })
              })
              .collect::<Vec<_>>()
          })
        })
        .await;
      this
        .update_in(cx, |this, window, cx| {
          if refresh {
            notice.finish(&notice_id, this.settings.language, &result, window, cx);
          }
          if this.require_apple_sign_in(&result, window, cx) {
            return;
          }
          let Some(entry) = this.app_data.get_mut(&app).and_then(|data| data.product_assets.get_mut(&locale)) else { return };
          entry.task = None;
          entry.assets = match result {
            Ok(media) => Load::Ready(media),
            Err(error) => Load::Failed(error.to_string().into()),
          };
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  pub(crate) fn render_product_page(&self, app: &StoreApp, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let locale = self.product_localization();
    let entry = locale.and_then(|locale| self.active_data()?.product_assets.get(locale));
    let loading = entry.is_some_and(|entry| entry.task.is_some());
    let mut page = v_flex().gap_4().child(
      h_flex().gap_3().flex_wrap().items_center().justify_end().child(
        h_flex().gap_2().flex_wrap().child(store_link("manage-product-media", tr(l, "manageProductPage"), store_version(app))).child(
          Button::new("reload-product-media")
            .label(tr(l, "reload"))
            .icon(Icon::new(IconName::RefreshCw))
            .small()
            .outline()
            .loading(loading)
            .disabled(locale.is_none() || loading)
            .on_click(cx.listener(|this, _, window, cx| this.load_product_assets(true, window, cx))),
        ),
      ),
    );
    if locale.is_none() {
      return page.child(Alert::info("no-product-locale", tr(l, "noProductLocalization")));
    }
    match entry.map(|entry| &entry.assets) {
      Some(Load::Ready(media)) => {
        let header = media.iter().filter(|item| item.asset.kind == "PRODUCT_PAGE_HEADER_ASSET");
        page = page.child(section(tr(l, "headerAndSearchResults"), self.media_strip(header, "header", true, cx)));
        let mut groups = std::collections::BTreeMap::<&str, Vec<_>>::new();
        for item in media.iter().filter(|item| matches!(item.asset.kind.as_str(), "APP_SCREENSHOT" | "APP_PREVIEW")) {
          groups.entry(&item.asset.device).or_default().push(item);
        }
        let mut previews = v_flex().gap_3().child(card(
          "iPhone Duo",
          Some(tr(l, "duoScreenshotSizes")),
          Some(store_link("add-duo-media", tr(l, "addDuoScreenshots"), store_version(app)).into_any_element()),
          div().text_sm().child(tr(l, "duoMediaHelp")),
          &colors(cx),
        ));
        if groups.is_empty() {
          previews = previews.child(Alert::info("empty-previews", tr(l, "noProductAssets")));
        }
        for (group, items) in groups {
          let shots = items.iter().filter(|item| item.asset.kind == "APP_SCREENSHOT").count().to_string();
          let videos = items.iter().filter(|item| item.asset.kind == "APP_PREVIEW").count().to_string();
          previews = previews.child(card(
            display_name(group.trim_end_matches("_PROFILE")),
            Some(trf(l, "productMediaCounts", &[&shots, &videos])),
            None,
            self.media_strip(items.into_iter(), group, false, cx),
            &colors(cx),
          ));
        }
        page.child(section(tr(l, "appPreviewsAndScreenshots"), previews))
      }
      Some(Load::Failed(error)) => page.child(Alert::error("product-media-failed", error.clone())).child(
        Button::new("retry-product-media")
          .label(tr(l, "tryAgain"))
          .outline()
          .on_click(cx.listener(|this, _, window, cx| this.load_product_assets(true, window, cx))),
      ),
      _ => page.child(h_flex().gap_2().child(Spinner::new()).child(tr(l, "loadingFromAppStore"))),
    }
  }

  fn media_strip<'a>(&self, items: impl Iterator<Item = &'a Media>, group: &str, wide: bool, cx: &mut Context<Self>) -> impl IntoElement {
    let c = colors(cx);
    let l = self.settings.language;
    let editable = self.ready_snapshot().and_then(|snapshot| snapshot.version.as_ref()).is_some_and(|version| version.is_editable());
    let busy = self.saving || self.store_write_busy;
    let mut items = items.peekable();
    let empty = items.peek().is_none();
    let row = h_flex().gap_3().flex_wrap().items_start().children(items.map(|item| {
      let asset = &item.asset;
      let mut tile = v_flex().id(SharedString::from(asset.id.clone())).gap_2().flex_none().w_32();
      if wide {
        tile = tile.w_full().max_w(rems(28.));
      }
      let preview = div().w_full().h_56().rounded_md().bg(c.border).overflow_hidden();
      tile = tile.child(match &item.image {
        Some(image) => preview.child(img(image.clone()).size_full().object_fit(ObjectFit::Contain)),
        None => preview.flex().items_center().justify_center().child(tr(l, "previewUnavailable")),
      });
      if asset.kind == "APP_PREVIEW" {
        tile = tile.children(asset.video.as_ref().map(|url| store_link("play-app-preview", tr(l, "playPreview"), url.clone())));
      }
      tile = tile.child(
        h_flex()
          .gap_1()
          .flex_wrap()
          .children(item.image.as_ref().map(|image| {
            let image = image.clone();
            Button::new("preview-product-asset").label(tr(l, "previewMedia")).small().outline().on_click(move |_, window, cx| {
              let image = image.clone();
              window.open_dialog(cx, move |dialog, _, _| {
                dialog
                  .width(MEDIA_PREVIEW_WIDTH)
                  .title(tr(l, "previewMedia"))
                  .child(img(image.clone()).w_full().h_96().object_fit(ObjectFit::Contain))
              });
            })
          }))
          .children(editable.then(|| {
            let asset = asset.id.clone();
            Button::new("remove-product-placement")
              .label(tr(l, "removePlacement"))
              .small()
              .outline()
              .disabled(busy)
              .on_click(cx.listener(move |this, _, window, cx| this.confirm_remove_placement(asset.clone(), window, cx)))
          })),
      );
      if !asset.state.is_empty() {
        tile = tile.child(div().text_sm().text_color(c.text_secondary).child(display_name(&asset.state)));
      }
      tile
    }));
    v_flex().id(SharedString::from(format!("media-gallery-{group}"))).min_w_0().w_full().child(if empty {
      row.child(tr(l, "noProductAssets"))
    } else {
      row
    })
  }

  fn confirm_remove_placement(&self, placement: String, window: &mut Window, cx: &mut Context<Self>) {
    let app = self.selected.clone();
    let workspace = cx.weak_entity();
    let l = self.settings.language;
    window.open_dialog(cx, move |dialog, _, _| {
      let (app, workspace, placement) = (app.clone(), workspace.clone(), placement.clone());
      dialog.title(tr(l, "removePlacement")).child(div().child(tr(l, "removePlacementHelp"))).footer(
        h_flex()
          .gap_2()
          .justify_end()
          .child(Button::new("cancel-remove-placement").label(tr(l, "cancel")).outline().on_click(|_, window, cx| window.close_dialog(cx)))
          .child(Button::new("confirm-remove-placement").label(tr(l, "removePlacement")).danger().on_click(move |_, window, cx| {
            workspace
              .update(cx, |this, cx| {
                if this.selected != app || this.saving || this.store_write_busy {
                  return;
                }
                let placement = placement.clone();
                window.close_dialog(cx);
                this.run_save(window, cx, move |client| client.remove_product_placement(&placement));
              })
              .ok();
          })),
      )
    });
  }

  /// The license agreement: Apple's standard one (a link to its text) or the app's own, edited in App Store Connect.
  fn license_agreement(&self, app: &StoreApp, custom: Option<bool>, c: &crate::theme::Colors) -> Div {
    let l = self.settings.language;
    let value = match custom {
      Some(false) => {
        let url = "https://www.apple.com/legal/internet-services/itunes/dev/stdeula/";
        div()
          .id("standard-license")
          .cursor_pointer()
          .text_color(c.accent)
          .child(tr(l, "standardLicenseAgreement"))
          .on_click(move |_, _, cx| cx.open_url(url))
          .into_any_element()
      }
      Some(true) => div().child(tr(l, "customLicenseAgreement")).into_any_element(),
      None => div().child("—").into_any_element(),
    };
    let edit_url = store_info(app);
    v_flex()
      .gap_0p5()
      .child(
        h_flex().gap_3().child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, "licenseAgreement"))).child(
          div().id("edit-license").cursor_pointer().text_color(c.accent).child(tr(l, "edit")).on_click(move |_, _, cx| cx.open_url(&edit_url)),
        ),
      )
      .child(value)
  }

  pub(crate) fn render_general(&self, app: &StoreApp, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let c = colors(cx);
    let info = self.ready_snapshot().map(|snapshot| &snapshot.general);
    let field = |key: &'static str, value: &str| info_field(l, &c, key, value);
    let general = v_flex()
      .gap_3()
      .child(pair(field("bundleId", &app.bundle_id), field("sku", &app.sku)))
      .child(pair(field("appleId", &app.id), field("primaryLanguage", locale_name(&app.primary_locale))));
    let rights = match app.content_rights.as_str() {
      "DOES_NOT_USE_THIRD_PARTY_CONTENT" => tr(l, "noThirdPartyContent"),
      "USES_THIRD_PARTY_CONTENT" => tr(l, "usesThirdPartyContent"),
      _ => SharedString::from("—"),
    };
    let general = general.child(field("contentRights", &rights)).child(self.license_agreement(app, info.and_then(|info| info.custom_license), &c));
    let mut categories = v_flex().gap_2();
    if let Some(info) = info {
      for (label, values) in [("primaryCategory", &info.categories[..3]), ("secondaryCategory", &info.categories[3..])] {
        let value = values.iter().filter(|item| !item.is_empty()).map(|item| display_name(item)).collect::<Vec<_>>().join(" / ");
        categories = categories.child(field(label, &value));
      }
    }
    let age_ratings = self.render_age_ratings_section(app, cx);
    v_flex()
      .gap_6()
      .child(page_section(
        tr(l, "generalInformation"),
        general.child(categories).child(h_flex().child(store_link("manage-general-info", tr(l, "manageCategoriesAndRights"), store_info(app)))),
        &c,
      ))
      .child(age_ratings)
      .children(self.render_information_extras(app, cx))
  }
}
