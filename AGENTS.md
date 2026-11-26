# clipboard-palette project details

## Overview

A Tauri application that receives text from standard input and displays buttons for copying it to the clipboard.

## Frameworks and tech stack

- **Tauri2**: Desktop application framework
- **Svelte5**: Frontend framework  
- **Tailwind4**: CSS framework
- **Rust**: Backend language
- **clap**: Command-line argument parser for Rust

## Project structure

```
clipboard-palette/
├── src/                    # Svelte frontend
│   ├── routes/
│   │   ├── +page.svelte   # Main page
│   │   ├── Help.svelte    # Help component
│   │   ├── TextCard.svelte # Text card component
│   │   └── licenses/+page.svelte # Third-Party Licenses window
│   └── app.html
├── src-tauri/              # Tauri backend
│   ├── src/
│   │   ├── lib.rs         # Main logic
│   │   ├── notices.rs     # --license / Third-Party Licenses (embeds THIRD-PARTY-NOTICES.txt)
│   │   ├── open_target.rs # Open mode (Shift + click opens a URL / path)
│   │   └── main.rs        # Entry point
│   └── Cargo.toml         # Rust dependencies
├── tests/                  # Test scripts
└── build/                  # Build output
```

## Main features

### Command-line argument handling

The following options are supported using the `clap` library:

- `--multiline` / `-m`: Split on newlines
- `--split-empty-line[=N]` / `-s [N]`: Split on runs of N or more empty lines (default 1)
- `--json` / `-j`: Parse as JSON
- `--theme=auto|light|dark`: Specify the color theme (default auto = follow the OS setting). Applied to both the window contents and the title bar
- `--input <FILE>` / `-i`: Read from a file instead of standard input. An empty file is an error (it does not fall back to sample data).
  Claude Code runs invocations containing redirects, pipes, or `$(...)` inside the sandbox even when they match excludedCommands,
  so to launch the app outside the sandbox the file has to be passed as an argument
- `--license`: Print the app's own MIT license and `THIRD-PARTY-NOTICES.txt` to stdout and exit (does not read stdin and does not show a window)

### Data processing modes

1. **normal**: Display the text as-is
2. **multiline**: Split on newlines and turn each line into a button
3. **split-empty-line**: Split into sections on the specified number of empty lines
4. **json**: Parse a JSON array and produce label/text pairs

### Theme control

- Tailwind's `dark:` variant is changed with `@custom-variant` to be based on `html[data-theme="dark"]` (src/app.css)
- The window is set to `"create": false` in tauri.conf.json and assembled in the Rust setup with `WebviewWindowBuilder::from_config`. This is so that `initialization_script` can inject the `--theme` value (`window.__CLIPBOARD_PALETTE_THEME__`) into the page
- The inline script in src/app.html decides `data-theme` from the injected value, then the OS setting. It is decided before the first paint, so there is no flicker
- After the startup data is fetched, `applyTheme()` in `src/lib/theme.ts` is called. With `auto`, it watches `matchMedia` and also follows OS-side switches
- The title bar is set with `WebviewWindow::set_theme()` (on macOS this applies to the whole app)

### Open mode (src-tauri/src/open_target.rs)

While Shift is held, cards whose text is a URL or a file path are underlined, and clicking them
opens them instead of copying (URLs in the default browser; paths are shown selected in Finder).

- Classification (`classify`) is done only on the Rust side and passed to the frontend as `ClipboardItem.open_kind`
  (`"url"` / `"path"` / null). Writing `open_kind` in `--json` input is ignored (`skip_deserializing`)
- **The frontend sends only the item's index.** `open_item` re-reads the text from state,
  classifies it again, and then opens it. This is so the web view cannot make the app open an arbitrary string
- Only http / https URLs. `file:` and custom schemes can launch apps, so they are not opened.
  A host must be written right after `//` (`https://?q` / `https:///path` are not treated as URLs)
