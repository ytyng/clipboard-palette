# Release (GitHub Actions build + signing + notarization + Homebrew cask)

A universal dmg for macOS is built on GitHub Actions, signed with Developer ID,
taken all the way through Apple notarization + staple, and published to a GitHub Release.
Notarization and stapling are applied to both the `.app` inside and the dmg.
`Casks/clipboard-palette.rb` in the Homebrew tap ([ytyng/homebrew-tap](https://github.com/ytyng/homebrew-tap))
is rewritten by a workflow on the tap side that checks the latest published Release every hour.
This repository does not push to the tap.

## Usage

```shell
pnpm release           # 0.1.0 -> 0.1.1 (patch, default)
pnpm release minor     # 0.1.0 -> 0.2.0
pnpm release major     # 0.1.0 -> 1.0.0
```

`scripts/release.sh` does the following.

1. Verifies the `main` branch, a clean working tree, and `HEAD == origin/main`
2. Assigns the new version in `src-tauri/tauri.conf.json` and applies the same version to `package.json`
   (pnpm-lock.yaml does not hold the package's own version, so it is not touched)
3. Commits `chore: release vX.Y.Z` and pushes it to `main`
4. Finds the run started by that push (by head SHA) and watches it until it completes

What starts a release is the push, not the script. Changing the version in `tauri.conf.json`
by hand and pushing does the same thing.

The version is applied by rewriting the relevant field of the 2 files directly (the whole JSON is not reformatted).

## Core idea: decide by "is that version already published", not "did the version change"

`release.yml` runs on every push to `main`, and the `plan` job calls
`GET /repos/{repo}/releases/tags/v<version>`.

- 404 → not published. `test` → `build` → `publish` run.
- 200 → already published. Nothing happens (a push that does not change the version finishes with a checkout and one API call).
- Anything else (rate limit / outage) → undeterminable, so it **fails**. Reading it as unpublished would rebuild and
  try to publish an already published version.

Since it does not look at the diff, the result is the same whether the change lands via squash, rebase, or a direct push (idempotent).
It is not narrowed with `paths: [src-tauri/tauri.conf.json]` for the same reason: so that a release that failed because of a build or workflow
bug can be retried by "fix the cause and push"
(that fix does not touch the version). **Do not bump the version again after a failure**.

`workflow_dispatch` is for rerunning when "the version is right but the run failed due to a transient outage".
It goes through the same check, so it cannot publish an already published version again. Dispatches from refs other than `main`
are rejected at the start of plan (so that a release is not made from the contents of a branch carrying an unpublished version).

## Design decisions (why it is this way)

- **build runs the tauri CLI directly, and notarizing and uploading the dmg are separate steps**.
  When `APPLE_*` is passed, the tauri CLI signs + notarizes + staples the `.app`, but only
  signs the dmg. Without notarizing the dmg, a downloaded dmg is
  `rejected / source=Unnotarized Developer ID` by
  `spctl -a -t open --context context:primary-signature` (this was the state up to v0.3.0).
  tauri-action does the build and upload in a single step and cannot insert dmg notarization
  in between, so it is not used. The dmg goes through `notarytool submit --wait` (judged by both the exit code and
  `status: Accepted`; on failure `notarytool log` is printed) →
  `stapler staple` → `stapler validate` → `spctl --type open`, in that order.
- **Two stages: draft → publish**. The last step of build creates the `v<version>`
  Release as a draft (an existing draft is reused and retargeted to this run's commit),
  uploads the dmg by ID, and after all builds succeed, the `publish` job
  publishes it with `gh release edit --draft=false`. If a Windows leg is added to the matrix in the future,
  this prevents publishing an incomplete Release where only one side succeeded. On rerun,
  assets with the same name are deleted before uploading again. If a Release for the same tag is already published, it does nothing and fails.
- **Only test runs on PRs**. `plan` is skipped on `pull_request`, and `build` /
  `publish` are skipped along with it. `test` removes the automatic skip with `!cancelled()`, and
  runs only for PRs or for "a version to be released" (so that a push to main that does not change the version
  does not spin up a macOS runner).
- **`test` cross-checks the major.minor of the Tauri crates and npm packages**
  (`scripts/check-tauri-versions.mjs`). `tauri build` stops before building when these diverge,
  but the PR `test` does not run `tauri build`, so a dependency update on only one side
  would first fail in the release build (v0.1.2 was not published because of this). When upgrading
  dependencies, upgrade `tauri` ↔ `@tauri-apps/api` and `tauri-plugin-*` ↔ `@tauri-apps/plugin-*`
  together.
- **The command name is `release`**. `publish` collides with a built-in npm/pnpm command.
- **`pnpm exec tauri build`** calls the CLI directly (not `pnpm tauri build`).
  The moment an inline assignment like `APPLE_SIGNING_IDENTITY='...' tauri` is added to the `tauri` script in
  `package.json`, the env passed from the workflow would be silently overridden
  (an inline shell assignment beats the inherited env). Calling the CLI directly makes the Secret the single source of truth.
- **`concurrency` is `cancel-in-progress: false` + `queue: max`**. 1 push =
  1 version, so if a run is cancelled, publishing that version is delayed
  (the bump commit stays on main). The default `queue: single` keeps only one pending run,
  and a new push wipes out the existing pending one, so `queue: max` is needed.
  PR runs go into a separate group per commit so they do not queue behind releases.
- **Every `uses:` is pinned to a commit SHA**. This job handles a certificate containing Apple's private key,
  so if a tag were swapped, the secrets could be stolen. When updating, look up the new SHA using
  the trailing `# v4` comment as a guide. For `dtolnay/rust-toolchain`, pin a SHA from the **master history**
  (the SHA at the tip of the `stable` branch will be GC'd in the future and the run will fail).
- **`persist-credentials: false`**. Do not leave a write-capable `GITHUB_TOKEN` in `.git/config`
  (it could be picked up by `pnpm install` install scripts or third-party actions).
- **The frontend build is separated into a step without Secrets**. `tauri build` launches
  `beforeBuildCommand` (`pnpm build`) as a child process, and child processes inherit
  environment variables. Without the separation, vite and its dependencies would run in an environment where they can read
  `APPLE_PASSWORD`, and a single malicious dependency would be enough to exfiltrate it.
  The signing build overlays `src-tauri/tauri.ci.conf.json` with `--config` to empty
  `beforeBuildCommand` and avoid building twice.
- **Local builds stay ad-hoc signed**. Leaving `signingIdentity: "-"` in `tauri.conf.json`
  means a local build without the env is ad-hoc signed, while in CI `APPLE_SIGNING_IDENTITY`
  overrides the config and signs with Developer ID (tauri-cli precedence: env > config).
- **The Homebrew cask is updated from the tap side (the project does not push)**.
  If the project pushed to the tap, a token that can write to the tap would have to be
  distributed to every project. If the tap pulls instead, the `GITHUB_TOKEN` that Actions has
  for its own repository is enough, and there are zero new secrets. The only cost is the delay until the next hourly run. The cask includes a `binary` stanza, so `brew install` alone links the executable into
  Homebrew's bin (a manual `ln -s` is no longer needed).

## Required Repository Secrets

The following 6 in `ytyng/clipboard-palette`.

| Secret | Contents |
| --- | --- |
| `APPLE_CERTIFICATE` | The `.p12` of the Developer ID Application certificate + private key, base64-encoded |
| `APPLE_CERTIFICATE_PASSWORD` | Password of the `.p12` |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: <Name> (<TeamID>)` |
| `APPLE_ID` | Email address of the Apple account |
| `APPLE_PASSWORD` | App-specific password (the regular password does not work) |
| `APPLE_TEAM_ID` | 10-character Team ID |

The workflow first checks that all 6 APPLE_* secrets are present and fails immediately if any are missing.
Without this, "a dmg that is neither signed nor notarized" would be published as a success.
The tap token (`HOMEBREW_TAP_TOKEN`) is no longer used.

## Verification after publishing

Download the dmg from the Release and check it on a real machine.

```shell
DMG=Clipboard-palette_X.Y.Z_universal.dmg
spctl -a -vv -t open --context context:primary-signature "$DMG"   # accepted / source=Notarized Developer ID
xcrun stapler validate "$DMG"        # The validate action worked!
hdiutil attach -nobrowse -quiet "$DMG"
APP=/Volumes/Clipboard-palette/Clipboard-palette.app
codesign -dv --verbose=2 "$APP"      # Authority=Developer ID Application: ... / flags=...runtime
spctl -a -vvv "$APP"                 # accepted / source=Notarized Developer ID
xcrun stapler validate "$APP"        # The validate action worked!
lipo -archs "$APP/Contents/MacOS/clipboard-palette"   # x86_64 arm64
hdiutil detach -quiet /Volumes/Clipboard-palette
```

On the Homebrew side, (after the tap's next hourly run succeeds) check that the cask's version / sha256 match the new
Release. If you do not want to wait, trigger the tap's workflow manually.

```shell
gh workflow run update.yml -R ytyng/homebrew-tap
gh api repos/ytyng/homebrew-tap/contents/Casks/clipboard-palette.rb -q .content | base64 -d
brew install --cask ytyng/tap/clipboard-palette
clipboard-palette --help
```

If `source=Notarized Developer ID` and a successful staple appear, users will not see a Gatekeeper warning
when they download and open it. "It is signed" alone does not confirm that
`APPLE_SIGNING_IDENTITY` took effect, so check all the way to `Authority=` matching the identity
stored in the Secret.

## Known weaknesses

- **Merging a PR that happens to include a version change publishes immediately**. Change the version with
  `pnpm release` (a separate commit) and do not mix it into feature PRs.
- **Rust build scripts can still see the secrets**. The frontend
  build is separated, but `tauri build` runs the cargo build as one unit with signing and notarization,
  so the `build.rs` of Rust dependency crates runs in an environment that has `APPLE_PASSWORD`
  (`GITHUB_TOKEN` is not passed to the Build step). Closing this completely would require breaking it down into
  "build unsigned → manually run codesign + notarytool + stapler on the `.app` in a separate step".
  Cargo dependencies are pinned by `Cargo.lock`, so for now
  this risk is accepted.
- `pnpm release` **pushes directly** to `main`. It breaks if branch protection (PRs required)
  is enabled. If you enable it, switch to submitting the version bump as a PR (the workflow side
  works as-is).
- Windows builds are not included. If needed, add a `windows-latest` / `--bundles nsis` leg to the matrix of
  `release.yml` (APPLE_* is already limited to macOS by the
  `matrix.platform == 'macos-latest'` condition).
  The dmg notarization and draft upload steps are also macOS-only, so a separate step is needed
  to upload the Windows artifacts to the draft.
