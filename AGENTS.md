
# clipboard-palette プロジェクト詳細

## 概要

標準入力からテキストを受け取り、クリップボードにコピーするためのボタンを表示する Tauri アプリケーション。

## フレームワーク・技術スタック

- **Tauri2**: デスクトップアプリケーションフレームワーク
- **Svelte5**: フロントエンドフレームワーク  
- **Tailwind4**: CSS フレームワーク
- **Rust**: バックエンド言語
- **clap**: Rust のコマンドライン引数パーサー

## プロジェクト構成

```
clipboard-palette/
├── src/                    # Svelte フロントエンド
│   ├── routes/
│   │   ├── +page.svelte   # メインページ
│   │   ├── Help.svelte    # ヘルプコンポーネント
│   │   ├── TextCard.svelte # テキストカードコンポーネント
│   │   └── licenses/+page.svelte # Third-Party Licenses ウィンドウ
│   └── app.html
├── src-tauri/              # Tauri バックエンド
│   ├── src/
│   │   ├── lib.rs         # メインロジック
│   │   ├── notices.rs     # --license / Third-Party Licenses (THIRD-PARTY-NOTICES.txt を埋め込む)
│   │   ├── open_target.rs # オープンモード (Shift + クリックで URL / パスを開く)
│   │   └── main.rs        # エントリーポイント
│   └── Cargo.toml         # Rust 依存関係
├── tests/                  # テストスクリプト
└── build/                  # ビルド出力
```

## 主要機能

### コマンドライン引数処理

`clap` ライブラリを使用して以下のオプションをサポート：

- `--multiline` / `-m`: 改行で分割
- `--split-empty-line[=N]` / `-s [N]`: N行以上の空行で分割（デフォルト1）
- `--json` / `-j`: JSON形式で解析
- `--theme=auto|light|dark`: カラーテーマの指定 (デフォルト auto = OS 設定に従う)。ウィンドウ内容とタイトルバーの両方に適用される
- `--license`: アプリ自身の MIT ライセンスと `THIRD-PARTY-NOTICES.txt` を stdout に出して終了する (stdin を読まず、ウィンドウも出さない)

### データ処理モード

1. **normal**: そのままのテキストを表示
2. **multiline**: 改行で分割して各行をボタン化
3. **split-empty-line**: 指定した数の空行で分割してセクション化
4. **json**: JSON配列を解析してlabel/textペアを生成

### テーマ制御

- Tailwind の `dark:` バリアントは `@custom-variant` で `html[data-theme="dark"]` ベースに変更している (src/app.css)
- ウィンドウは tauri.conf.json で `"create": false` にし、Rust の setup で `WebviewWindowBuilder::from_config` を使って組み立てる。これは `initialization_script` で `--theme` の値 (`window.__CLIPBOARD_PALETTE_THEME__`) をページへ注入するため
- src/app.html のインラインスクリプトが、注入値 → OS 設定の順で `data-theme` を決める。初回描画より前に確定するのでちらつかない
- 起動データ取得後に `src/lib/theme.ts` の `applyTheme()` を呼ぶ。`auto` のときは `matchMedia` を監視して OS 側の切り替えにも追従する
- タイトルバーは `WebviewWindow::set_theme()` で設定する (macOS ではアプリ全体に効く)

### オープンモード (src-tauri/src/open_target.rs)

Shift を押している間、テキストが URL かファイルパスのカードは下線付きになり、クリックで
コピーの代わりに開く (URL はデフォルトブラウザ、パスは Finder で選択状態にして表示)。

- 判定 (`classify`) は Rust 側でだけ行い、`ClipboardItem.open_kind` (`"url"` / `"path"` / null)
  としてフロントに渡す。`--json` の入力に `open_kind` を書いても無視される (`skip_deserializing`)
- **フロントから送るのは item の index だけ。** `open_item` は state からテキストを読み直して
  もう一度判定してから開く。web view から任意の文字列を開かせないため
- URL は http / https だけ。`file:` や独自スキームはアプリを起動しうるので開かない。
  `//` の直後にホストが書かれていること (`https://?q` / `https:///path` は URL 扱いしない)
