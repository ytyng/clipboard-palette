#!/usr/bin/env zsh
# テストスクリプト共通の設定・起動処理
# 各テストスクリプトから source して使う
#
# 環境変数で一時的に上書きできる:
#   THEME=dark ./tests/simple-text.sh
#   RUN_MODE=release ./tests/json.sh

# カラーテーマ: auto (OS の設定に従う) / light / dark
: ${THEME:=auto}

# 実行方法: dev (vite + cargo run) / release (ビルド済みバイナリ)
: ${RUN_MODE:=dev}

# 開発サーバーの URL (src-tauri/tauri.conf.json の build.devUrl と合わせる)
: ${DEV_SERVER_URL:=http://localhost:1420}

# 開発サーバーがこのアプリのものか判定するための文字列 (src/app.html の title)
: ${DEV_SERVER_MARKER:=Clipboard Palette}

# このファイルの位置からプロジェクトルートを決定する
PROJECT_ROOT=${${(%):-%x}:a:h:h}
RELEASE_BINARY=$PROJECT_ROOT/src-tauri/target/release/clipboard-palette
VITE_BIN=$PROJECT_ROOT/node_modules/.bin/vite

# 開発サーバーの状態を出力する
#   ours    : このアプリの開発サーバーが応答している
#   foreign : 何かが応答しているが、このアプリのものではない
#   down    : 応答が無い
dev_server_state() {
  local body
  if ! body=$(curl -s --max-time 2 "$DEV_SERVER_URL" 2>/dev/null); then
    echo down
  elif [[ $body == *"$DEV_SERVER_MARKER"* ]]; then
    echo ours
  else
    # vite は strictPort なので、別プロセスが居ると起動できない。
    # 黙って別のページを読み込まないよう down とは区別する
    echo foreign
  fi
}

# 自分が起動した開発サーバーを止める
stop_dev_server() {
  if [[ -n $STARTED_VITE_PID ]]; then
    kill $STARTED_VITE_PID 2>/dev/null
    STARTED_VITE_PID=""
  fi
}

# 中断された場合も vite を残さないようにする。
# zsh では関数内で張った EXIT トラップが「関数の return 時」に発火してしまうため、
# 必ずトップレベル (source された時点) で登録する。
# 対話シェルに source された場合は、Ctrl-C でそのシェルごと終了してしまうので張らない
if [[ ! -o interactive ]]; then
  trap 'stop_dev_server' EXIT
  trap 'stop_dev_server; exit 130' INT TERM
fi

# 開発サーバーが起動していなければ起動する
# 起動した場合はその PID を STARTED_VITE_PID に入れる (既に起動済みなら空)
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
  # exec で置き換えることで $! が vite 本体の PID になる (後で確実に止められる)
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

# 標準入力を受け取りつつ clipboard-palette を起動する
# 引数はそのままアプリに渡される (--multiline など)
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
      # `npm run tauri dev` は標準入力をアプリまで渡さないため使わない。
      # 開発サーバーを別途起動し、cargo run に直接パイプする
      # (デバッグビルドは tauri.conf.json の build.devUrl を読みに行く)
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
