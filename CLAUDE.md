
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

### データ処理モード

1. **normal**: そのままのテキストを表示
2. **multiline**: 改行で分割して各行をボタン化
3. **split-empty-line**: 指定した数の空行で分割してセクション化
4. **json**: JSON配列を解析してlabel/textペアを生成

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

## コーディングルール

- Tauri2, Svelte5, Tailwind4 の使い方は、Context7 MCP サーバーを参照
- Rust コードは標準的な Rust スタイルに従う
- 空行のみの行は作成しない
- コメントは日本語で記述（ただし関西弁は使わない）
