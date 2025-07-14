# clipboard-palette

このアプリは、標準入力に入ってきた内容をクリップボードにコピーするためのボタンを表示するアプリです。

## インストール・ビルド

### 開発環境のセットアップ

```shell
npm install
```

### 開発モードでの起動

```shell
npm run tauri dev
```

### リリースビルド

```shell
npm run tauri build
```

ビルド後の実行ファイルは `src-tauri/target/release/clipboard-palette` に作成されます。

## 使い方

### テキストのみ

```shell
echo "Hello, World!" | clipboard-palette
```

`Hello, World!` をコピーするボタンが 1つだけ表示されます。

### 複数行テキスト（--multiline, -m）

```shell
echo -e "Hello, World!\nこんにちは、世界！" | clipboard-palette --multiline
```

改行で分割し、各行に対してコピーするボタンが表示されます。

### 空行で分割（--split-empty-line, -s）

```shell
echo -e "Hello, World!\n\nこんにちは、世界！" | clipboard-palette --split-empty-line
```

1行の空行で分割し、各セクションに対してコピーするボタンが表示されます。

#### 指定した数の空行で分割

```shell
# 2行以上の空行で分割
echo -e "Section1\n\nSection2\n\n\nSection3" | clipboard-palette --split-empty-line=2

# 以下の書き方も可能
clipboard-palette --split-empty-line 2
clipboard-palette -s 2
```

数値を指定することで、その数以上の連続する空行でテキストを分割できます。

### JSON形式（--json, -j）

```shell
echo '[{"label": "Copy text", "text": "Hello, World!"}, {"label": "日本語", "text": "こんにちは、世界！"}]' | clipboard-palette --json
```

JSON形式で入力を受け取り、各オブジェクトの`label`と`text`フィールドに基づいてボタンを表示します。

## テスト

プロジェクトには動作確認用のテストスクリプトが含まれています：

```shell
# シンプルなテキスト
./tests/simple-text.sh

# 複数行テキスト（空行での分割）
./tests/multi-line-text.sh

# JSON形式
./tests/json.sh
```

