//! The Pricing and Availability page: the price schedule, what every country or region costs and where the app is on sale,
//! with every change written through the App Store Connect API.

use crate::{
  forms::{read_input, set_input},
  i18n::{tr, trf},
  theme::{colors, Colors, FONT_SIZE_SM},
  workspace::{Load, Workspace},
};
use appforge_appstore::{territory_code, territory_name, Client, Error, PricePeriod, Pricing};
use chrono::{Local, NaiveDate};
use gpui_kit::{
  component::{
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    spinner::Spinner,
    switch::Switch,
    v_flex, Disableable as _, Icon, IconName, Sizable as _, WindowExt as _,
  },
  *,
};

impl Workspace {
  pub(crate) fn load_pricing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.refresh_pricing(false, window, cx);
  }

  pub(crate) fn refresh_pricing(&mut self, announce: bool, window: &mut Window, cx: &mut Context<Self>) {
    let (Some(client), Some(app)) = (self.client.clone(), self.selected.clone()) else { return };
    let Some(data) = self.app_data.get_mut(&app) else { return };
    if data.pricing_task.is_some() {
      return;
    }
    if !matches!(data.pricing, Load::Ready(_)) {
      data.pricing = Load::Loading;
    }
    let target = app.clone();
    let notice = crate::workspace::refresh::Refresh::Pricing;
    if announce {
      notice.start(&app, self.settings.language, window, cx);
    }
    data.pricing_task = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { client.pricing(&target) }).await;
      this
        .update_in(cx, |this, window, cx| {
          if announce {
            notice.finish(&app, this.settings.language, &result, window, cx);
          }
          if this.require_apple_sign_in(&result, window, cx) {
            return;
          }
          let Some(data) = this.app_data.get_mut(&app) else { return };
          data.pricing_task = None;
          let active = this.selected.as_ref() == Some(&app);
          match result {
            Ok(pricing) => data.pricing = Load::Ready(pricing),
            Err(error) => {
              if !matches!(data.pricing, Load::Ready(_)) {
                data.pricing = Load::Failed(error.to_string().into());
              }
              if active && !announce {
                Self::error(window, cx, error.to_string());
              }
            }
          }
          if active {
            cx.notify();
          }
        })
        .ok();
    }));
    cx.notify();
  }

  /// Runs one write to App Store Connect, then reads the page again.
  fn run_pricing_write(&mut self, window: &mut Window, cx: &mut Context<Self>, work: impl FnOnce(&Client) -> Result<(), Error> + Send + 'static) {
    let (Some(client), Some(app)) = (self.client.clone(), self.selected.clone()) else { return };
    if self.store_write_busy || self.saving {
      return;
    }
    self.store_write_busy = true;
    self.tasks.store_write = Some(cx.spawn_in(window, async move |this, cx| {
      let worker = client.clone();
      let result = cx.background_spawn(async move { work(&worker) }).await;
      this
        .update_in(cx, |this, window, cx| {
          this.store_write_busy = false;
          if !this.client.as_ref().is_some_and(|current| std::sync::Arc::ptr_eq(current, &client)) || this.require_apple_sign_in(&result, window, cx)
          {
            return;
          }
          match &result {
            Ok(()) => Self::success(window, cx, tr(this.settings.language, "savedToAppStore")),
            Err(error) => Self::error(window, cx, error.to_string()),
          }
          if let Some(data) = this.app_data.get_mut(&app) {
            data.pricing_task = None;
            if this.selected.as_ref() != Some(&app) {
              data.pricing = Load::Idle;
            }
          }
          if this.selected.as_ref() == Some(&app) {
            this.refresh_pricing(false, window, cx);
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  /// Sets a price, in the base territory or, with `new_base`, in the territory that becomes the base.
  fn change_price(&mut self, territory: String, price: String, start: String, new_base: bool, window: &mut Window, cx: &mut Context<Self>) {
    let Some(app) = self.selected.clone() else { return };
    let periods: Vec<PricePeriod> = match self.active_data().map(|data| &data.pricing) {
      Some(Load::Ready(pricing)) if !new_base => pricing.periods.clone(),
      _ => Vec::new(),
    };
    self.run_pricing_write(window, cx, move |client| {
      let territory = territory_code(&territory).ok_or_else(|| Error::Unexpected(format!("\"{territory}\" is not a country or region.")))?;
      let wanted: f64 = price.replace(',', ".").parse().map_err(|_| Error::Unexpected(format!("\"{price}\" is not a price.")))?;
      let start = match start.as_str() {
        "" => None,
        text => Some(NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|_| Error::Unexpected("Enter the start date as 2027-01-31.".into()))?),
      };
      let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
      // A new schedule replaces the old one: what is still ahead, and what applies now when the new price starts later, is sent again.
      let keep: Vec<PricePeriod> = periods
        .into_iter()
        .filter(|period| {
          let ended = !period.end.is_empty() && period.end < today;
          let ahead = !period.start.is_empty() && period.start > today;
          !ended && (start.is_some() || ahead)
        })
        .collect();
      let points = client.price_points(&app, territory)?;
      let point = points
        .iter()
        .find(|point| point.price.parse::<f64>().is_ok_and(|value| (value - wanted).abs() < 0.005))
        .ok_or_else(|| Error::Unexpected(format!("Apple has no price of {price} in {}.", territory_name(territory))))?;
      client.set_price(&app, territory, &point.id, start.map(|date| date.format("%Y-%m-%d").to_string()).as_deref(), &keep)
    });
  }

  /// "Change price" and "Edit base country or region" share one dialog; the base one asks for the country too.
  fn open_price_dialog(&mut self, new_base: bool, window: &mut Window, cx: &mut Context<Self>) {
    let l = self.settings.language;
    let base = match self.active_data().map(|data| &data.pricing) {
      Some(Load::Ready(pricing)) => pricing.base.clone().unwrap_or_default(),
      _ => String::new(),
    };
    let territory = cx.new(|cx| InputState::new(window, cx).placeholder("United States"));
    set_input(&territory, territory_name(&base), window, cx);
    let price = cx.new(|cx| InputState::new(window, cx).placeholder("0.99"));
    let start = cx.new(|cx| InputState::new(window, cx).placeholder("2027-01-31"));
    let workspace = cx.entity();
    let title = tr(l, if new_base { "editBaseCountry" } else { "changePrice" });
    window.open_dialog(cx, move |dialog, _, _| {
      let (workspace, territory, price, start) = (workspace.clone(), territory.clone(), price.clone(), start.clone());
      let field = |label: SharedString, input: &Entity<InputState>| v_flex().gap_1().child(div().text_sm().child(label)).child(Input::new(input));
      let mut form = v_flex().gap_3();
      if new_base {
        form = form.child(field(tr(l, "baseCountry"), &territory));
      }
      form = form.child(field(tr(l, "priceLabel"), &price));
      if !new_base {
        form = form.child(field(tr(l, "startDate"), &start)).child(div().text_sm().child(tr(l, "startDateHelp")));
      }
      let footer = h_flex()
        .justify_end()
        .gap_2()
        .w_full()
        .child(Button::new("price-cancel").label(tr(l, "cancel")).outline().on_click(|_, window, cx| window.close_dialog(cx)))
        .child(Button::new("price-confirm").label(tr(l, "save")).primary().on_click(move |_, window, cx| {
          let (territory, price, start) = (read_input(&territory, cx), read_input(&price, cx), read_input(&start, cx));
          window.close_dialog(cx);
          workspace.update(cx, |this, cx| this.change_price(territory, price, start, new_base, window, cx));
        }));
      dialog.title(title.clone()).child(form).footer(footer)
    });
  }

  fn set_territory_available(&mut self, id: String, available: bool, window: &mut Window, cx: &mut Context<Self>) {
    self.run_pricing_write(window, cx, move |client| client.set_territory_available(&id, available));
  }

  /// Writes every territory's price to a file in the Downloads folder.
  fn download_prices(&self, pricing: &Pricing, window: &mut Window, cx: &mut Context<Self>) {
    let l = self.settings.language;
    let mut csv = String::from("Country or region,Code,Currency,Customer price,Proceeds\n");
    for item in &pricing.prices {
      let name = territory_name(&item.territory).replace('"', "\"\"");
      csv.push_str(&format!("\"{name}\",{},{},{},{}\n", item.territory, item.currency, item.price, item.proceeds));
    }
    let name = self.selected_app().map(|app| app.bundle_id).unwrap_or_else(|| "app".into());
    let target = std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join("Downloads").join(format!("{name}-prices.csv")));
    match target.map(|path| std::fs::write(&path, csv).map(|()| path)) {
      Some(Ok(path)) => Self::success(window, cx, trf(l, "savedTo", &[&path.display().to_string()])),
      Some(Err(error)) => Self::error(window, cx, error.to_string()),
      None => Self::error(window, cx, tr(l, "downloadsUnavailable")),
    }
  }

  pub(crate) fn render_pricing(&self, cx: &mut Context<Self>) -> AnyElement {
    let l = self.settings.language;
    let c = colors(cx);
    let pricing = match self.active_data().map(|data| &data.pricing) {
      Some(Load::Ready(pricing)) => pricing,
      Some(Load::Failed(message)) => return Alert::error("pricing-error", message.clone()).into_any_element(),
      _ => return h_flex().gap_2().child(Spinner::new()).child(tr(l, "loadingFromAppStore")).into_any_element(),
    };
    let busy = self.store_write_busy || self.saving;
    let muted = |text: SharedString| div().text_color(c.text_secondary).child(text);
    let link = |id: &'static str, label: SharedString| div().id(id).cursor_pointer().text_color(c.accent).child(label);

    // Price Schedule ---------------------------------------------------------------------------------------------------
    let base = match &pricing.base {
      Some(code) => format!("{} ({})", territory_name(code), pricing.base_currency),
      None => "—".to_owned(),
    };
    let base_row = h_flex()
      .gap_2()
      .child(div().child(base))
      .child(link("edit-base-country", tr(l, "edit")).on_click(cx.listener(|this, _, window, cx| this.open_price_dialog(true, window, cx))));
    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
    let mut rows = Vec::new();
    for period in pricing.periods.iter().filter(|period| period.end.is_empty() || period.end >= today) {
      let current = period.start.is_empty() || period.start <= today;
      let when = if current { tr(l, "currentPrice") } else { trf(l, "priceFrom", &[&period.start]) };
      let countries = if current { trf(l, "nCountries", &[&pricing.prices.len().to_string()]) } else { trf(l, "nCountries", &["1"]) };
      rows.push((when, countries, format!("{} {}", pricing.base_currency, period.price)));
    }
    let table = v_flex()
      .w_full()
      .child(Self::pricing_row(["dates", "countriesOrRegions", "priceAdjustment"].map(|key| tr(l, key).to_uppercase().into()), true, &c))
      .children(rows.into_iter().map(|(when, countries, price)| Self::pricing_row([when, countries, SharedString::from(price)], false, &c)));
    let expanded = self.distribution_expanded.contains("pricing-all");
    let all = link("all-prices", tr(l, if expanded { "hideAllPrices" } else { "allPricesAndCurrencies" })).on_click(cx.listener(|this, _, _, cx| {
      if !this.distribution_expanded.remove("pricing-all") {
        this.distribution_expanded.insert("pricing-all");
      }
      cx.notify();
    }));
    let all_prices = expanded.then(|| {
      v_flex()
        .w_full()
        .child(Self::pricing_row(["countryOrRegion", "currency", "priceLabel"].map(|key| tr(l, key).to_uppercase().into()), true, &c))
        .children(pricing.prices.iter().map(|item| {
          Self::pricing_row([territory_name(&item.territory).to_owned().into(), item.currency.clone().into(), item.price.clone().into()], false, &c)
        }))
    });
    let plus = Button::new("change-price")
      .icon(Icon::new(IconName::Plus))
      .small()
      .outline()
      .tooltip(tr(l, "changePrice"))
      .disabled(busy || pricing.base.is_none())
      .on_click(cx.listener(|this, _, window, cx| this.open_price_dialog(false, window, cx)));
    let download = {
      let snapshot = pricing.clone();
      link("download-prices", tr(l, "download")).on_click(cx.listener(move |this, _, window, cx| this.download_prices(&snapshot, window, cx)))
    };
    let schedule = v_flex()
      .gap_4()
      .child(h_flex().gap_3().items_center().child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(tr(l, "priceSchedule"))).child(plus))
      .child(muted(tr(l, "priceScheduleHelp")))
      .child(
        v_flex()
          .gap_2()
          .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(tr(l, "baseCountryTitle")))
          .child(muted(tr(l, "baseCountryHelp")))
          .child(base_row),
      )
      .child(
        v_flex()
          .gap_2()
          .child(
            h_flex()
              .justify_between()
              .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(tr(l, "prices")))
              .child(h_flex().gap_4().child(all).child(download)),
          )
          .child(table)
          .children(all_prices),
      );

    // App Availability -------------------------------------------------------------------------------------------------
    let managing = self.distribution_expanded.contains("pricing-availability");
    let availability = pricing.availability.as_ref().map(|availability| {
      let list = managing.then(|| {
        v_flex().w_full().gap_1().children(availability.territories.iter().map(|item| {
          let id = item.id.clone();
          let note = item
            .statuses
            .iter()
            .find(|status| status.starts_with("CANNOT_") || status.starts_with("MISSING_"))
            .map(|status| status.replace('_', " ").to_lowercase());
          h_flex()
            .id(SharedString::from(format!("territory-{}", item.territory)))
            .w_full()
            .gap_3()
            .px_3()
            .py_1()
            .items_center()
            .child(
              Switch::new(SharedString::from(format!("available-{}", item.territory)))
                .checked(item.available)
                .disabled(busy)
                .label(territory_name(&item.territory).to_owned())
                .on_click(cx.listener(move |this, value: &bool, window, cx| this.set_territory_available(id.clone(), *value, window, cx))),
            )
            .children(note.map(|note| div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(note)))
        }))
      });
      v_flex()
        .gap_3()
        .child(
          h_flex()
            .gap_3()
            .items_center()
            .child(div().size_2().rounded_full().bg(c.success))
            .child(div().child(trf(l, "nAvailable", &[&pricing.available_count().to_string()])))
            .child(link("manage-availability", tr(l, "manage")).on_click(cx.listener(|this, _, _, cx| {
              if !this.distribution_expanded.remove("pricing-availability") {
                this.distribution_expanded.insert("pricing-availability");
              }
              cx.notify();
            }))),
        )
        .children(list)
    });
    let availability = v_flex()
      .gap_3()
      .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(tr(l, "appAvailability")))
      .child(muted(tr(l, "appAvailabilityHelp")))
      .child(availability.unwrap_or_else(|| v_flex().child(muted(tr(l, "noAvailability")))));
    v_flex().gap_8().child(schedule).child(availability).into_any_element()
  }

  /// One row of a table with three columns, with rules between rows as App Store Connect draws them.
  fn pricing_row(cells: [SharedString; 3], header: bool, c: &Colors) -> Div {
    let cell = |text: SharedString| {
      let cell = div().flex_1().min_w_0().truncate().child(text);
      if header {
        cell.text_size(FONT_SIZE_SM).text_color(c.text_secondary)
      } else {
        cell
      }
    };
    let [first, second, third] = cells;
    h_flex()
      .w_full()
      .gap_3()
      .px_3()
      .py_3()
      .items_center()
      .border_b_1()
      .border_color(c.border)
      .child(cell(first))
      .child(cell(second))
      .child(cell(third))
  }
}
