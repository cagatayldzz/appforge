# Architecture

AppForge is a Rust Cargo workspace. The executable in `apps/appforge` calls `appforge-app`, which initializes GPUI Kit and opens an `appforge-ui` workspace. GPUI Kit uses the `gpui-fast` backend.

```
apps/appforge → app → ui → compliance → appstore
                             │            → ai
                             │            → storage → ai
                             → project
```

`ai`, `appstore`, `compliance`, `project` and `storage` never depend on the interface and are tested without a window. Their calls are blocking, so the interface runs them on a background task and never waits for them on its own thread.

## Crates

| Crate | Responsibility |
| --- | --- |
| `appstore` | Blocking App Store Connect API client: ES256 token from the API key, apps, App Information, listing, screenshots, review details and app icons (from the newest build that has one); PATCH of only the changed fields. |
| `ai` | One call, `complete` (and `complete_with_usage`, which also returns the tokens used), for OpenAI (chat completions) and Claude (messages) with an API key. It lists the low-cost models Settings offers (`Provider::models`) with their prices per million tokens and prices a request (`ModelInfo::cost`); models that think by default are asked to think as little as possible. |
| `compliance` | Built-in rules over an app snapshot, the Apple guidelines catalog, the playbook download and cache, the AI analysis (prompt, JSON answer) and the AI-written store listing (`draft_listing_field`: prompt, answer cut to Apple's limits). |
| `project` | Reads a short description of an app's source folder for the AI: the folder shape, README, manifests and the texts the app shows. Never reads secrets (`.env`, keys, signing files) or follows links; at most about 10,000 tokens. |
| `storage` | The data directory: `settings.json` (language, account ids, AI choices, per-app privacy answers and code folders, the window as it was left), `ai-usage.json` (`UsageLedger`), `icons/<app id>.png` (`IconCache`) and the Keychain (`SecretStore`). |
| `ui` | The window: sign-in, the bars and pages of the signed-in window, Analysis and Settings. |
| `updater` | Checks GitHub releases, verifies and stages macOS app bundles, and replaces the installed bundle on restart. The UI runs its blocking calls on background tasks. |
| `app` | Startup: the window (opened the way it was left), key bindings and the menu. |

Detail pages use the content width beside the two navigation rails to choose between one and two columns. Forms keep the editable sections together and place related reference information beside them when space allows. Media and review lists can use a wider frame. The all-apps grid adapts its column count to the full area beside the activity bar.

## Code map (`crates/ui/src`)

For all crates in one list, run `scripts/code-map.sh`: every source file with its line count and the first line of its `//!` header (about 3,000 tokens). Keep that header true when a file's job changes; a file without one shows as `NO HEADER`.

| File | Owns |
| --- | --- |
| `workspace/mod.rs` | `Workspace`, the one state of the window (fields, `Load<T>`, tabs, `Tasks`, `AppData`) and `new`. Behavior lives in the sibling files below, all `impl Workspace`. |
| `workspace/session.rs` | Startup from the Keychain, sign-in and sign-out, the `.p8` file. |
| `workspace/apps.rs` | The app list (`Arc<[App]>`, shared), icons, versions, the selected app, its snapshot, the report (`Arc<Report>`), page jumps and Back. |
| `workspace/saving.rs` | Saving the pages to App Store Connect and the privacy answers. |
| `workspace/ai.rs` | The AI analysis, the playbook, provider and keys, and the AI usage totals. |
| `workspace/settings.rs` | Saving settings, the window place, code folders. |
| `view.rs` | `Render for Workspace`: the title band and which screen shows. |
| `welcome.rs`, `login.rs`, `setup.rs`, `guide_art.rs` | First run: the welcome screen, the sign-in guide, its frame (sidebar, progress, pinned actions) and the small drawings of Apple's page. |
| `sidebar.rs` | The signed-in bars: the activity bar and the sidebar (app, page menu, Reload and Save; or the pages of Settings). |
| `apps_view.rs` | The grid of all apps. |
| `detail.rs`, `forms.rs` | The app detail layout and the forms for listing, review and privacy. |
| `privacy_preview.rs` | The local preview of the App Privacy label. |
| `store_pages.rs` | Pages App Store Connect owns (App Compliance: the export compliance declarations from the API, whether the medical device and Vietnam game license declarations apply (from the categories and age rating answers), and a link for each of the four declarations the API does not offer; App Accessibility: explanation and link) and the rejection notice of Distribution. |
| `product_page.rs`, `localization.rs` | Per-language media from Apple’s Asset Library, general App Information summaries and locale creation. Media and decoded image inputs are cached in memory by app and version localization. |
| `customer_reviews.rs`, `customer_reviews/cards.rs` | Written-review ratings, country and version filtering, and developer replies. Apple’s app- and version-scoped review requests run on demand. |
| `distribution.rs` | iOS version history, build selection, review submissions and release controls. Requests run on background tasks retained by the workspace. |
| `review_submissions.rs` | The App Review page: the review submissions as a table, as App Store Connect lists them. |
| `analysis.rs` | The Analysis page: score, findings, AI analysis, rules. |
| `listing_ai.rs` | Field-specific AI actions, confirmation dialogs, and background generation. |
| `settings_view.rs` | The pages of Settings. |
| `widgets.rs`, `theme.rs` | Shared pieces (sections, rows, avatars, badges) and the palettes. |
| `window_state.rs` | Opening the window the way it was left. |
| `i18n.rs`, `../locales/en.json`, `../locales/tr.json` | `tr` and `trf`. The texts are flat `"key": "text"` JSON files embedded at build time. |

## Window layout

A title band (drag area, the rail color) sits above the content.

**Signed out**: a fixed sidebar holds the brand and language. The welcome screen uses the main workspace’s section layout with the progress and Get started action in the sidebar. During sign-in, progress appears above the current step and Back and Next stay pinned at the bottom. The steps are the welcome screen (first launch only), then the three-step sign-in guide: a checklist of five things to do on Apple's site (the drawings are pictures of Apple's page, labelled as such, and only the first button is real), the Issuer ID and Key ID, and the `.p8` file.

