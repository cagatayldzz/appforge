//! The app list, icons, versions, the selected app and its snapshot, and moving between pages.

use super::*;

impl Workspace {
  pub fn load_apps(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(client) = self.client.clone() else { return };
    self.apps = Load::Loading;
    self.tasks.apps = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { client.apps() }).await;
      this
        .update_in(cx, |this, window, cx| {
          if this.require_apple_sign_in(&result, window, cx) {
            return;
          }
          match result {
            Ok(apps) => {
              let first = apps.first().map(|app| app.id.clone());
              let ids: Vec<String> = apps.iter().map(|app| app.id.clone()).collect();
              this.load_icons(ids.clone(), window, cx);
              this.load_versions(ids, window, cx);
              this.apps = Load::Ready(apps.into());
              if this.selected.is_none() {
                if let Some(id) = first {
                  this.select_app(id, window, cx);
                }
              }
            }
            Err(error) => {
              this.apps = Load::Failed(error.to_string().into());
              Self::error(window, cx, error.to_string());
            }
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  /// Asks App Store Connect for the version in front of each app; the cards show them as they arrive.
  pub(super) fn load_versions(&mut self, ids: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
    let Some(client) = self.client.clone() else { return };
    self.tasks.versions = Some(cx.spawn_in(window, async move |this, cx| {
      let found = cx.background_spawn(async move { client.current_versions(&ids) }).await;
      this
        .update(cx, |this, cx| {
          this.versions.extend(found);
          cx.notify();
        })
        .ok();
    }));
  }

  /// Shows the icons of the apps: first the ones kept on disk, so they appear at once, then the ones that are missing or old,
  /// asked from App Store Connect. An app with no icon there keeps its letter.
  pub(super) fn load_icons(&mut self, ids: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
    let Some(client) = self.client.clone() else { return };
    let cache = IconCache::new(&self.data_dir);
    self.tasks.icons = Some(cx.spawn_in(window, async move |this, cx| {
      let kept = {
        let (cache, ids) = (cache.clone(), ids.clone());
        cx.background_spawn(async move { ids.into_iter().filter_map(|id| cache.path(&id).map(|path| (id, path))).collect::<Vec<_>>() }).await
      };
      this
        .update(cx, |this, cx| {
          this.icons.extend(kept);
          cx.notify();
        })
        .ok();
      let (fetched, failure) = cx
        .background_spawn(async move {
          let mut fetched = Vec::new();
          let mut failure = None;
          for id in ids.into_iter().filter(|id| !cache.is_fresh(id)) {
            match client.app_icon(&id, ICON_SIZE) {
              Ok(Some(bytes)) => match cache.store(&id, &bytes) {
                Ok(path) => fetched.push((id, path)),
                Err(error) => failure = failure.or(Some(format!("Could not keep the icon of app {id}: {error}"))),
              },
              // An app with no build has no icon: it keeps its letter.
              Ok(None) => {}
              Err(error) => failure = failure.or(Some(error.to_string())),
            }
          }
          (fetched, failure)
        })
        .await;
      this
        .update_in(cx, |this, window, cx| {
          // One notice, and only when not a single icon could be had: a failure for one app is not worth interrupting for.
          if let (true, Some(error)) = (fetched.is_empty(), failure) {
            Self::warning(window, cx, trf(this.settings.language, "couldNotGetThe", &[&error]));
          }
          this.icons.extend(fetched);
          cx.notify();
        })
        .ok();
    }));
  }

  /// Opens a page from a link inside another page, remembering where it came from.
  /// The page whose data and Save the screen works on: the version page is several pages in one (as in App Store Connect), and
  /// its tabs save, reload and keep history like the pages they stand for.
  pub(crate) fn effective_tab(&self) -> DetailTab {
    match (self.tab, self.product_tab) {
      (DetailTab::ProductPage, 2) => DetailTab::Review,
      (DetailTab::ProductPage, 3) => DetailTab::Distribution,
      (tab, _) => tab,
    }
  }

  /// Opens a page. Review and Distribution are tabs of the version page, which also holds the Product Page.
  pub(crate) fn jump_to(&mut self, tab: DetailTab, window: &mut Window, cx: &mut Context<Self>) {
    let (tab, sub) = match tab {
      DetailTab::ProductPage => (DetailTab::ProductPage, Some(0)),
      DetailTab::Review => (DetailTab::ProductPage, Some(2)),
      DetailTab::Distribution => (DetailTab::ProductPage, Some(3)),
      tab => (tab, None),
    };
    if self.tab != tab {
      self.back_tab = Some(self.tab);
      self.tab = tab;
    }
    if let Some(sub) = sub {
      self.product_tab = sub;
    }
    self.ensure_app_page(window, cx);
    cx.notify();
  }

  /// Goes back to the page a jump came from.
  pub(crate) fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(tab) = self.back_tab.take() {
      self.tab = tab;
      self.ensure_app_page(window, cx);
    }
    cx.notify();
  }

  pub fn select_app(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
    self.show_settings = false;
    self.show_all_apps = false;
    if self.selected.as_ref() != Some(&id) {
      self.back_tab = None;
      self.selected = Some(id.clone());
      self.locale.clear();
      self.review_territory = None;
      self.review_rating = None;
      self.review_status = Default::default();
      self.reset_ai_analysis(window, cx);
      self.report = None;
      self.app_data.entry(id).or_default();
      self.fill_snapshot_forms(window, cx);
    }
    self.ensure_app_page(window, cx);
    cx.notify();
  }

  pub(crate) fn active_data(&self) -> Option<&AppData> {
    self.app_data.get(self.selected.as_ref()?)
  }

  pub(crate) fn ensure_app_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.tab == DetailTab::CustomerReviews && self.active_data().is_some_and(|data| matches!(data.reviews, Load::Idle)) {
      self.load_customer_reviews(window, cx);
    }
    if self.tab != DetailTab::CustomerReviews && self.active_data().is_some_and(|data| matches!(data.snapshot, Load::Idle)) {
      self.load_snapshot(window, cx);
    }
    if self.tab == DetailTab::ProductPage && self.product_tab == 1 {
      self.load_product_assets(false, window, cx);
    }
    if matches!(self.tab, DetailTab::ProductPage | DetailTab::Distribution | DetailTab::Analysis)
      && self.active_data().is_some_and(|data| matches!(data.distribution, Load::Idle))
    {
      self.load_distribution(window, cx);
    }
  }

  /// The apps that are loaded (none while loading), as a shared handle.
  pub(crate) fn app_list(&self) -> Arc<[StoreApp]> {
    match &self.apps {
      Load::Ready(apps) => apps.clone(),
      _ => Arc::from([]),
    }
  }

  pub fn selected_app(&self) -> Option<StoreApp> {
    let Load::Ready(apps) = &self.apps else { return None };
    apps.iter().find(|app| Some(&app.id) == self.selected.as_ref()).cloned()
  }

  pub fn ready_snapshot(&self) -> Option<&AppSnapshot> {
    match &self.active_data()?.snapshot {
      Load::Ready(snapshot) => Some(snapshot),
      _ => None,
    }
  }

  pub fn privacy_declaration(&self) -> PrivacyDeclaration {
    self.selected.as_ref().and_then(|id| self.settings.privacy.get(id)).cloned().unwrap_or_default()
  }

  pub fn load_snapshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.refresh_snapshot(None, window, cx);
  }

