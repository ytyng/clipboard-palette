
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
│   │   └── TextCard.svelte # テキストカードコンポーネント
│   └── app.html
├── src-tauri/              # Tauri バックエンド
│   ├── src/
│   │   ├── lib.rs         # メインロジック
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

### Tauri コマンド

- `get_clipboard_data`: フロントエンドからバックエンドのデータを取得

## 依存関係

### Rust (src-tauri/Cargo.toml)

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }

[target.'cfg(any(target_os = "macos", windows, target_os = "linux"))'.dependencies]
tauri-plugin-cli = "2.4.0"
```

### Node.js (package.json)

主要な依存関係は Svelte5, Tailwind4, Vite など。

## 開発・ビルド

### 開発モード

```bash
npm run tauri dev
```

### リリースビルド  

```bash
npm run tauri build
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
`npm run tauri dev` はパイプした標準入力をアプリまで渡さないため使わない。
なお `npm run tauri dev` にアプリ用の引数を渡す場合は `--` が3つ必要
(`npm run tauri dev -- -- -- --theme dark`。npm / tauri CLI / cargo が1つずつ消費する)。

## コーディングルール

- Tauri2, Svelte5, Tailwind4 の使い方は、Context7 MCP サーバーを参照
- Rust コードは標準的な Rust スタイルに従う
- 空行のみの行は作成しない
- コメントは日本語で記述（ただし関西弁は使わない）