**Signed in**: three columns.

- The **activity bar** (56 px): the Apps button, one avatar button per app (its icon, or its first letter), and Settings at the bottom.
- The **sidebar** (280 px): at the top the open app (icon, name, bundle id, the version with where it stands at Apple, and the review score, which opens the Analysis page), under it the pages of the app as a menu grouped like App Store Connect (iOS App: the version chip, which opens the version page; General: App Information, App Compliance, App Review; App Store › Trust & Safety: App Privacy, App Accessibility, Ratings and Reviews). The version page (`DetailTab::ProductPage`, titled "iOS App Version 1.0") has four tabs: listing texts, media, App Review Information (contact, sign-in, notes) and build and release (the Distribution content); the sidebar's App Review row is its own page (`DetailTab::Submissions`, `review_submissions.rs`): the submissions sent to App Review as a table (date, versions, submitted by, items, status), each date opening the submission in App Store Connect. `Workspace::effective_tab` maps these tabs to the page whose Save, Reload and history they use. App Compliance and App Accessibility only link to App Store Connect. Analysis is not listed; the score tile opens it, and at the bottom Reload and Save (sidebar Save on App Information, App Review and App Privacy). While Settings is open it lists the pages of Settings instead. It does not show while the grid of all apps is open.
- The **main area**: the page, which scrolls, in flat sections (a heading with a line under it, no boxes). The grid of all apps (`apps_view.rs`, three columns) takes the whole width.