  pub(crate) fn refresh_snapshot(&mut self, from: Option<DetailTab>, window: &mut Window, cx: &mut Context<Self>) {
    let (Some(client), Some(app)) = (self.client.clone(), self.selected_app()) else { return };
    let id = app.id.clone();
    let data = self.app_data.entry(id.clone()).or_default();
    if data.snapshot_task.is_some() || (self.saving && matches!(data.snapshot, Load::Ready(_))) {
      return;
    }
    if !matches!(data.snapshot, Load::Ready(_)) {
      data.snapshot = Load::Loading;
    }
    let notice = from.map(|_| super::refresh::Refresh::AppDetails);
    if let Some(notice) = notice {
      notice.start(&id, self.settings.language, window, cx);
    }
    data.snapshot_task = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { client.snapshot(&app) }).await;
      this
        .update_in(cx, |this, window, cx| {
          if let Some(notice) = notice {
            notice.finish(&id, this.settings.language, &result, window, cx);
          }
          if this.require_apple_sign_in(&result, window, cx) {
            return;
          }
          let Some(data) = this.app_data.get_mut(&id) else { return };
          data.snapshot_task = None;
          let active = this.selected.as_ref() == Some(&id);
          match result {
            Ok(snapshot) => {
              let same_version = matches!(&data.snapshot, Load::Ready(previous) if previous.version == snapshot.version);
              if !same_version {
                data.distribution = Load::Idle;
                data.distribution_task = None;
              }
              data.snapshot = Load::Ready(snapshot);
              if active {
                this.fill_snapshot_forms(window, cx);
                if matches!(from, Some(DetailTab::Distribution)) {
                  this.refresh_distribution(true, window, cx);
                }
                this.ensure_app_page(window, cx);
              }
            }
            Err(error) => {
              if !matches!(data.snapshot, Load::Ready(_)) {
                data.snapshot = Load::Failed(error.to_string().into());
              }
              if active && this.tab != DetailTab::CustomerReviews {
                if notice.is_none() {
                  Self::error(window, cx, error.to_string());
                }
                this.ensure_app_page(window, cx);
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

  pub(super) fn fill_snapshot_forms(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(snapshot) = self.ready_snapshot().cloned() else { return };
    let available = locales(&snapshot);
    if !available.contains(&self.locale) {
      self.locale = available.first().cloned().unwrap_or_default();
    }
    self.forms.fill_locale(&snapshot, &self.locale, window, cx);
    self.forms.fill_review(snapshot.review_detail.as_ref(), window, cx);
    let declaration = self.privacy_declaration();
    set_input(&self.forms.data_types, &declaration.data_types, window, cx);
    set_input(&self.forms.sdks, &declaration.third_party_sdks, window, cx);
    self.forms.privacy = declaration;
    self.recompute_report();
  }

  pub fn select_locale(&mut self, locale: String, window: &mut Window, cx: &mut Context<Self>) {
    if self.locale == locale {
      return;
    }
    self.locale = locale;
    if let Some(snapshot) = self.ready_snapshot() {
      let snapshot = snapshot.clone();
      self.forms.fill_locale(&snapshot, &self.locale, window, cx);
    }
    self.ensure_app_page(window, cx);
    cx.notify();
  }

  pub fn recompute_report(&mut self) {
    let declaration = self.privacy_declaration();
    self.report = self.ready_snapshot().map(|snapshot| Arc::new(check(snapshot, &declaration)));
  }
}
