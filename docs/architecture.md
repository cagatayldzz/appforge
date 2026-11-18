# Architecture

AppForge is a Rust Cargo workspace. The executable in `apps/appforge` calls `appforge-app`, which initializes GPUI Kit and opens an `appforge-ui` workspace. GPUI Kit uses the `gpui-fast` backend.

```
apps/appforge → app → ui → compliance → appstore
                                      → ai
                                      → storage → ai
```

| Crate | Responsibility |
| --- | --- |
| `appstore` | Blocking App Store Connect API client: ES256 token from the API key, apps, App Information, listing, screenshots and review details; PATCH of only the changed fields. |
| `ai` | One call, `complete`, for OpenAI (chat completions) and Claude (messages) with an API key. |
| `compliance` | Built-in rules over an app snapshot, the playbook download and cache, and the AI analysis (prompt, JSON answer). |
| `storage` | `icons/<app id>.png` (app icons, `IconCache`), `settings.json` (language, account, AI provider, the window as it was left), per-app privacy answers and the Keychain (`SecretStore`). |
| `ui` | The window: sign-in, sidebar, app screens, Analysis and Settings. |

## Window layout

The window opens the way it was left (size, place, maximized or full screen: `Settings.window`, written by `Workspace::remember_window` half a second after the last change, read by `ui::initial_window_bounds`, which falls back to the middle of the main display when the saved place is no longer on a screen). A title band (drag area, the rail color) above the content. `ui/src/theme.rs` holds the light and dark palettes (GitHub Primer, as in Vekro), applied to the Kit theme and following the system appearance; the interface text is 14 px (`FONT_SIZE`). Signed out: a fixed sidebar on the left (brand, language) and on the right a progress bar with the step counter ("Step 2 of 4") above the current step, and the Back/Next actions pinned at the bottom. The first guide step is a checklist of five things to do on Apple's site, each with a checkbox. The steps are the welcome screen, then the three-step sign-in guide (create the key on Apple's site, copy Issuer ID and Key ID, choose the `.p8` file); the welcome screen only shows on the first launch. Signed in, in the style of Vekro: the **activity bar** (56 px strip: the Apps button, one avatar button per app, and Settings at the bottom) and the main area. The Apps button shows every app as a grid of three columns that takes the whole width (`apps_view.rs`, no sidebar then). With an app open there is a **sidebar** (280 px): at the top the app (icon, name, bundle id; under them the version with where it stands at Apple, and the review score, which opens the Analysis page), under it the four pages of the app as a menu, and at the bottom Reload and Save (no Save on Analysis). While Settings is open the sidebar lists the pages of Settings instead. The page itself scrolls; pages are flat sections (a heading with a line under it), not boxes. An app has four pages: **App Information**, **App Review**, **App Privacy**, **Analysis**. Settings has the pages General (language), Account, AI provider (model and API key of each provider), Code folders (the folder with each app's source code, `Settings.code_folders`, chosen with the folder dialog), Rules and About (version, data folder).