- Only single-line text is eligible. Relative paths are resolved against the process cwd (the directory of the shell that launched it),
  and `~` is expanded to the home directory. Windows-style paths (`C:\` / UNC) are treated as paths only on Windows
  (on macOS they would become relative paths and open an unrelated folder under cwd)
- **Paths are always revealed (`reveal_item_in_dir`); `open_path` is never used.** An `.app` bundle
  is a directory, so opening it would launch the app. For a nonexistent path, the nearest existing ancestor is revealed
- Opening is done by calling `tauri-plugin-opener` functions directly from Rust (the plugin is not registered
  and nothing is imported on the JS side, so no opener permission needs to be added to the capability)
- `+page.svelte` tracks the Shift state with `shiftKey` of keydown / keyup / mousemove, and clears it on window
  blur. The branch on click is decided by the `shiftKey` of the click event itself

### Lifecycle log (src-tauri/src/lifecycle.rs)

A log that lets the caller (a shell script or an AI agent) determine whether startup succeeded
**from stdout alone**. Every line starts with `[lifecycle]`.

- Right after startup, it prints the pid and other processes running under the same program name. This is where
  you can spot orphans left over from a previous run. If detection fails, it prints `unknown (reason)`
  rather than `none` (so a failure is not read as "there are no others")
- The window's visibility is measured when the event loop reaches `RunEvent::Ready`, and a conclusion is printed as
  either `startup ok: window is visible` or `startup incomplete: ...`.
  The caller only needs to look at this one line. A minimized window is also reported as `startup incomplete`
  (the visible flag is set, but it is not on screen). If either visibility or minimization could not be
  obtained, it also falls back to `startup incomplete` — the log exists to avoid a false "it started",
  so an unconfirmed state must not be reported as ok
- Exit reasons are printed through three paths: `RunEvent` (`ExitRequested` / `Exit` / `WindowEvent`), signals
  (SIGINT / SIGTERM / SIGHUP via `libc::signal`), and panics (panic hook).
  SIGKILL cannot be caught, so ending without a single `exit:` line means it was force-killed
- The window is built before `RunEvent`, so a failure is printed inside `setup` as
  `startup failed:`. Panics only go to stderr, so the panic hook also prints
  `exit: reason=panic` to stdout
- **Startup timeout (`--startup-timeout`, default 10 seconds, 0 disables it)**: `page loaded` is printed when the main page
  calls `get_clipboard_data` (it means the IPC arrived; it does not guarantee
  visibility. Success is judged by both this and `startup ok`). The watchdog starts after stdin is read
  and before Tauri initialization (so it also catches a hang during GUI backend initialization). If time runs out before then, it prints
  `startup failed: the page did not load ...` and `exit: reason=startup-timeout code=1`,
  writes sandbox workarounds to stderr, and exits 1. This is because inside Claude Code's
  sandbox (Seatbelt) no window appears, yet the process does not die and lingers.
  The window flags are the app's own state, so `visible=true` can happen even in this situation.
  The watchdog is a separate thread and calls `process::exit` without touching the main thread or the event loop
  (so it fires even if the event loop is stuck). **Whether page loaded really never appears inside the sandbox,
  and what the flags look like there, could not be confirmed on Linux; verification on a real macOS machine is still pending**
- Other instances are detected with `ps -A -o pid=,comm=`. macOS prints the full path of the executable
  in comm, while Linux prints the basename truncated to 15 characters, so the comparison
  matches both (`is_same_program`). This comparison and the parsing of ps output have unit tests

Sample output and the meaning of each line are summarized in the "Lifecycle Log" section of README.md.

### License notices for dependencies (src-tauri/src/notices.rs)

`THIRD-PARTY-NOTICES.txt` is generated by `scripts/generate-third-party-notices.sh` (`pnpm notices`);
`notices.rs` embeds it with `include_str!` and shows it via `--license` and the app menu's Third-Party Licenses
(right below About; the `licenses` window = `src/routes/licenses/+page.svelte`).

- **When adding or upgrading dependencies, rerun `pnpm notices` and commit the result.** If the notices disagree with `Cargo.lock` /
  `pnpm-lock.yaml`, `cargo test` fails
- The Rust side uses cargo-about (`cargo install cargo-about --locked --features cli`). `targets` in `src-tauri/about.toml`
  narrows it to the distribution targets only (the two macOS ones). Windows / Linux-only crates are not listed
- The npm side lists, in `NPM_PACKAGES` inside the script, **the packages vite bundles into the web view**,
  not the `dependencies` of `package.json` (`@tailwindcss/vite` is a build tool,
  `@tauri-apps/plugin-*` are not imported from the frontend, and the SvelteKit runtime is in devDependencies).
  When the frontend imports a new package, add it to `NPM_PACKAGES`
- The menu is `Menu::default` with one item inserted. It is set only on macOS (other OSes still have no menu bar)

### Tauri commands

- `get_clipboard_data`: The frontend fetches data from the backend
- `open_item`: Opens the item at the given index in open mode (open_target.rs)
- `third_party_notices`: Returns the embedded `THIRD-PARTY-NOTICES.txt` (for the licenses window)

## Dependencies

### Rust (src-tauri/Cargo.toml)

```toml
[dependencies]
tauri = { version = "2.11", features = [] }
tauri-plugin-opener = "2.5"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }

[target.'cfg(any(target_os = "macos", windows, target_os = "linux"))'.dependencies]
tauri-plugin-cli = "2.4.1"

[target.'cfg(unix)'.dependencies]
libc = "0.2"   # Used only for the exit log on signal receipt (lifecycle.rs)
```

### Node.js (package.json)

The main dependencies are Svelte5, Tailwind4, Vite, and so on.

Keep the major.minor of the Tauri crates and npm packages (`tauri` ↔ `@tauri-apps/api`, `tauri-plugin-*` ↔ `@tauri-apps/plugin-*`)
in sync. If they diverge, `tauri build` stops before building. `scripts/check-tauri-versions.mjs`
in PR CI checks this.

## Development and build

### Development mode

```bash
pnpm tauri dev
```

### Release build  

```bash
pnpm tauri build
```

### Tests

```bash
# Run each test script
./tests/simple-text.sh
./tests/multi-line-text.sh  
./tests/json.sh
```

Each script sources `tests/config.sh` and launches the app through the `run_clipboard_palette` function.

- `THEME` (`auto` / `light` / `dark`, default `auto`): passed to `--theme`
- `RUN_MODE` (`dev` / `release`, default `dev`): whether to use a debug build via `cargo run` or the already built binary

They can be overridden temporarily with environment variables (e.g. `THEME=dark ./tests/simple-text.sh`).

With `RUN_MODE=dev`, vite (`node_modules/.bin/vite dev`) is started first, then `cargo run`.
If port 1420 is used by another server, it stops with an error (vite uses strictPort).
`pnpm tauri dev` is not used because it does not pass piped standard input through to the app.
Note that passing app arguments to `pnpm tauri dev` requires two `--`
(`pnpm tauri dev -- -- --theme dark`; the tauri CLI and cargo each consume one).

## Coding rules

- For how to use Tauri2, Svelte5, and Tailwind4, refer to the Context7 MCP server
- Rust code follows standard Rust style
- Do not create lines that contain only whitespace
- Write comments in source code in English

## macOS app icon

Generated with `bash scripts/generate-macos-icons.sh` from a fixed source image and `layout.json`.
Small @1x sizes fill the whole background with the artwork centered at 80%; large sizes have a 10% transparent margin on all four sides outside the background.
16pt @2x and 32pt @1x are separate images. Small-size ICNS entries are ARGB, and the large sizes are placed first.
`tauri.macos.conf.json` puts Assets.car directly under Resources, and it is registered with
CFBundleIconName in Info.plist. When the CLI is launched directly, `set_dock_icon` embeds and uses the ICNS.
Render verification: `swift scripts/macos-icon.swift verify resources/app-icons/AppIcon.iconset src-tauri/icons ClipboardPalette`.
Details on the source image, registration, and verification are in `resources/app-icons/README.md`. The PNG/ICO for Windows/Web are maintained as separate assets.