- 1 行のテキストだけが対象。相対パスはプロセスの cwd (起動したシェルのディレクトリ) 基準、
  `~` はホームに展開する。Windows 形式 (`C:\` / UNC) は Windows でだけパス扱い
  (macOS では相対パスになり cwd 下の無関係なフォルダを開くため)
- **パスは必ず reveal (`reveal_item_in_dir`) し、`open_path` は使わない。** `.app` バンドルは
  ディレクトリなので、open するとアプリが起動する。存在しないパスは最も近い既存の祖先を reveal する
- 開く処理は `tauri-plugin-opener` の関数を Rust から直接呼んでいる (プラグインの登録も
  JS 側の import も無いので、capability への opener 権限追加は不要)
- Shift の状態は `+page.svelte` が keydown / keyup / mousemove の `shiftKey` で追い、ウィンドウの
  blur で解除する。クリック時の分岐は click イベント自身の `shiftKey` で決める

### ライフサイクルログ (src-tauri/src/lifecycle.rs)

呼び出し元 (シェルスクリプトや AI エージェント) が **stdout だけ**で起動の成否を
判定できるようにするためのログ。全行が `[lifecycle]` で始まる。

- 起動直後に pid と、同じプログラム名で動いている他プロセスを出す。前回の孤児が
  残っている状況をここで見つけられる。検知に失敗した場合は `none` ではなく
  `unknown (理由)` を出す (失敗を「他に無い」と読ませない)
- ウィンドウはイベントループが `RunEvent::Ready` になった時点で可視性を測り、
  `startup ok: window is visible` / `startup incomplete: ...` のどちらかで結論を出す。
  呼び出し元はこの 1 行だけ見ればよい。最小化されている場合も `startup incomplete`
  にする (可視フラグは立つが画面には出ていないため)。可視・最小化のどちらかが取得
  できなかった場合も `startup incomplete` に倒す — 偽の「起動できた」を出さないための
  ログなので、確認できていない状態を ok と報告してはいけない
- 終了理由は `RunEvent` (`ExitRequested` / `Exit` / `WindowEvent`)、シグナル
  (`libc::signal` で SIGINT / SIGTERM / SIGHUP)、パニック (panic hook) の 3 経路で出す。
  SIGKILL は捕捉できないため、`exit:` 行が 1 つも無いまま終わった = 強制終了、と読む
- ウィンドウの build は `RunEvent` より前なので、失敗は `setup` 内で
  `startup failed:` として出す。パニックは stderr にしか出ないため、panic hook で
  stdout にも `exit: reason=panic` を出している
- 他インスタンスの検知は `ps -A -o pid=,comm=` を使う。macOS は comm に実行ファイルの
  フルパス、Linux は 15 文字に切り詰めた basename を出すので、両方に一致するよう
  比較している (`is_same_program`)。この比較と ps 出力のパースには単体テストがある

出力例と各行の意味は README.md の「Lifecycle Log」節にまとめてある。

### 依存ライブラリのライセンス表示 (src-tauri/src/notices.rs)

`THIRD-PARTY-NOTICES.txt` は `scripts/generate-third-party-notices.sh` (`pnpm notices`) の生成物で、
`notices.rs` が `include_str!` で埋め込み、`--license` とアプリメニューの Third-Party Licenses
(About の直下。`licenses` ウィンドウ = `src/routes/licenses/+page.svelte`) に出す。

- **依存を足す・上げる時は `pnpm notices` を流し直してコミットする。** notices が `Cargo.lock` /
  `pnpm-lock.yaml` と食い違うと `cargo test` が落ちる
- Rust 側は cargo-about (`cargo install cargo-about --locked --features cli`)。`src-tauri/about.toml`
  の `targets` で配布ターゲット (macOS の 2 つ) だけに絞っている。Windows / Linux 専用の crate は載らない
- npm 側は `package.json` の `dependencies` ではなく、**vite が web view に bundle するパッケージ**を
  スクリプト内の `NPM_PACKAGES` に列挙している (`@tailwindcss/vite` はビルドツール、
  `@tauri-apps/plugin-*` はフロントから import していない、SvelteKit のランタイムは devDependencies)。
  フロントで新しいパッケージを import したら `NPM_PACKAGES` に足す
- メニューは `Menu::default` に項目を 1 つ挿入したもの。macOS だけで設定する (他 OS はメニューバー無しのまま)

### Tauri コマンド

- `get_clipboard_data`: フロントエンドからバックエンドのデータを取得
- `open_item`: オープンモードで index の item を開く (open_target.rs)
- `third_party_notices`: 埋め込んだ `THIRD-PARTY-NOTICES.txt` を返す (licenses ウィンドウ用)

## 依存関係

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
libc = "0.2"   # シグナル受信時の終了ログのみに使う (lifecycle.rs)
```

### Node.js (package.json)

主要な依存関係は Svelte5, Tailwind4, Vite など。

Tauri の crate と npm パッケージ (`tauri` ↔ `@tauri-apps/api`、`tauri-plugin-*` ↔ `@tauri-apps/plugin-*`)
は major.minor を揃えること。ずれると `tauri build` がビルド前に止まる。PR CI の
`scripts/check-tauri-versions.mjs` が検査する。

## 開発・ビルド

### 開発モード

```bash
pnpm tauri dev
```

### リリースビルド  

```bash
pnpm tauri build
```

### テスト

```bash
# 各テストスクリプトを実行
./tests/simple-text.sh
./tests/multi-line-text.sh  
./tests/json.sh
```

各スクリプトは `tests/config.sh` を source して `run_clipboard_palette` 関数経由でアプリを起動する。

- `THEME` (`auto` / `light` / `dark`, デフォルト `auto`): `--theme` に渡される
- `RUN_MODE` (`dev` / `release`, デフォルト `dev`): デバッグビルドを `cargo run` で使うか、ビルド済みバイナリを使うか

環境変数で一時的に上書きできる (例: `THEME=dark ./tests/simple-text.sh`)。

`RUN_MODE=dev` では vite (`node_modules/.bin/vite dev`) を先に起動してから `cargo run` する。
1420 が別のサーバーに使われている場合はエラーで止まる (vite は strictPort のため)。
`pnpm tauri dev` はパイプした標準入力をアプリまで渡さないため使わない。
なお `pnpm tauri dev` にアプリ用の引数を渡す場合は `--` が2つ必要
(`pnpm tauri dev -- -- --theme dark`。tauri CLI / cargo が1つずつ消費する)。

## コーディングルール

- Tauri2, Svelte5, Tailwind4 の使い方は、Context7 MCP サーバーを参照
- Rust コードは標準的な Rust スタイルに従う
- 空行のみの行は作成しない
- ソースコード内のコメントは英語で記述する
