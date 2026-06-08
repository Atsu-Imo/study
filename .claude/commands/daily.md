---
description: 毎朝のテックニュース＋脆弱性リサーチを daily/<date>.md にまとめる
argument-hint: "[追加で深掘りしたいキーワード（任意）]"
allowed-tools: Bash(date:*), Bash(git rev-parse:*), WebSearch, WebFetch, Read, Write
---

あなたは毎朝のテック＆セキュリティのリサーチ担当です。以下を実行してください。

## 0. 準備
- `date +%Y-%m-%d` で今日の日付を取得する（変数 DATE とする）。
- `git rev-parse --show-toplevel` でリポジトリのルートを取得し、出力先を `<root>/daily/<DATE>.md` とする。
- 同名ファイルが既にある場合は、上書きせず内容を確認し、追記でなく「再生成して良いか」を一言ユーザーに確認してから進める。

## 1. リサーチ範囲

直近およそ24時間（前回チェック以降）の新しい話題が対象。**2層**で集める。

### アンカー層（必ず WebFetch で直接見る一次ソース）
取りこぼしたくない公式・一次情報。毎回ここを起点にする。

- **My Stack**
  - This Week in Rust: https://this-week-in-rust.org/
  - Rust Blog: https://blog.rust-lang.org/
  - Go Blog: https://go.dev/blog/
  - AWS What's New: https://aws.amazon.com/about-aws/whats-new/recent/feed/
- **Vulnerabilities & Security**（脆弱性は必ずこれらを確認）
  - CISA KEV（悪用が確認された脆弱性）: https://www.cisa.gov/known-exploited-vulnerabilities-catalog
  - GitHub Advisory Database: https://github.com/advisories
  - RustSec: https://rustsec.org/advisories/
  - Go Vulnerability DB: https://pkg.go.dev/vuln/
- **Video / Media / Streaming**
  - AWS Media Blog: https://aws.amazon.com/blogs/media/
  - Streaming Media: https://www.streamingmedia.com/
  - Mux Blog: https://www.mux.com/blog
- **General Tech**
  - 主要AIラボ・大手の公式発表（Anthropic / OpenAI / Google など）

### 発見層（固定リストの偏りを防ぐコミュニティ集約）
人手で選んでいないコミュニティランキングを見て、**アンカー層の外側にある話題**を拾う。

- Hacker News フロントpage: https://news.ycombinator.com/
- lobste.rs: https://lobste.rs/

### 補完
アンカー／発見層で拾った話題の裏取り・深掘りにのみ `WebSearch` を使う（探索の起点にはしない）。

### 取捨選択ルール（膨張防止・偏り防止）
- **各セクション 3〜6件まで**（ハード上限。超えたら重要度で削る）。
- 直近約24時間、重複・宣伝色の強いものは除外。
- 各日、最低1件は**発見層由来**（アンカー層に無かった話題）を含めるよう意識する。
- 引数 `$ARGUMENTS` があれば、そのキーワードを各分野の追加重点テーマにする。

> ソース一覧は固定だが恒久ではない。役に立たないソースが続いたり、良いソースを見つけたら、このファイルを編集して入れ替えてよい。

## 2. 出力フォーマット
`<root>/daily/<DATE>.md` に以下の構成で **新規作成**（Write）する。

- 見出し（記事タイトル）は **英語の原文のまま**。
- 各項目に **日本語で1〜2行の要約** と、なぜ気にすべきか（影響）を添える。
- 必ず **ソースURL** をリンクで付ける。脆弱性には深刻度を明記。
- 該当ニュースが無い分野は「該当なし」と書く（無理に埋めない）。

```markdown
# Daily Tech & Security Digest — <DATE>

> 自動生成。気になった項目はこのセッションでそのまま深掘りできます。

## 🦀 My Stack (Rust / Go / AWS)
- **[英語の見出し](URL)**
  - 日本語要約。なぜ重要か。

## 🔒 Vulnerabilities & Security
- **[英語の見出し](URL)** — `CVSS x.x / Critical`
  - 日本語要約。影響範囲・対象バージョン・対応の要否。

## 🎬 Video / Media / Streaming
- **[英語の見出し](URL)**
  - 日本語要約。

## 📰 General Tech
- **[英語の見出し](URL)**
  - 日本語要約。

---
### 🔖 今日の深掘り候補
- 特に追って調べる価値がある2〜3件を箇条書きで提案。
```

## 3. 仕上げ
- ファイルを書き終えたら、保存先パスと「特に注目の1〜2件」を会話で短く報告する。
- 「どれか深掘りする？」と一言添えて、ユーザーがそのまま続けて掘れるようにする。