The app has these pages: the version page (**iOS App Version**, with listing texts, media, App Review Information and build and release), **App Information** (its display-only tab includes the age ratings), **App Compliance**, **App Review** (the submissions), **App Privacy**, **App Accessibility**, **Ratings and Reviews**, and **Analysis** (AppForge's own, opened from the score tile). Settings has General, Account, AI provider, Code folders, Rules and About.

App Information exposes AI generation inside each supported listing field. The confirmation names the target field and locale. The background task sends the listing and project summary to the configured provider, then updates only the matching field if it was left unchanged during the request.

## Interface rules

- GPUI Kit components are preferred (`Sidebar`, `Select`, `Dialog`, `Notification`, `Progress`, `Input`, `Button`); a hand-built element is fine when it is clearly better, kept in one shared component (`widgets.rs`) with colors from the theme.
- The font is Geist (`crates/ui/assets/fonts`, SIL Open Font License), installed at startup by `install_fonts`. Text is 14 px (`FONT_SIZE`); the Kit's own parts are sized in `rem`, so the theme's `font_size` is never changed.
- `theme.rs` holds the light and dark palettes (GitHub Primer colors), applied to the Kit theme and following the system appearance.
- Every screen is built to be verified by running it (`bun run dev`).

## Languages

The interface language (English or Türkçe) is `Settings.language`. Texts live in `crates/ui/locales/en.json` and `tr.json`, flat `"camelCaseKey": "text"` objects such as `"createAccount": "Create Account"`. Code passes the key to `tr`/`trf` (`tr(l, "createAccount")`); `en.json` is the English wording, so English can be reworded without touching code. English text that comes from another crate (the `compliance` severities and categories) is translated by its wording. Unknown text is shown as it is. `crates/ui/tests/i18n.rs` fails when a key used in code is missing, when the two files have different keys, or when a file has duplicates, empty texts or different `{}` placeholders. To add a language: a new `locales/xx.json`, a `Language` variant and a line in `i18n::source`. Add a new text by adding its key to both files and using the key in code. Placeholders in text fields, the rules and AI findings, model names and the words inside the drawings of Apple's page are not translated.

## Data directory

`~/Library/Application Support/AppForge` (override with `APPFORGE_DATA_DIR`):

| Path | Holds |
| --- | --- |
| `settings.json` | Language, whether the welcome screen was done, Issuer ID and Key ID, the AI provider and models, privacy answers per app, code folders per app, the window. A file that cannot be read is never overwritten. |
| `ai-usage.json` | What the AI features used: requests, tokens and estimated cost per model since the last reset (`UsageLedger`). A file that cannot be read is never overwritten. |
| `icons/<app id>.png` | App icons, refreshed when a day old. |
| `apple-guidelines.json` | The last validated Apple guidelines catalog, check time and latest per-rule changes. |
| `playbook/` | The cached compliance playbook, refreshed weekly. |

Secrets (the `.p8` key, AI API keys) are only in the macOS Keychain, service `AppForge`, in release builds. Development builds (`bun run dev`) keep them in `dev-secrets.json` in the data directory (mode 0600, `FileSecrets`), because an ad-hoc signed build is new every time and the Keychain would ask for its password after each build. Sign in again once in a development build.

## Window state

The window opens the way it was left: size, place, and maximized or full screen (`Settings.window`). `Workspace::remember_window` writes it half a second after the last change; `ui::initial_window_bounds` reads it at startup and falls back to the middle of the main display when the saved place is no longer on a screen.


## App Store Connect API schema

`crates/appstore/schema/openapi.oas.json` contains the complete OpenAPI document from [Apple’s specification archive](https://developer.apple.com/sample-code/app-store-connect/app-store-connect-openapi-specification.zip). The checked-in baseline is API 4.6.1. The application uses the existing App Store Connect client at runtime.

- `bun run appstore:schema` downloads the latest archive and reports added, removed, changed, and newly deprecated operations and changed reusable components.
- `bun run appstore:schema --write` shows the same report and replaces the checked-in baseline with the validated Apple document.
- `bun run appstore:schema /path/to/openapi.oas.json` compares a local download. An Apple ZIP can be passed in the same way, with optional `--write`.

Compare before updating the baseline and review the resulting Git diff for exact field changes.

Confirmed form saves are recorded in the private local `form-history.git` repository in the data directory. Historical demo passwords are stored in Keychain.

The `ai-reviews/` data subdirectory keeps the latest successful AI analysis per account, app, version, interface language and analysis mode. It contains findings and run metadata.
