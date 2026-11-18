# AppForge

AppForge is a native macOS app for managing your App Store apps and checking them against Apple's review rules before you submit. It is built with Rust and GPUI Kit using `gpui-fast`.

- **Sign in** to App Store Connect with an API key (Issuer ID, Key ID and the `.p8` file). Apple offers no password sign-in for tools. The key is kept in the macOS Keychain. Google Play is planned.
- **Your apps**: every app of the account is listed; open one to see and edit **App Information** (name, subtitle, store listing), **App Review** (contact, demo account, notes) and **App Privacy** (privacy policy URL and your privacy answers). Changes are sent to App Store Connect.
- **Analysis**: built-in rules plus an AI review (OpenAI or Claude, with your own API key) read the saved data against the [App Store Compliance Playbook](https://github.com/mjmirza/app-store-compliance) and show what could get the app rejected. The playbook is downloaded from GitHub and refreshed weekly.

Apple's API does not expose the App Privacy (nutrition label) answers; AppForge links to that page in App Store Connect and keeps your answers locally for the analysis.

## Development

Requires macOS 15 or later, Rust 1.92 or later, and Bun.

```sh
bun run dev
bun run check
```

Settings and the cached playbook are saved in `~/Library/Application Support/AppForge`. Set `APPFORGE_DATA_DIR` to use another directory. Secrets (the `.p8` key, AI API keys) are saved in the Keychain under the service `AppForge`.
