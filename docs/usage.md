# Usage

## First launch

The welcome screen explains what AppForge does and lets you pick the language (English or Türkçe). **Get started** opens the sign-in guide. You can change the language later in Settings > General.

The window opens the way you left it: its size, its place, and whether it was maximized or full screen. If the place is no longer on a screen (a display was unplugged) it opens in the middle of the main one.

## Sign in

AppForge signs in with an App Store Connect API key, because Apple offers no password sign-in for tools.

The sign-in screen is a three-step guide, with a progress bar on top and the Back and Next buttons always at the bottom:

1. **Create key**: a checklist of five things to do on Apple's site. **Open App Store Connect** opens the page; the drawings show where Apple puts things (they are pictures of Apple's page, not buttons). Tick each step as you finish it. Create the key with the **App Manager** role and download the `.p8` file (Apple lets you download it once). If you already have a key file, go on.
2. **Copy IDs**: enter the **Issuer ID** (shown above the key list) and the **Key ID** (in the key table).
3. **Sign in**: choose the `.p8` file and press **Sign in**. A file named `AuthKey_<KEY ID>.p8` fills in the Key ID for you.

The private key is kept in the macOS Keychain; only the Issuer ID and Key ID are written to `settings.json`. **Sign out** removes the key from the Keychain. If App Store Connect rejects an active session with HTTP 401, AppForge returns to sign-in with the previous IDs filled in and asks for a valid key file.

## Finding your way

On the far left is a narrow bar:

- **Apps** (the top button) shows every app as a grid of three columns across the whole window. Click a card to open the app.
- Under it, one button per app, with its icon (the icon comes with the app's latest build in App Store Connect; an app with no build yet shows its first letter).
- **Settings** at the bottom.

With an app open, the column next to the bar shows the app (icon, name, bundle id, version and where it stands at Apple, and the **score**, which opens the Analysis page), its six pages as a menu, and **Reload** and **Save** at the bottom.

## Your apps

AppForge keeps loaded pages for each app during the signed-in session. Switching tabs or returning to an app reuses that data. **Reload** updates the open page, and existing data stays visible while Apple responds.

| Page | Shows and edits |
| --- | --- |
| App Information | Name and subtitle, and the listing of the version in preparation: description, keywords, promotional text, support and marketing URLs, What's New. Counters show the limits (name and subtitle 30, keywords 100, promotional text 170). A language picker appears when the app has several languages. |
| Distribution | iOS version history, compatible uploaded builds, review submissions and release settings. |
| App Review | Contact, "Sign-in required" with the demo account, and the notes for the reviewer. Saving creates the review details when there are none yet. |
| App Privacy | The privacy policy URL (saved to App Store Connect) and your own answers: collects data, tracking, App Tracking Transparency, accounts and account deletion, a reviewer sign-in, data types and third-party SDKs. The answers stay on this Mac after you confirm Save. |
| Ratings and Reviews | Customer reviews, ratings and public developer replies. Publishing and deleting a reply require confirmation. |
| Analysis | The readiness score, the findings and the AI analysis. |

**Save** sends only the fields you changed and then reads the app again. Apple only accepts edits while a version is in preparation or was rejected; otherwise it refuses and AppForge shows Apple's message.

App Store Connect's API cannot read or write the App Privacy (nutrition label) answers. The **Open App Privacy in App Store Connect** button opens that page; tick "I answered App Privacy" when it is done.

## Distribute an iOS version

1. Open **Distribution** and create a version if there is no editable version.
2. In **Builds**, select a valid build that has already been uploaded to Apple.
3. Choose manual release, release after approval, or a scheduled release. Enter a scheduled date and time in ISO 8601 format, such as `2027-01-31T12:00:00Z` for UTC. Eligible updates can use phased release.
4. Add the version to a review draft, then choose **Submit for review**. Use **Reload** to check its status. A version approved for manual release has a **Release this version** action.

## Write with AI

Use the AI icon inside a listing field on **App Information**. The confirmation shows the selected field, app, language, and model. **Generate** requests one draft for the selected field.

If the provider key or source folder is missing, AppForge helps you configure it before generating. Review the draft and press **Save** to send your edits to App Store Connect. AppForge skips the draft if you change the selected app, language, or field while the request is running.

What is sent to the provider you chose: the current listing texts, and a short description of the folder: its shape (top levels), the README, the project files that name the app and what it uses (for example `Package.swift`, `Info.plist`, `package.json`), and the texts the app shows (such as `Localizable.strings`), about 10,000 tokens at most. Source code itself is not sent. Files that hold secrets (`.env`, keys, signing files, `GoogleService-Info.plist`) and links that leave the folder are never read.

## Analysis

The built-in rules check the saved data (not unsaved edits): limits, placeholder text, support and privacy URLs, screenshots, review contact, a demo account when a sign-in is needed, and your privacy answers. Each finding names the App Store Review Guideline, what is wrong and how to fix it, with a button that opens the right page. Critical findings cost 20 points, warnings 7 and tips 1. A high score is not a promise of approval: the rules see the listing, not the app itself.

**Analyze with Claude / OpenAI** asks the model to read the listing, review notes and privacy answers against the [App Store Compliance Playbook](https://github.com/mjmirza/app-store-compliance) and to add what the built-in rules miss. It sends the text to the provider you chose; the demo account name and password and the reviewer's contact details are never sent. The playbook is downloaded from GitHub on first use and refreshed weekly (**Update rules** does it now).

## Apple App Review Guidelines

AppForge checks Apple’s guidelines in the background at startup if the last successful check was at least 24 hours ago. **Update rules** runs the check on demand. If Apple’s page is unavailable or cannot be parsed, AppForge keeps the last valid catalog. Findings link to the official rules where a matching number exists, and AI analysis receives a limited set of relevant Apple excerpts alongside the community playbook.

## Settings

| Page | Holds |
| --- | --- |
| General | The language of AppForge. The listing you edit keeps its own languages. |
| Account | Which key you are signed in with, and **Sign out**. |
| AI provider | For Claude and OpenAI: a menu of the models, with what a million input and output tokens cost, the API key (kept in the Keychain), and which provider is in use. |
| Code folders | For each app, the folder that holds its source code (**Choose folder**, **Change**, **Remove**). A folder that was moved or deleted is shown in red. |
| Rules | Search the Apple App Review Guidelines by number or topic, open the official rule, and see the latest additions, changes and removals. |
| About | The version, GitHub release updates and a button that shows the data folder. |

AppForge checks GitHub for a newer stable release on launch. In **Settings → About**, you can turn off launch checks, check manually, or read the release notes. Choose **Download update** to verify and stage the release, then **Restart to update** to install it. This requires a signed Apple Silicon release in a writable folder. AppForge verifies the checksum, macOS code signature, Gatekeeper assessment, developer team, bundle ID and version before offering to restart.

Only low-cost models are offered: Claude Haiku 5.5 and 4.5, and GPT-6 Luna. The prices are the providers' list prices when they were last checked and can change, so check the provider's price page. A model you had chosen earlier that is no longer offered is replaced by the default.

## Where things are kept

In `~/Library/Application Support/AppForge` (or `APPFORGE_DATA_DIR`): `settings.json` (language, account ids, AI choices, privacy answers, code folders, the window), `icons/` (app icons, refreshed daily) and `playbook/`. Secrets are only in the Keychain.
