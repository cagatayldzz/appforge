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

The private key is kept in the macOS Keychain; only the Issuer ID and Key ID are written to `settings.json`. **Sign out** removes the key from the Keychain.

## Finding your way

On the far left is a narrow bar:

- **Apps** (the top button) shows every app as a grid of three columns across the whole window. Click a card to open the app.
- Under it, one button per app, with its icon (the icon comes with the app's latest build in App Store Connect; an app with no build yet shows its first letter).
- **Settings** at the bottom.

With an app open, the column next to the bar shows the app (icon, name, bundle id, version and where it stands at Apple, and the **score**, which opens the Analysis page), its six pages as a menu, and **Reload** and **Save** at the bottom.

## Your apps

| Page | Shows and edits |
| --- | --- |
| App Information | Name and subtitle, and the listing of the version in preparation: description, keywords, promotional text, support and marketing URLs, What's New. Counters show the limits (name and subtitle 30, keywords 100, promotional text 170). A language picker appears when the app has several languages. |
| Distribution | iOS version history, compatible uploaded builds, review submissions and release settings. |
| App Review | Contact, "Sign-in required" with the demo account, and the notes for the reviewer. Saving creates the review details when there are none yet. |
| App Privacy | The privacy policy URL (saved to App Store Connect) and your own answers: collects data, tracking, App Tracking Transparency, accounts and account deletion, a reviewer sign-in, data types and third-party SDKs. The answers stay on this Mac. |
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

On the App Information page, the **Write with AI** button sits above **Reload** and **Save** in the sidebar footer. It writes the subtitle, description, keywords, promotional text and what's new, from what the app's source folder says about it. It does not change the app name or the URLs.

1. **No API key yet?** A notice says the AI is not connected and has an **Open Settings** button that goes to the AI provider page.
2. **No source folder yet?** A dialog asks you to choose the folder that holds the app's source code; once you do, the writing starts. You can also set or change the folders in Settings > Code folders.
3. **Writing.** The result fills the fields. **Nothing is saved**: read it, change what you like, then press **Save**. A field the AI had nothing to add to keeps its text. A notice says how many tokens it used and about what it cost.

What is sent to the provider you chose: the current listing texts, and a short description of the folder: its shape (top levels), the README, the project files that name the app and what it uses (for example `Package.swift`, `Info.plist`, `package.json`), and the texts the app shows (such as `Localizable.strings`), about 10,000 tokens at most. Source code itself is not sent. Files that hold secrets (`.env`, keys, signing files, `GoogleService-Info.plist`) and links that leave the folder are never read.

## Analysis

The built-in rules check the saved data (not unsaved edits): limits, placeholder text, support and privacy URLs, screenshots, review contact, a demo account when a sign-in is needed, and your privacy answers. Each finding names the App Store Review Guideline, what is wrong and how to fix it, with a button that opens the right page. Critical findings cost 20 points, warnings 7 and tips 1. A high score is not a promise of approval: the rules see the listing, not the app itself.

**Analyze with Claude / OpenAI** asks the model to read the listing, review notes and privacy answers against the [App Store Compliance Playbook](https://github.com/mjmirza/app-store-compliance) and to add what the built-in rules miss. It sends the text to the provider you chose; the demo account name and password and the reviewer's contact details are never sent. The playbook is downloaded from GitHub on first use and refreshed weekly (**Update rules** does it now).

## Settings

| Page | Holds |
| --- | --- |
| General | The language of AppForge. The listing you edit keeps its own languages. |
| Account | Which key you are signed in with, and **Sign out**. |
| AI provider | For Claude and OpenAI: a menu of the models, with what a million input and output tokens cost, the API key (kept in the Keychain), and which provider is in use. |
| Code folders | For each app, the folder that holds its source code (**Choose folder**, **Change**, **Remove**). A folder that was moved or deleted is shown in red. |
| Rules | The playbook the analysis reads. |
| About | The version and a button that shows the data folder. |

Only low-cost models are offered: Claude Haiku 5.5 and 4.5, and GPT-4.1 nano, GPT-5 nano, GPT-6 Luna and GPT-4.1 mini. The prices are the providers' list prices when they were last checked and can change, so check the provider's price page. A model you had chosen earlier that is no longer offered is replaced by the default.

## Where things are kept

In `~/Library/Application Support/AppForge` (or `APPFORGE_DATA_DIR`): `settings.json` (language, account ids, AI choices, privacy answers, code folders, the window), `icons/` (app icons, refreshed daily) and `playbook/`. Secrets are only in the Keychain.
