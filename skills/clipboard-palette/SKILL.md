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
of `{"label", "text"}` objects to a temporary file outside the repository
(`mktemp`, or the agent's scratch directory) and pipe it in. The app reads all
of stdin at startup, so the file is deleted right after the log confirms the
read (below); the palette often carries commands, tokens and queries that must
not end up indexed, backed up or committed.

```json
[
  {"label": "Open an SSH tunnel to the production DB", "text": "ssh -N -L 13306:db.internal:3306 bastion"},
  {"label": "Count deleted users", "text": "SELECT COUNT(*) FROM users WHERE deleted_at IS NOT NULL;"}
]
```

```sh
PALETTE=$(mktemp -t palette).json; LOG="$PALETTE.log"   # unique per launch
# ... write the JSON to "$PALETTE" with a file-writing tool ...
clipboard-palette --json < "$PALETTE" > "$LOG" 2>&1 &
for _ in $(seq 1 30); do grep -q '\[lifecycle\] \(startup\|exit\)' "$LOG" && break; sleep 0.5; done
cat "$LOG"; rm -f "$PALETTE"
```

- `label` is what the button shows: say what the item does, not the command.
- `text` is copied verbatim, so it has to be complete and runnable as is.
  Multi-line text uses `\n`. Avoid nesting double quotes inside a command:
  a mis-escaped `\"` still parses as JSON and only shows up when the user runs it.
- Write the JSON with a file-writing tool rather than a shell heredoc; commands
  are full of quotes, backslashes and `$`.
- Run it in the background (`&` or the tool's background option). In the
  foreground the call blocks until the user closes the window.
- Before telling the user anything, read the log for two lines. `Successfully
  read stdin data: N items` with N equal to the number of objects written means
  the input was accepted. `Error reading stdin data: ...` (stderr) means it was
  rejected and the window shows only an error; `Empty input detected, using
  default data` (or `stdin is a terminal`) means the app fell back to its two
  sample buttons, and the success line that follows it is about those.
  `[lifecycle] startup ok: window is visible` means the palette is on screen;
  `startup incomplete` / `startup failed` / `exit:` lines mean it is not, and
  no verdict after the 15-second poll above is a timeout to report as such.
  Announce the palette only when both are good; otherwise report what the log
  says.
- Then tell the user the palette is open and what each button is; the window
  is easy to miss.

`--json` is required; JSON is never auto-detected. Both fields are required in
every object, or the app refuses the input.

Other modes, for input that needs no labels: `-m` (one button per line),
`-s [N]` (split at N consecutive empty lines), or no flag (whole input as one
button). Only the first of `-m`, `-s`, `-j` applies if several are given.

## Notes

- Input must come through a pipe or redirect. With stdin on a terminal, or empty
  input, the app shows sample data instead.
- Secrets in the palette are shown in plain text on screen; say so when handing
  one over.
- Closing the window ends the process. To change the palette, launch it again.
