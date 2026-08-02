# リリース (GitHub Actions ビルド + 署名 + 公証)

macOS 向けの universal dmg を GitHub Actions でビルドし、Developer ID で署名 +
Apple の公証 (notarization) + staple まで通して GitHub Release に公開する。

## 使い方

```shell
npm run release              # 0.1.0 -> 0.1.1 (patch, 既定)
npm run release -- minor     # 0.1.0 -> 0.2.0
npm run release -- major     # 0.1.0 -> 1.0.0
```

`scripts/release.sh` が以下を行う。

1. `main` ブランチ・クリーンな作業ツリー・`HEAD == origin/main` を検証
2. `src-tauri/tauri.conf.json` の version を採番し、`package.json` /
   `package-lock.json` にも同じ version を反映
3. `chore: release vX.Y.Z` を commit して `main` に push
4. `gh workflow run release.yml` で workflow を起動し、完了まで watch

version の反映は 3 ファイルの該当フィールドを直接書き換える。
`npm install --package-lock-only` は使わない — registry に問い合わせて依存ツリーを
再解決するため、version 採番と無関係な差分がリリースコミットに混ざり、
「ビルドしたもの != テストしたもの」になり得るから。なお `npm ci` はトップレベルの
version 不一致では失敗しない (実測確認済み) ので、lock の version を揃えているのは
整合性のためだけ。

## 構成上の判断 (なぜこうなっているか)

- **トリガーは `workflow_dispatch` のみ**。push ごとにビルドしない (macOS runner
  は消費が大きく、リリース以外でビルドする意味がない)。
- **draft → publish の 2 段構え**。tauri-action は `v<version>` の Release を
  draft で作り、ビルドが全て成功した後に `publish` ジョブが
  `gh release edit --draft=false` で公開する。将来 Windows leg を matrix に
  足した時、片方だけ成功した不完全な Release が公開されるのを防ぐ。
- **version を毎回インクリメントする**。公開済みの version で workflow を再実行
  すると、tauri-action が draft 状態の不一致でエラーになる。採番を自動化して
  「bump し忘れ」を構造的に消している。
- **コマンド名は `release`**。`publish` は npm/pnpm 組み込みコマンドと衝突する。
- **`tauriScript: npm exec -- tauri`** を明示する。省略すると tauri-action は
  `npm run tauri build` を実行するため、`package.json` の `tauri` スクリプトに
  `APPLE_SIGNING_IDENTITY='...' tauri` のようなインライン代入を足した瞬間に、
  workflow から渡した env が黙って上書きされる (シェルのインライン代入は継承 env
  より強い)。CLI を直接叩けば Secret 側が唯一の正になる。
- **`concurrency` は `cancel-in-progress: false` + `queue: max`**。1 dispatch =
  1 version なので、run がキャンセルされるとその version は永久に公開されない
  (bump コミットは main に載ったまま)。既定の `queue: single` は pending を 1 件
  しか保持せず新しい dispatch が既存の pending を潰すため、`queue: max` が要る。
- **`uses:` は全て commit SHA 固定**。Apple の秘密鍵入り証明書を扱うジョブなので、
  タグが差し替えられると secrets を抜かれる。更新時は行末の `# v4` コメントを
  頼りに新しい SHA を調べる。`dtolnay/rust-toolchain` は **master 履歴**の SHA を
  pin すること (`stable` ブランチ先端の SHA は将来 GC されて run が落ちる)。
- **`persist-credentials: false`**。write 権限の `GITHUB_TOKEN` を `.git/config`
  に残さない (`npm ci` の install script や third-party action から拾えてしまう)。
- **フロントエンドのビルドを Secret の無いステップに分離する**。`tauri build` は
  `beforeBuildCommand` (`npm run build`) を子プロセスとして起動し、子プロセスは
  環境変数を継承する。分離しないと vite とその依存パッケージが `APPLE_PASSWORD` /
  `GITHUB_TOKEN` を読める環境で動くことになり、悪意ある依存が 1 つ混ざるだけで
  持ち出せてしまう。署名ビルド側は `src-tauri/tauri.ci.conf.json` を `--config` で
  重ねて `beforeBuildCommand` を空にし、二重ビルドを避ける。
  この設定を `--config {"build":{...}}` のインライン JSON で渡してはいけない —
  tauri-action は `args` を string-argv でパースしてクォートを剥がすため JSON が
  壊れる。クォートを含まないファイルパスなら影響を受けない。
- **ローカルは ad-hoc 署名のまま**。`tauri.conf.json` に `signingIdentity: "-"` を
  残しておくと、env の無いローカルビルドは ad-hoc、CI は `APPLE_SIGNING_IDENTITY`
  が config を上書きして Developer ID で署名する (tauri-cli の優先順位 env > config)。

## 必要な Repository Secrets

`ytyng/clipboard-palette` に以下 6 つ (登録済み)。

| Secret | 内容 |
| --- | --- |
| `APPLE_CERTIFICATE` | Developer ID Application 証明書 + 秘密鍵の `.p12` を base64 したもの |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` のパスワード |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: <Name> (<TeamID>)` |
| `APPLE_ID` | Apple アカウントのメールアドレス |
| `APPLE_PASSWORD` | App用パスワード (通常のパスワードは不可) |
| `APPLE_TEAM_ID` | 10 桁の Team ID |

workflow は最初に 6 つが揃っているかを検査して、欠けていれば即失敗する。
これが無いと「署名も公証もされていない dmg」が成功扱いで公開されてしまう。

## 公開後の検証

Release の dmg をダウンロードして実機確認する。

```shell
hdiutil attach -nobrowse -quiet clipboard-palette_X.Y.Z_universal.dmg
APP=/Volumes/clipboard-palette/clipboard-palette.app
codesign -dv --verbose=2 "$APP"      # Authority=Developer ID Application: ... / flags=...runtime
spctl -a -vvv "$APP"                 # accepted / source=Notarized Developer ID
xcrun stapler validate "$APP"        # The validate action worked!
lipo -archs "$APP/Contents/MacOS/clipboard-palette"   # x86_64 arm64
hdiutil detach -quiet /Volumes/clipboard-palette
```

`source=Notarized Developer ID` と staple 成功が出れば、ユーザーがダウンロードして
開いても Gatekeeper 警告は出ない。「署名されている」だけでは
`APPLE_SIGNING_IDENTITY` が効いている確認にならないので、`Authority=` が Secret に
入れた identity と一致することまで見ること。

## 既知の弱点

- **Rust 側のビルドスクリプトには依然として secrets が見える**。フロントエンドの
  ビルドは分離したが、`tauri build` は署名・公証と一体で cargo のビルドを走らせる
  ため、`APPLE_PASSWORD` / `GITHUB_TOKEN` を持つ環境で Rust 依存クレートの
  `build.rs` が実行される。完全に塞ぐには「未署名でビルド → 別ステップで codesign +
  notarytool + stapler を手動実行」まで分解する必要があり、tauri-action を捨てて
  workflow が大幅に複雑化する。cargo の依存は `Cargo.lock` で固定されているため、
  現状はこのリスクを受け入れている。
- `npm run release` は `main` へ**直接 push** する。ブランチ保護 (PR 必須) を
  掛けると破綻する。掛けるなら tag 駆動 (CI で version 注入) に切り替えること。
- Windows ビルドは含めていない。必要になったら `release.yml` の matrix に
  `windows-latest` / `--bundles nsis` の leg を足す (APPLE_* は
  `matrix.platform == 'macos-latest'` の条件式で既に macOS 限定になっている)。
