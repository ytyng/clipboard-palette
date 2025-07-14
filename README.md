# clipboard-palette

このアプリは、標準入力に入ってきた内容をクリップボードにコピーするためのボタンを表示するアプリです。


## 使い方

### テキストのみ

```shell
echo "Hello, World!" | clipboard-palette
```

`Hello, World!` をコピーするボタンが 1つだけ表示されます。

### 複数行テキスト

```shell
echo -e "Hello, World!\nこんにちは、世界！" | clipboard-palette --multiline
```

改行で分割し、各行に対してコピーするボタンが表示されます。

### 空行で分割

```shell
echo -e "Hello, World!\n\nこんにちは、世界！" | clipboard-palette --split-empty-line
```

空行で分割し、各セクションに対してコピーするボタンが表示されます。

### JSON形式

```shell
echo '[{"label" "Copy text", "text": "Hello, World!"}, ...]' | clipboard-palette --json
```

JSON形式で入力を受け取り、各オブジェクトの`label`と`text`フィールドに基づいてボタンを表示します。

