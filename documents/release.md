# リリース (GitHub Actions ビルド + 署名 + 公証 + Homebrew cask)

macOS 向けの universal dmg を GitHub Actions でビルドし、Developer ID で署名 +
Apple の公証 (notarization) + staple まで通して GitHub Release に公開する。
Homebrew tap ([ytyng/homebrew-tap](https://github.com/ytyng/homebrew-tap)) の
`Casks/clipboard-palette.rb` は tap 側の workflow が毎時、公開済みの最新 Release を
見に来て書き換える。このリポジトリから tap へ push はしない。

## 使い方

```shell
pnpm release           # 0.1.0 -> 0.1.1 (patch, 既定)
pnpm release minor     # 0.1.0 -> 0.2.0
pnpm release major     # 0.1.0 -> 1.0.0
```

`scripts/release.sh` が以下を行う。

1. `main` ブランチ・クリーンな作業ツリー・`HEAD == origin/main` を検証
2. `src-tauri/tauri.conf.json` の version を採番し、`package.json` にも同じ version を反映
   (pnpm-lock.yaml は自パッケージの version を持たないので触らない)
3. `chore: release vX.Y.Z` を commit して `main` に push
4. その push で始まった run を (head SHA で) 見つけて完了まで watch

リリースを始めるのは push であってスクリプトではない。`tauri.conf.json` の version を
手で変えて push しても同じことが起きる。

version の反映は 2 ファイルの該当フィールドを直接書き換える (JSON 全体を再整形しない)。

## 中核: 「version が変わったか」ではなく「その version が公開済みか」で決める

`release.yml` は `main` への push ごとに起動し、`plan` ジョブが
`GET /repos/{repo}/releases/tags/v<version>` を叩く。

- 404 → 未公開。`test` → `build` → `publish` が走る。
- 200 → 公開済み。何もしない (version を変えない push は checkout と API 1 回で終わる)。
- それ以外 (rate limit / 障害) → 判定不能として **失敗させる**。未公開と読むと、公開済みの
  version をもう一度ビルドして publish しにいく。

diff を見ないので、squash / rebase / 直 push のどれで着地しても結果は同じ (冪等)。
`paths: [src-tauri/tauri.conf.json]` で絞らないのも同じ理由で、ビルドや workflow の
不具合で失敗したリリースを「原因を直して push」で再試行できるようにするため
(その修正は version を触らない)。**失敗しても version を上げ直さない**。

`workflow_dispatch` は「version は正しいのに run が一時障害で落ちた」時の再実行用で、
同じ判定を通るため公開済みの version をもう一度出すことはできない。`main` 以外の ref
からの dispatch は plan の冒頭で拒否する (未公開 version を載せたブランチの内容で
リリースされないように)。

## 構成上の判断 (なぜこうなっているか)

- **draft → publish の 2 段構え**。tauri-action は `v<version>` の Release を
  draft で作り、ビルドが全て成功した後に `publish` ジョブが
  `gh release edit --draft=false` で公開する。将来 Windows leg を matrix に
  足した時、片方だけ成功した不完全な Release が公開されるのを防ぐ。失敗した run が
  残した draft は、同じ version の再実行で tauri-action がそのまま使う。
- **PR では test だけが走る**。`plan` が `pull_request` で skipped になり、`build` /
  `publish` は道連れで skipped になる。`test` は `!cancelled()` で自動スキップを外し、
  PR か「リリースする version」の時だけ走る (version を変えない main への push で
  macOS ランナーを動かさない)。
- **コマンド名は `release`**。`publish` は npm/pnpm 組み込みコマンドと衝突する。
- **`tauriScript: pnpm exec tauri`** を明示する。省略すると tauri-action は
  `pnpm tauri build` を実行するため、`package.json` の `tauri` スクリプトに
  `APPLE_SIGNING_IDENTITY='...' tauri` のようなインライン代入を足した瞬間に、
  workflow から渡した env が黙って上書きされる (シェルのインライン代入は継承 env
  より強い)。CLI を直接叩けば Secret 側が唯一の正になる。
- **`concurrency` は `cancel-in-progress: false` + `queue: max`**。1 push =
  1 version なので、run がキャンセルされるとその version の公開が遅れる
  (bump コミットは main に載ったまま)。既定の `queue: single` は pending を 1 件
  しか保持せず新しい push が既存の pending を潰すため、`queue: max` が要る。
  PR の run は commit ごとに別グループにして、リリースのキューに並ばせない。
- **`uses:` は全て commit SHA 固定**。Apple の秘密鍵入り証明書を扱うジョブなので、
  タグが差し替えられると secrets を抜かれる。更新時は行末の `# v4` コメントを
  頼りに新しい SHA を調べる。`dtolnay/rust-toolchain` は **master 履歴**の SHA を
  pin すること (`stable` ブランチ先端の SHA は将来 GC されて run が落ちる)。
- **`persist-credentials: false`**。write 権限の `GITHUB_TOKEN` を `.git/config`
  に残さない (`pnpm install` の install script や third-party action から拾えてしまう)。
- **フロントエンドのビルドを Secret の無いステップに分離する**。`tauri build` は
  `beforeBuildCommand` (`pnpm build`) を子プロセスとして起動し、子プロセスは
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
- **Homebrew cask は tap 側から更新する (プロジェクト側から push しない)**。
  プロジェクトから tap へ push する形だと、tap に書ける token を全プロジェクトに
  配ることになる。tap 側から聞きに行けば、Actions が自分のリポジトリに対して持つ
  `GITHUB_TOKEN` だけで済み、新しい secret はゼロ。代償は次の毎時 run までの遅れだけ。cask には `binary` stanza が入っており、`brew install` だけで実行ファイルが
  Homebrew の bin にリンクされる (手動の `ln -s` は不要になる)。

## 必要な Repository Secrets

`ytyng/clipboard-palette` に以下 6 つ。

| Secret | 内容 |
| --- | --- |
| `APPLE_CERTIFICATE` | Developer ID Application 証明書 + 秘密鍵の `.p12` を base64 したもの |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` のパスワード |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: <Name> (<TeamID>)` |
| `APPLE_ID` | Apple アカウントのメールアドレス |
| `APPLE_PASSWORD` | App用パスワード (通常のパスワードは不可) |
| `APPLE_TEAM_ID` | 10 桁の Team ID |

workflow は最初に APPLE_* の 6 つが揃っているかを検査して、欠けていれば即失敗する。
これが無いと「署名も公証もされていない dmg」が成功扱いで公開されてしまう。
tap 用の token (`HOMEBREW_TAP_TOKEN`) はもう使わない。

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

Homebrew 側は (tap の次の毎時 run が成功した後に) cask の version / sha256 が新しい
Release と一致しているかを確認する。待ちたくなければ tap の workflow を手で起動する。

```shell
gh workflow run update.yml -R ytyng/homebrew-tap
gh api repos/ytyng/homebrew-tap/contents/Casks/clipboard-palette.rb -q .content | base64 -d
brew install --cask ytyng/tap/clipboard-palette
clipboard-palette --help
```

`source=Notarized Developer ID` と staple 成功が出れば、ユーザーがダウンロードして
開いても Gatekeeper 警告は出ない。「署名されている」だけでは
`APPLE_SIGNING_IDENTITY` が効いている確認にならないので、`Authority=` が Secret に
入れた identity と一致することまで見ること。

## 既知の弱点

- **version 変更が紛れた PR をマージした瞬間に公開される**。version の変更は
  `pnpm release` (独立したコミット) で行い、機能 PR に混ぜないこと。
- **Rust 側のビルドスクリプトには依然として secrets が見える**。フロントエンドの
  ビルドは分離したが、`tauri build` は署名・公証と一体で cargo のビルドを走らせる
  ため、`APPLE_PASSWORD` / `GITHUB_TOKEN` を持つ環境で Rust 依存クレートの
  `build.rs` が実行される。完全に塞ぐには「未署名でビルド → 別ステップで codesign +
  notarytool + stapler を手動実行」まで分解する必要があり、tauri-action を捨てて
  workflow が大幅に複雑化する。cargo の依存は `Cargo.lock` で固定されているため、
  現状はこのリスクを受け入れている。
- `pnpm release` は `main` へ**直接 push** する。ブランチ保護 (PR 必須) を
  掛けると破綻する。掛けるなら version bump を PR で出す運用にする (workflow 側は
  そのままで動く)。
- Windows ビルドは含めていない。必要になったら `release.yml` の matrix に
  `windows-latest` / `--bundles nsis` の leg を足す (APPLE_* は
  `matrix.platform == 'macos-latest'` の条件式で既に macOS 限定になっている)。
