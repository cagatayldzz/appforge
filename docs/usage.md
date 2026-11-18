# Usage

## Sign in

AppForge signs in with an App Store Connect API key, because Apple offers no password sign-in for tools.

1. In App Store Connect open Users and Access > Integrations and create a key with the **App Manager** role.
2. Download the `.p8` file (Apple lets you download it once).
3. In AppForge enter the **Issuer ID** (shown above the key list) and the **Key ID**, choose the `.p8` file and press **Sign in**. A file named `AuthKey_<KEY ID>.p8` fills in the Key ID for you.

The private key is kept in the macOS Keychain; only the Issuer ID and Key ID are written to `settings.json`. **Sign out** removes the key from the Keychain. Google Play is planned and shown as "Coming soon".

## Your apps

The sidebar lists every app of the account. Pick one to open it. The page has four tabs and a language picker when the app has several languages.

| Tab | Shows and edits |
| --- | --- |
| App Information | Name and subtitle, and the listing of the version in preparation: description, keywords, promotional text, support and marketing URLs, What's New. Counters show the limits (name and subtitle 30, keywords 100, promotional text 170). |
| App Review | Contact, "Sign-in required" with the demo account, and the notes for the reviewer. Saving creates the review details when there are none yet. |
| App Privacy | The privacy policy URL (saved to App Store Connect) and your own answers: collects data, tracking, App Tracking Transparency, accounts and account deletion, a reviewer sign-in, data types and third-party SDKs. The answers stay on this Mac. |
| Analysis | The readiness score, the findings and the AI analysis. |

Saving sends only the fields you changed and then reads the app again. Apple only accepts edits while a version is in preparation or was rejected; otherwise it refuses and AppForge shows Apple's message.

App Store Connect's API cannot read or write the App Privacy (nutrition label) answers. The **Open App Privacy in App Store Connect** button opens that page; tick "I answered App Privacy" when it is done.

## Analysis

The built-in rules check the saved data (not unsaved edits): limits, placeholder text, support and privacy URLs, screenshots, review contact, a demo account when a sign-in is needed, and your privacy answers. Each finding names the App Store Review Guideline, what is wrong and how to fix it, with a button that opens the right tab. Critical findings cost 20 points, warnings 7 and tips 1. A high score is not a promise of approval: the rules see the listing, not the app itself.

**Analyze with Claude / OpenAI** asks the model to read the listing, review notes and privacy answers against the [App Store Compliance Playbook](https://github.com/mjmirza/app-store-compliance) and to add what the built-in rules miss. It sends the text to the provider you chose; the demo account name and password and the reviewer's contact details are never sent. The playbook is downloaded from GitHub on first use and refreshed weekly (**Update rules** does it now).

## Settings

Choose the AI provider (Claude or OpenAI), set the model and paste your API key. Keys go to the Keychain. More providers can be added later.
