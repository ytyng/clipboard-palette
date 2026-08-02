#!/usr/bin/env zsh
# Shared settings and launcher for the test scripts.
# Source this from each test script.
#
# The settings can be overridden per run with environment variables:
#   THEME=dark ./tests/simple-text.sh
#   RUN_MODE=release ./tests/json.sh

# Color theme: auto (follow the OS setting) / light / dark
: ${THEME:=auto}

# How to run the app: dev (vite + cargo run) / release (the built binary)
: ${RUN_MODE:=dev}

# Dev server URL (keep in sync with build.devUrl in src-tauri/tauri.conf.json)
: ${DEV_SERVER_URL:=http://localhost:1420}

# Marker used to tell whether the dev server is this app (the title in src/app.html)
: ${DEV_SERVER_MARKER:=Clipboard Palette}

# Resolve the project root from the location of this file
PROJECT_ROOT=${${(%):-%x}:a:h:h}
RELEASE_BINARY=$PROJECT_ROOT/src-tauri/target/release/clipboard-palette
VITE_BIN=$PROJECT_ROOT/node_modules/.bin/vite

# Print the state of the dev server
#   ours    : the dev server of this app is answering
#   foreign : something is answering, but it is not this app
#   down    : nothing is answering
dev_server_state() {
  local body
  if ! body=$(curl -s --max-time 2 "$DEV_SERVER_URL" 2>/dev/null); then
    echo down
  elif [[ $body == *"$DEV_SERVER_MARKER"* ]]; then
    echo ours
  else
    # vite uses strictPort, so it cannot start while another process holds the
    # port. Keep this distinct from down so we never load the wrong page silently
    echo foreign
  fi
}

# Stop the dev server we started ourselves
stop_dev_server() {
  if [[ -n $STARTED_VITE_PID ]]; then
    kill $STARTED_VITE_PID 2>/dev/null
    STARTED_VITE_PID=""
  fi
}

# Make sure vite is not left behind when the script is interrupted.
# In zsh an EXIT trap set inside a function fires when that function returns, so
# it has to be registered at the top level (that is, when this file is sourced).
# Skip it in an interactive shell, where Ctrl-C would then exit the shell itself
if [[ ! -o interactive ]]; then
  trap 'stop_dev_server' EXIT
  trap 'stop_dev_server; exit 130' INT TERM
fi

# Start the dev server unless it is already running.
# When we start it, its PID goes to STARTED_VITE_PID (empty if it was running)
start_dev_server_if_needed() {
  STARTED_VITE_PID=""
  case $(dev_server_state) in
    ours)
      echo "開発サーバーは起動済み: $DEV_SERVER_URL"
      return 0
      ;;
    foreign)
      echo "$DEV_SERVER_URL の応答に \"$DEV_SERVER_MARKER\" が含まれていない" >&2
      echo "別のサーバーが使っているならそれを終了してから再実行する" >&2
      return 1
      ;;
  esac
  if [[ ! -x $VITE_BIN ]]; then
    echo "vite が見つからない: $VITE_BIN ('npm install' を実行する)" >&2
    return 1
  fi
  echo "開発サーバーを起動する: $DEV_SERVER_URL"
  # exec replaces the subshell so that $! is the PID of vite itself,
  # which lets us stop it reliably later
  (cd $PROJECT_ROOT && exec "$VITE_BIN" dev >/dev/null 2>&1) &
  STARTED_VITE_PID=$!
  local i
  for i in {1..60}; do
    sleep 0.5
    [[ $(dev_server_state) == ours ]] && return 0
  done
  echo "開発サーバーが起動しない: $DEV_SERVER_URL" >&2
  stop_dev_server
  return 1
}

# Launch clipboard-palette with standard input attached.
# Arguments are passed through to the app (--multiline and so on)
run_clipboard_palette() {
  case $RUN_MODE in
    release)
      if [[ ! -x $RELEASE_BINARY ]]; then
        echo "リリースバイナリが見つからない: $RELEASE_BINARY" >&2
        echo "先に 'npm run tauri build' を実行するか、RUN_MODE=dev を指定する" >&2
        return 1
      fi
      "$RELEASE_BINARY" --theme "$THEME" "$@"
      ;;
    dev)
      # `npm run tauri dev` is not used because it does not forward piped
      # standard input to the app. Start the dev server separately and pipe
      # into cargo run instead (a debug build loads build.devUrl from
      # tauri.conf.json)
      start_dev_server_if_needed || return 1
      cargo run --manifest-path "$PROJECT_ROOT/src-tauri/Cargo.toml" \
        -- --theme "$THEME" "$@"
      local app_status=$?
      stop_dev_server
      return $app_status
      ;;
    *)
      echo "RUN_MODE の値が不正: $RUN_MODE (release または dev)" >&2
      return 1
      ;;
  esac
}
