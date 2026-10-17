---
name: clipboard-palette
description: "Hand the user commands, SQL, URLs or tokens as one-click copy buttons with the `clipboard-palette` CLI (macOS). Use when the user has to run something themselves (an interactive login, a query in their own DB client, a command the agent is not allowed to run) or asked for text to be made easy to copy. Japanese triggers: コマンドを渡して, コピーしやすくして, パレットで出して."
---

# clipboard-palette

`clipboard-palette` reads text from stdin and opens a small window with one
copy-to-clipboard button per item. Use it instead of asking the user to select
text out of the chat.

Install: `brew install --cask ytyng/tap/clipboard-palette` (links the
`clipboard-palette` command). Check with `clipboard-palette --version`.

## Use the JSON mode

Collect what the user has to run or paste themselves, then write a JSON array
of `{"label", "text"}` objects to a temporary file outside the repository (the
agent's scratch directory) and pass it with `--input`. The app reads the file
at startup, so the file is deleted right after the output confirms the read
(below); the palette often carries commands, tokens and queries that must not
end up indexed, backed up or committed.

```json
[
  {"label": "Open an SSH tunnel to the production DB", "text": "ssh -N -L 13306:db.internal:3306 bastion"},
  {"label": "Count deleted users", "text": "SELECT COUNT(*) FROM users WHERE deleted_at IS NOT NULL;"}
]
```

Launch it as a single plain command with a literal path, in the background
(the Bash tool's background option), and read its output from there:

```sh
clipboard-palette --json --input /path/to/scratch/palette.json
```

Keep the call exactly that shape: no `<` / `>` redirect, no pipe, no `$(...)`,
no `cd` or `&&` chain. A sandboxed agent (Claude Code) only lets a call out of
its sandbox when the whole call matches `sandbox.excludedCommands`, and any of
those keeps it inside, where no window can open (see Notes). Delete the file
in a separate call once the output shows the read.

The output ends with a verdict within about 10 seconds. Wait for one of:

- `[lifecycle] startup ok: window is visible` **and** `[lifecycle] page loaded`
- a `[lifecycle] startup failed`, `[lifecycle] startup incomplete` or
  `[lifecycle] exit:` line

- `label` is what the button shows: say what the item does, not the command.
- `text` is copied verbatim, so it has to be complete and runnable as is.
  Multi-line text uses `\n`. Avoid nesting double quotes inside a command:
  a mis-escaped `\"` still parses as JSON and only shows up when the user runs it.
- Write the JSON with a file-writing tool rather than a shell heredoc; commands
  are full of quotes, backslashes and `$`.
- Run it in the background. In the foreground the call blocks until the user
  closes the window.
- Before telling the user anything, read the output for two lines. `Successfully
  read stdin data: N items` with N equal to the number of objects written means
  the input was accepted. `Error reading stdin data: ...` (stderr) means it was
  rejected and the window shows only an error; `Empty input detected, using
  default data` (or `stdin is a terminal`) means the app fell back to its two
  sample buttons, and the success line that follows it is about those.
  `[lifecycle] startup ok: window is visible` together with
  `[lifecycle] page loaded` (the page reached the app) means the palette is
  on screen and working;
  `startup incomplete` / `startup failed` / `exit:` lines mean it is not, and
  anything else after about 15 seconds (for example only one of the two
  success lines) is not a success: report it as a timeout.
  The app gives up by itself (exit status 1) when the page has not loaded
  within 10 seconds (`--startup-timeout`).
  Announce the palette only when both are good; otherwise report what the output
  says.
- Then tell the user the palette is open and what each button is; the window
  is easy to miss.

`--json` is required; JSON is never auto-detected. Both fields are required in
every object, or the app refuses the input.

Other modes, for input that needs no labels: `-m` (one button per line),
`-s [N]` (split at N consecutive empty lines), or no flag (whole input as one
button). Only the first of `-m`, `-s`, `-j` applies if several are given.

## Notes

- It cannot open a window from inside a sandbox (Claude Code's sandbox, macOS
  Seatbelt): the output shows `Connection Invalid error for service
  com.apple.hiservices-xpcservice`, then `startup failed: the page did not
  load ...` and `exit: reason=startup-timeout`. The command has to run outside
  it: `sandbox.excludedCommands` needs an entry matching the call (such as
  `clipboard-palette *`), and the call must have the plain shape above,
  because a redirect, pipe or `$(...)` keeps a matching call sandboxed. If it
  still fails, tell the user why instead of announcing the palette.
- Without `--input` the text comes from stdin. With stdin on a terminal, or
  empty input, the app shows sample data instead; an empty `--input` file is
  an error.
- Secrets in the palette are shown in plain text on screen; say so when handing
  one over.
- Closing the window ends the process. To change the palette, launch it again.
