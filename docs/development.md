# Development

## Platform

Supported and tested: macOS 15+ on Apple Silicon. Intel, Linux and Windows are unsupported. The minimum
system version is `LSMinimumSystemVersion` in `apps/appforge/macos/Info.plist` (`scripts/app.sh` links
against it).

Requirements: Command Line Tools (not Xcode), Rust 1.92+ (stable) and Bun. After cloning run `bun install`.

## Commands

| Command | Does |
| --- | --- |
| `bun run dev` | debug build, launch `dist/AppForge.app` |
| `bun run build` | release build → `dist/AppForge.app` |
| `bun run check` | script syntax and shell tests, `cargo fmt --check`, clippy with warnings as errors, tests (run before finishing a change) |
| `bun run update` | update the Rust dependencies in `Cargo.lock` to the newest versions `Cargo.toml` allows, then clippy and tests; `Cargo.lock` is put back if they fail |
| `bun run test` | shell tests, then `cargo test --locked --workspace` |
| `bun run release` | request a release: dispatches the Release workflow for the pushed default branch (needs `gh`) |
| `bun run icon` | redraw `AppIcon.icns` and `DevIcon.icns` from `apps/appforge/macos/icon.svg` (needs Google Chrome) |
| `bun run commit` | `git add -A && oco --yes` (needs [OpenCommit](https://github.com/di-sukharev/opencommit)) |

## Scripts

Automation uses only shell and Rust; there is no TypeScript or other runtime in scripts or workflows.

| Path | Purpose |
| --- | --- |
| `scripts/app.sh` | build, bundle, sign, launch, check (`--dev`, `[debug\|release] --build`, `--check`, `--update`, `--icon`) |
| `scripts/release.sh` | request a release; in CI: `prepare` (next version, stamping), `notes`, `package`, `finalize` (assets, checksums) |
| `scripts/notarize-app.sh` | sign with Developer ID and notarize `dist/AppForge.app` (macOS tools, release secrets) |
| `scripts/tests/*.sh` | shell tests, run by `scripts/tests/run.sh` (version, notes, packaging, workflow guarantees) |

## Releases

`bun run release` (or running the Release workflow by hand) releases the pushed default branch. The version comes
from Conventional Commits since the latest `v*` tag: a breaking change (`!:` or `BREAKING CHANGE:`) is major,
`feat` is minor, everything else is patch. The workflow then:

1. **Validate**: Apple secrets are present, shell tests, `cargo fmt`, clippy and tests of `appforge-ai`, `appforge-appstore`, `appforge-compliance`, `appforge-project`, `appforge-storage`.
2. **Prepare**: choose the version, stamp `Cargo.toml`, `Cargo.lock`, `package.json` and `Info.plist` in the CI checkout, write `RELEASE_NOTES.md` (commit titles grouped as New Features, Bug Fixes, Documentation, Chores, with a changelog link).
3. **Build** (macOS, in parallel): workspace clippy and tests; release build, Developer ID signature, notarization, `AppForge-<tag>-macos-arm64.zip`.
4. **Verify**: exactly the expected assets, none empty, `SHA256SUMS.txt`.
5. **Publish**: checksums verified, draft GitHub release, then published as latest.

### First release setup

Add these repository secrets in **Settings → Secrets and variables → Actions** on GitHub.

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE_BASE64` | Base64 of the Developer ID Application certificate and private key exported together as a `.p12`. On macOS, run `base64 -i /path/to/certificate.p12` and save the output as the secret. |
| `APPLE_CERTIFICATE_PASSWORD` | Password set when exporting the `.p12`. |
| `APPLE_ID` | Apple Account email used for notarization. |
| `APPLE_TEAM_ID` | Apple Developer Team ID matching the certificate. |
| `APPLE_APP_SPECIFIC_PASSWORD` | App-specific password for that Apple Account. |

With the release files on the default branch, open **Actions → Release → Run workflow**. From a clean local
checkout that matches the pushed default branch, you can also run `bun run release` with an authenticated GitHub CLI.
Download the signed ZIP and `SHA256SUMS.txt` from **Releases** once the workflow finishes.

The version in the repository is stamped only in CI; the tag is the source of truth.
`CI` (`.github/workflows/ci.yml`) runs the shell tests, formatting, and clippy and tests of the plain-Rust crates (the ones without the interface) on Linux.

## Packages

| Package | Kind | Purpose |
| --- | --- | --- |
| `appforge` | app | executable entry point (binary `AppForge`) |
| `appforge-app` | lib | startup, window, key bindings |
| `appforge-ui` | lib | views built with GPUI Kit |
| `appforge-compliance` | lib | review rules, playbook, AI analysis, AI-written listing |
| `appforge-project` | lib | short description of an app's source folder for the AI |
| `appforge-appstore` | lib | App Store Connect API client, app icons |
| `appforge-ai` | lib | OpenAI and Claude clients, the low-cost models and their prices |
| `appforge-storage` | lib | settings, icon cache, Keychain secrets |

Code map and data flow: [architecture](architecture.md).

## Tests

Tests live in `crates/<crate>/tests/`, never inline. API clients are tested against a local HTTP server on a random port; no test reaches Apple, OpenAI or Anthropic, and none touches the Keychain (`MemorySecrets`). Tests that touch the data directory use temporary directories; `APPFORGE_DATA_DIR` overrides the location. `crates/appstore/tests/test_key.p8` is a throwaway P-256 key for token tests, not a credential.

Shell tests for the scripts and workflows live in `scripts/tests/`. The interface has no automated tests; check a change by running `bun run dev`.

## Visual inspection

The interface has no automated tests; a change is checked by running it. Run `bun run dev` with a scratch data directory and take a screenshot of the AppForge window only. Screenshots can contain private paths or terminal history, so keep them in a scratch location and do not commit them.

## Running the app in isolation

Use a scratch data directory so your own settings stay out:

```sh
mkdir -p /tmp/appforge-data
echo '{"onboarded": true, "language": "english"}' > /tmp/appforge-data/settings.json
APPFORGE_DATA_DIR=/tmp/appforge-data ./target/debug/appforge
```

A signed-out start shows the welcome screen and the sign-in guide; the signed-in screens need a real App Store Connect account. `./target/debug/appforge` is the plain binary: it reads no Keychain until a key was saved for the account in `settings.json`.

## Keychain prompts in development

`bun run dev` signs the app after every build. An ad-hoc signature is new on every build, so macOS asks for the login password again each time the app reads the Keychain. To stop that, create a local code-signing certificate once and the script uses it:

1. Keychain Access > Certificate Assistant > Create a Certificate: name `AppForge Dev`, identity type Self Signed Root, certificate type Code Signing.
2. Run `bun run dev` and choose **Always Allow** once.

Set `APPFORGE_SIGN_IDENTITY` to use a certificate with another name. Without the certificate the build still works and prints a note.

## Dependencies

`bun run update` moves the Rust dependencies to the newest versions `Cargo.toml` allows (only `Cargo.lock` changes), then runs clippy and the tests, and puts the old `Cargo.lock` back if they fail. A version beyond what `Cargo.toml` allows (a new `gpui-kit` minor, for example) is raised by hand in `Cargo.toml`.

## README images

The images in `docs/images` show made-up demo apps ("Pocket Garden"), so that no real app name, account, machine name, path or terminal history appears. They were made by filling a window with demo apps through a throwaway example (not kept in the repository), with a scratch `APPFORGE_DATA_DIR`, then capturing only the window without its shadow (`screencapture -o -l <window id>`) and scaling it to 1600 px wide. Check every image for private details before committing it.

## Formatting

2 spaces, LF, final newline, `max_width = 150`. `rustfmt.toml` and `.editorconfig` hold the rules; run
`cargo fmt --all`.
