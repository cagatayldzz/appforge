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
| `storage` | `settings.json`, per-app privacy answers and the Keychain (`SecretStore`). |
| `ui` | The window: sign-in, sidebar, app screens, Analysis and Settings. |

## Window layout

A title bar (drag area) above the content. Signed out: a fixed sidebar on the left (brand, language, Google Play as coming soon) and on the right a progress bar with the step counter ("Step 2 of 4") above the current step, and the Back/Next actions pinned at the bottom. The first guide step is a checklist of five things to do on Apple's site, each with a checkbox. The steps are the welcome screen, then the three-step sign-in guide (create the key on Apple's site, copy Issuer ID and Key ID, choose the `.p8` file); the welcome screen only shows on the first launch. Signed in: a Kit `Sidebar` with the app list, Google Play as coming soon, Settings and Sign out, and the main area. The main area keeps the app header (avatar, bundle id, version, state) and the tabs in place, scrolls the page between them in `GroupBox` sections, and pins a save bar at the bottom. An app has four tabs: **App Information**, **App Review**, **App Privacy**, **Analysis**. Settings holds the language, the account, the AI provider, model and API key, and About (version, data folder).

The interface language (English or Türkçe) is `Settings.language`. Source text is English and is the key of `ui::i18n` (`tr`, `trf`); a text without a Turkish entry is shown in English, and a test in `crates/ui/tests/i18n.rs` fails when a used text has no entry. Placeholders in text fields and the rules and AI findings are not translated.

## Data flow

Every network call and Keychain access is blocking, so the interface runs it with `cx.background_spawn` inside a task stored on the workspace and applies the result with `update_in`. Opening an app reads its snapshot (App Information, listing, screenshot sets, review details) in one go; saving sends only changed fields and then reads the snapshot again. Errors appear as notifications.

The Analysis recomputes from the saved snapshot and the privacy answers. The AI analysis sends the listing text, review notes and privacy answers together with the playbook text to the provider the user chose. It never sends the demo account password, the demo user name or the reviewer contact details.

## Files in the data directory

`~/Library/Application Support/AppForge` (or `$APPFORGE_DATA_DIR`):

- `settings.json`: the language, whether the welcome screen was done, Issuer ID and Key ID of the signed-in key, the AI provider and models, and the privacy answers per app. An unreadable file is never overwritten.
- `playbook/`: the cached playbook pages (`APPLE.md`, `PRE-SUBMISSION-CHECKLIST.md`, `MISTAKE-PATTERNS.md`) and `fetched-at`.

Secrets live in the Keychain (service `AppForge`): `apple-private-key`, `openai-api-key`, `anthropic-api-key`.

## Limits

The App Store Connect API has no endpoint for the App Privacy (nutrition label) answers, so they are edited in App Store Connect and mirrored by hand in AppForge for the analysis. A clean analysis is not a promise of approval: the rules see metadata, not the app binary.
