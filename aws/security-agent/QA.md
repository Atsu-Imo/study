# AWS Security Agent — Q&A

2026-07-16

## Q1: ドメイン所有権の検証（①）で、ドメインの持ち主は何をすればいい？（Route53 / ACM 管理）

**結論：Route53 のホストゾーンに検証用 TXT レコードを1本追加するだけ。**

**Route53 が Security Agent と同一 AWS アカウントなら「ワンクリック検証」で TXT 手動追加すら不要**（公式確認済み）。Security Agent が自動で DNS レコードを作成し検証まで完了する。別アカウント/別 DNS プロバイダのときだけ下記の手動 TXT 追加になる。

検証方法（Step1 の Verification method で選択）:
- `DNS_TXT` — TXT レコードで所有権証明（別 DNS プロバイダはこれで手動追加）
- `HTTP_ROUTE` — 指定 URL に検証ファイルを設置して証明
- `PRIVATE_VPC` — プライベート VPC ペンテスト専用。ドメインがプライベート CIDR の IP に解決することを検証

手動 TXT の流れ（同一アカウント Route53 以外の場合）:
1. 作業者が Security Agent コンソールで対象ドメインを検証対象に追加 → 検証用トークン入りの TXT レコード値が発行される
2. ドメインの持ち主が、その名前・値どおりに DNS に TXT レコードを作成
   - Record type: `TXT`、Value: 発行トークン、TTL: 300秒程度、Name: 指定されたホスト名
   - IaC なら `AWS::Route53::RecordSet` でも可
3. Security Agent 側で Verify → DNS 照合で「検証済みドメイン」になる（反映は通常数分）

ポイント:
- **ACM は無関係**。所有権検証は Security Agent 独自トークンで行い、ACM の証明書とは別物。ただし「Route53 に AWS 発行の TXT を1本置いて所有権を示す」やり方は ACM の DNS 検証と同じ要領。
- **サブドメインは再検証不要**（`example.com` を検証すれば `app.example.com` 等もテスト対象にできる）。
- 検証が済むまでテスト自体を作成できない（①が全ての起点）。Route53 を触れる人に最初に依頼するのが段取り上いちばん重要。
- 検証後も TXT は消さないのが無難（再検証を求められることがある）。
- レコード名フォーマット（`_xxx` サブドメインか直下か）や検証が1回きりか継続かは、コンソールが提示する指示が正。

## Q2: アプリに WAF の IP 制限がかかっている。Security Agent を通すために許可すべき IP 範囲は？

**結論：AWS は Security Agent の送信元 IP 範囲を公開していない。IP 許可はできない。**

**重要な訂正（AWS公式推奨との切り分け）:**
- カスタム HTTP ヘッダー自体は公式機能（API リファレンスに `CustomHeader` あり、デフォルトで `User-Agent: securityagent` が付くのも公式記載）。
- **ただし「そのヘッダーでWAFのIP制限を通せ」とはAWSは一切明記していない。** Security Agent の enable/create ドキュメントにも WAF 設定にも、WAF/IP制限の通し方の記述はない。
- よって「秘密ヘッダーでWAFを通す」は〈正規機能 × ペンテスト業界の定石〉であって、**AWS公式推奨手順ではない**。

通し方の優先順位（AWSネイティブな順）:
1. **VPC内から直接テスト（PRIVATE_VPC / VPC構成）← 第一候補**
   - Security Agent を対象アプリの VPC・サブネット内から実行すると、トラフィックは内部から直接オリジンに届く。WAF が CloudFront/ALB のエッジ側なら経由せず、IP制限の問題自体が消える。`PRIVATE_VPC` 検証・VPC/サブネット実行は正式機能。
2. **テスト期間だけ WAF の IP 制限ルールを緩める/外す**
   - 一時的に無効化、または実行中だけ許可を広げる。単純でAWS機能の範囲内。
3. **カスタムヘッダーによる許可（業界定石・公式推奨ではない）**
   - AWS WAF は「特定ヘッダー値一致で Allow」を正式サポート。手段としては正規だが AWS が Security Agent 用途で推奨と明言はしていない。
   - 秘密ヘッダー（例 `X-Pentest-Auth: <ランダム文字列>`）＋期間限定なら実用上OK。`User-Agent: securityagent` だけの許可は詐称容易で弱い。

もう一つの観点:
- ヘッダー許可で WAF をすり抜けさせると「WAF抜きの素のアプリ」を評価することになる。アプリ本体の脆弱性を見たいならむしろ望ましいが、本番同等（WAF込み）で評価したいなら許可しない方が実態に近い。目的次第。

補足: どうしても IP 固定したいなら公式に固定 IP は出ていないので AWS Security Agent サポート/担当に開示可否を問い合わせるしかない。

## Q3: ソースコードや設計書を見せると精度が上がると聞いた。どう設定する？コードレビューに出していれば使われる？

**結論：コードレビューに出しても連携されない。テスト作成時（⑥）に明示的にリソースを添付する必要がある。**

誤解の解消:
- Security Agent は PR のコードレビューに割り込むツールではない。本体は「動いているアプリに対しヘッドレスブラウザで攻撃する」もの。
- ソース/設計書は攻撃精度を上げる任意の参考コンテキスト。添付しなくても動くが、添付で網羅性↑・誤検知↓。
- 「渡してあるから使われる」ではなく「テスト作成時に自分でぶら下げるから使われる」。

設定方法（テスト作成の「Attach additional resources / Connected resources」）:
- A. 既存から選ぶ（Select from available）: 連携済みの GitHub リポジトリ / S3 バケット / 過去アップロードファイル。→ 元の場所と同期され、更新すれば最新版を使う。
- B. その場でアップロード（Upload）: ローカルファイル（アーキ図・OpenAPI/Swagger・設定ファイル）or プレーンテキスト貼り付け（APIエンドポイント一覧など）。→ 添付時点のスナップショット。

渡すと効くもの: GitHub リポジトリ（構造理解＋自動修正PRに必須）/ API仕様 / アーキ図・認証フロー図 / 設定ファイル（要サニタイズ）。

自動修正PR: Enable automatic remediation を有効＋GitHub 接続で、検出脆弱性の修正PRを自動作成。ただし PR はリポジトリの read 権限保持者全員に見える。

注意:
- 機微情報を渡さない（本番資格情報・秘密鍵・PII 禁止、設定ファイルはサニタイズ版）。
- プライベート VPC + GitHub の場合、VPC から GitHub に到達できるようにする（NAT Gateway アウトバウンド許可 or GitHub Meta API の IP 許可）。

## Q4: リソースを S3 バケットにする場合、ディレクトリはどうアップロードする？

**結論：S3 に「ディレクトリ」概念はない（キー名の `/` で擬似フォルダ）。CLI の再帰アップロードで丸ごと上げる。**

```bash
# 差分同期（推奨・再実行が速い）
aws s3 sync ./my-project s3://your-bucket/security-agent/my-project/

# 全ファイルコピー（差分判定しない・単発向け）
aws s3 cp ./my-project s3://your-bucket/security-agent/my-project/ --recursive
```

よく使うオプション:
```bash
aws s3 sync ./my-project s3://your-bucket/security-agent/my-project/ \
  --exclude ".git/*" --exclude "node_modules/*" --exclude "*.env"
aws s3 sync ... --dryrun   # 何が上がるか事前確認
```

実務ポイント:
- 機微情報を上げない。`.env` / `*.pem` / `credentials` / `.git`（履歴に秘密が残る）/ 本番設定は `--exclude` で確実に外し、`--dryrun` で確認してから上げる。
- S3 から選んだリソースは元と同期される。コード修正後は `aws s3 sync` で上げ直せばテストは最新版を使う。
- バケットは Security Agent（Agent Space のロール）から `s3:GetObject` / `s3:ListBucket` できるようにしておく。
- `s3://bucket/security-agent/<アプリ名>/` のようにプレフィックスで用途を分けると紐づけやすい。
- 少量ならコンソールの Upload → Add folder でフォルダごとD&Dも可。数が多い/繰り返すなら CLI の sync が楽。
- 前提: `aws configure` 済みであること。`aws sts get-caller-identity` で実行アカウント/ロールを確認してから上げる。

## Q5: アップロードするものは git archive の tar.gz でいい？

**結論：git archive を「クリーンなファイル取り出し手段」に使うのは◎。ただし tar.gz のまま上げず、展開してから s3 sync する。**

tar.gz のまま上げない理由:
- Security Agent が読むのは中身のソースファイル。リソースは「ファイルをそのまま読む」想定で、tar.gz を自動解凍して読む保証がない（公式に解凍の記載なし）。圧縮のまま渡すと中身の見えない塊になりコンテキストとして効かないリスク。

git archive を経由する利点:
- 追跡ファイルだけをきれいに書き出せる（`.git/` 履歴を含めない、未追跡ファイル=ビルド成果物やローカル `.env` を含めない、`.gitattributes` の export-ignore も効く）。

推奨手順:
```bash
mkdir -p /tmp/pentest-export
git archive HEAD | tar -x -C /tmp/pentest-export   # 追跡ファイルだけ展開（タグ/ブランチ名も可）
ls -R /tmp/pentest-export | head -50               # 秘密混入チェック
aws s3 sync /tmp/pentest-export s3://your-bucket/security-agent/my-app/
```

注意:
- git archive は「追跡されている」ファイルを全部出す。コミット済みの秘密（誤コミットの `.env` 等）はそのまま入るので展開後に確認して外す。
- サブモジュールは git archive に含まれない。必要なら別途 archive して混ぜる。
- テスト対象と同じコミット/タグを指定すると正確。

## Q6: 同一アカウントでもドメイン設定は必要？ VPC を設定するとリクエストはどう飛ぶ？

### ドメイン設定は同一アカウントでも必須
- ワンクリック検証が省くのは「TXT を手で足す作業」だけ。ドメインを対象登録し検証する工程自体は常に必要（検証済みドメインしかテスト対象にできない原則は不変）。
- 公開アプリ: `DNS_TXT`。同一アカウント Route53 ならワンクリックで AWS が自動 TXT 作成・検証。
- VPC 内アプリ: 検証方法に `PRIVATE_VPC` を選ぶ（ドメインがプライベート CIDR の IP に解決することを検証）。検証ステータスが UNREACHABLE でも先に進め、各ペンテスト実行開始時に検証を試みる。

### VPC 設定時のリクエスト経路
モデル: Lambda の VPC 統合や Fargate と同じ「指定サブネットに ENI が生えて、そこから通信する」形（ドキュメントの「指定サブネットにデプロイされたリソースから実行」）。

(A) 対象アプリへ = VPC 内をプライベートに飛ぶ
- 送信元はサブネットの private IP。対象ドメインを VPC 内 DNS（Route53 プライベートホストゾーン等）で内部 IP に解決 → VPC 内ルーティングで内部エンドポイント（内部 ALB / ECS / EC2）に直接到達。
- SG: Agent 側はアウトバウンド許可、対象側は Agent のサブネット/SG からのインバウンド許可が必要。
- 対象ドメインが VPC 内で内部 IP に解決される必要（`PRIVATE_VPC` 検証と噛み合う）。

(B) 外部サービス（Okta SSO / GitHub 等）へ = NAT で外に出る
- SSO ログインやリポジトリ取得のため外向き通信が要る → サブネットに NAT Gateway が必要（ドキュメント明記）。プライベートサブネット + NAT が定石。

### WAF との関係（設置レイヤ次第）
- 「VPC 内実行なら必ず WAF 回避」ではない。
- WAF が CloudFront（エッジ）や公開 ALB にあり、Agent が内部エンドポイントを直接叩く → その経路は WAF を通らない（回避）。
- WAF が Agent の到達先である内部 ALB そのものにある → WAF は依然効く。
- 自社 WAF がどのレイヤ（CloudFront / 公開 ALB / 内部 ALB）にあるかで結論が変わる。

## Q7: VPC はどこから設定する？

設定箇所は2つ（役割が違う）。公開アプリなら両方スキップでよい（VPC 設定は VPC 内アプリのときだけ）。

### ① Agent 初期セットアップ（AWS マネジメントコンソール）
- 場所: コンソール → AWS Security Agent → 「Enable penetration test」ウィザード → Step 3「(Optional) Configure additional capabilities」→「VPCs」セクション（折りたたみ）
- 選ぶもの: VPC / Subnet（複数 AZ 推奨・NAT Gateway 含むもの）/ Security group（アウトバウンド許可）
- 「この Agent が VPC にアクセスできる能力」を持たせる登録。担当は AWS アカウント管理者（役割分担表の④相当）。

### ② 個別のペンテスト作成時（Security Agent Web アプリ）
- 場所: Web アプリ → Create a penetration test → 「Configure VPC resources (optional)」セクション
- 選ぶもの: VPC ID / Subnets / Security group
- 公開アプリならこのセクションごとスキップ可。担当は⑥のテスト作業者本人。

### 使い分け
- VPC 内プライベートアプリ: ① で Agent に VPC を紐づけ → ② のテスト作成時に同じ VPC/サブネット/SG を選ぶ。
- 公開アプリ: VPC 設定は不要（①②とも飛ばす）。

チェックポイント: サブネットは複数 AZ + NAT Gateway あり / Agent 側 SG は対象・外部サービスへのアウトバウンド許可 / 対象側 SG は Agent サブネット・SG からのインバウンド許可 / 対象ドメインが VPC 内で内部 IP に解決されること。

## Q8: Okta の TOTP シークレットはどこで確認する？

**結論：Okta は後から TOTP シークレットを表示できない。認証アプリ登録の瞬間に一度だけ表示される「手動入力キー」をその場で控えるのが唯一の入手方法。**

**重要: 認証器の選択を間違えると secret は取れない。** Okta には TOTP を出せる認証器が2種類ある:
- Okta Verify: 登録 QR が `oktaverify://...`。Okta 独自プロトコルで TOTP シード（共有鍵）はアプリ内にプロビジョニングされ外に出ない。**base32 シークレットを抽出できない** → Security Agent には使えない。
- Google Authenticator: 登録 QR が `otpauth://totp/...?secret=JBSWY3...`。標準 TOTP（RFC 6238）で **QR に base32 シークレットが入っている** → これを使う。

→ QR が `oktaverify://` なら Okta Verify を登録している。**Google Authenticator に切り替える。**

手順（Google Authenticator 認証器を登録する）:
1. 管理者: Okta 管理コンソール → Security → Authenticators → Add authenticator で「Google Authenticator」を有効化（Okta Verify とは別に追加が要る）。
2. Authenticator enrollment policy で、対象テストユーザー（のグループ）に Google Authenticator が登録可能になっているか確認（Okta Verify 必須/優先だと選択肢が出ないことがある）。
3. テストユーザーでログイン → MFA 登録時に「Google Authenticator」を選ぶ（Okta Verify を選ばない）。
4. QR が `otpauth://totp/...?secret=...` になる。「Can't scan?」/「手動で入力」で `JBSWY3...`（手動入力キー）が出る → これを控える。
5. 控えた値を Secrets Manager の `totpSecret` に入れる（生シークレットでも `otpauth://` URI でも可）。

確認のコツ: QR が `otpauth://totp/` で始まればOK（secret あり）。`oktaverify://` なら認証器の選択ミス。既に Okta Verify 登録済みでも Google Authenticator を別途併存登録すればよい。

注意:
- すでに登録済みで控えていない場合は閲覧不可（管理者コンソールでも既存シークレットは見れない。できるのは登録済み確認とリセットのみ）。→ 管理者が該当ファクターをリセット → 再登録し、登録時に新シークレットを控え直す。
- ②のテスト用ユーザー作成の流れの中で、登録した瞬間にキャプチャするのが正解（後回しは取り直しになる）。
- 担当は役割分担表の② Okta 管理者。控えたシークレットを③で AWS 管理者が Secrets Manager に登録。

## Q9: 本番テスト前に「疎通するか／ログインが通るか」だけ確かめる実行はある？

**結論：専用の疎通テスト（dry-run / スモークテスト）モードは無い。代わりに「実行 → 最初の数分のログでログイン成功を確認 → Stop」で代用する。**

理由と代替手順:
- テストは最初にサインイン（ログイン）から始まる設計。進行中はリアルタイムでログを見られ、いつでも停止でき、課金は秒単位。この3つで実質の疎通＆ログイン確認ができる。
1. Create and execute でテスト開始。
2. Penetration test runs → Penetration Test Logs をリアルタイム監視。エージェントがまずログインフローを辿る → アプリ到達 / Okta SSO 画面遷移 / username・password・TOTP が通り認証済みセッションになれたかがログで分かる。login prompt が効くかもここで検証できる。
3. ログイン成功を確認できた時点で Stop。秒課金なので数分で止めればコストはごくわずか。失敗（未到達・タイムアウト）が見えたらそこで止めて設定修正。

コストを疎通確認に寄せる小技:
- Out-of-scope URLs で大半のパスを除外して対象を極小化 → ログイン＋数リクエストで一巡。
- Exclude risk types で攻撃カテゴリを絞り探索を浅くする。

Login Optimization（デフォルト有効）:
- 初回ランでログインフローを学習し skill ファイルに保存 → 次回以降は discovery を飛ばして高速化。初回にログインを一度通すこと自体が疎通確認と高速化を兼ねる。
- ログイン失敗時は login prompt を具体化（フィールド名・ボタン名・遷移を明記）して再挑戦が定石。

## Q11: Okta ログインが失敗し続ける。スクショに「browser version is not supported」「Cookies are required」

**原因：エージェントのヘッドレスブラウザが「非対応ブラウザ/ボット」と判定されている。犯人はデフォルトの `User-Agent: securityagent`。**

- Security Agent はデフォルトで全リクエストに `User-Agent: securityagent` を付ける → Okta から見ると実在しないブラウザ → 「対応ブラウザじゃない」判定 → ボット扱いで Cookie も張れず「Cookies are required」。TOTP 以前の入口で詰む。

対処（効く順）:
1. User-Agent を本物のブラウザ文字列に上書き ← 本命
   - テスト設定の Custom HTTP headers で `User-Agent` を実在の Chrome UA に差し替え（デフォルト securityagent を潰す）。
   - 最も確実なのは、手動ログインできている PC の `navigator.userAgent`（DevTools Console で取得）をそのままコピーして貼る。
2. Accessible domains に Okta ドメイン（`your-org.okta.com`）が入っているか確認。無いと Okta ドメイン上で navigate できずセッション Cookie を保持できない。
3. Okta のボット検知/条件付きアクセスを緩める（⑤）。「cookies required / unsupported browser」は ThreatInsight・ボット検知が自動化クライアントに反応して出すこともある。テストアカウント（グループ）に対し新規デバイスチャレンジ・ボット検知・デバイストラストを緩和。

切り分け手順:
- まず 1〜2（User-Agent 差し替え）だけやって再実行 → ログイン最初の画面で 2 エラーが消えたか確認。
- 消えてもまだ TOTP で落ちるなら、シークレット検証（`oathtool --totp -b <secret>` で生成 → 手元で手動ログイン）で secret の正否を切り分け（oktaverify:// 由来だと不正）。

ログ/スクショの確認: 停止しても run は Penetration test runs に残る（削除されない）。CloudWatch Logs の `/aws/securityagent/...` に全アクションがテキストで記録。Stop は進行中アクションを終えてから巻き取るのでラグあり。

## Q10: Endpoint validation で ClientConnectorDNSError（0 accessible, 2 failed）。アプリは https://dev.{domain} で動いている

**結論：`ClientConnectorDNSError` = 名前解決失敗。Target URL を実際に動いているホストに直す（apex やプレースホルダは NG）。**

診断例（apex にレコードが無いのが原因）:
- `nkeihi.upst.io`（apex）→ A/AAAA/CNAME なし → 引けない = DNS エラー。アプリはここでは動いていない。
- `dev.nkeihi.upst.io` → 公開 IP に解決 ✅ 到達可能。ここが本体。
- `https://{domain}` → プレースホルダをそのまま保存した設定ミス。削除する。

直し方（スコープ設定の Target URLs）:
1. `https://nkeihi.upst.io` を `https://dev.nkeihi.upst.io` に変更（or 行削除して追加）。
2. `https://{domain}` の行を削除。
3. 保存して Endpoint validation を再実行。

所有権検証との関係:
- 所有権検証は親ドメイン（`nkeihi.upst.io` / `upst.io`）で済ませる。TXT は A/AAAA が無くても登録・検証できるので apex にアプリが無くても成立。
- Target の `dev.` は検証済みドメインのサブドメインなので追加の所有権検証は不要（「サブドメインは再検証不要」）。
- 正しい形 = 親で所有権検証 → Target URL は実際に動いている `dev.` サブドメインを指定。apex を Target にする必要はない。

補足:
- 公開 IP に解決するなら公開アプリ扱いで VPC 設定は不要。
- 手元確認: `dig +short dev.<domain>` で解決確認、`curl -I https://dev.<domain>` で 443 到達確認。
  - ⚠️ ただし手元 PC が IP 許可リストに載っている環境では、ローカルからの curl は必ず成功するのでエージェント視点の検証にならない。

## Q12: 「Your OneDrive version is not supported」「Cookies are required」「The page has timed out」の正体（Okta 公式に該当記事あり）

**結論：Okta の CDN（`*.oktacdn.com`）へのアクセスがブロックされているのが原因。OneDrive は一切関係ない（Okta 側の誤解を招くエラーメッセージ）。**

Okta 公式記事「Okta login displays a 'Your OneDrive version is not supported' error」より:

> Network configurations block access to the Okta CDN. This restriction prevents the login page from loading and triggers the error page.

Okta Identity Engine / Classic Engine 両方で発生する、設定起因の問題。

### Security Agent の文脈での意味

**ブロックしている「ネットワーク設定」＝ Security Agent の Accessible domains 許可リストそのもの。**
`your-org.okta.com` だけ登録して `*.oktacdn.com` を入れていないと、エージェントのヘッドレスブラウザは CDN に到達できない。

3 症状が全部これ 1 つで説明できる:
- 静的アセット（JS/CSS）が落ちてこない → 「Your OneDrive version is not supported」
- JS が動かないので Cookie を張れない → 「Cookies are required」
- ページが完成しないまま放置 → state token 期限切れ → 「The page has timed out」

UA を差し替えても無関係なので直らなかった（Q11 の UA 仮説は誤り。撤回済み）。

### 対処：Accessible domains に CDN ドメインを追加

Okta 公式の allow-list 対象のうち今回関係するもの:

| ドメイン | 用途 |
|----------|------|
| `*.oktacdn.com` | **本命。静的アセット CDN。これが抜けている** |
| `*.okta.com` | org ドメイン含む Okta 本体（ワイルドカードで） |
| `*.awsglobalaccelerator.com` | Okta が経路に使用 |
| `ocsp.digicert.com` / `crl3.digicert.com` / `crl4.digicert.com` | 証明書失効確認（**ポート 80**） |

ワイルドカードが使えない場合は、手動ログイン時に DevTools の Network タブで実際のアセットホスト（`ok11static.oktacdn.com` 等）を確認して個別に登録する。

参考: [Okta 公式 KB](https://support.okta.com/help/s/article/okta-login-displays-a-your-onedrive-version-is-not-supported-error) / [Okta IP allow-listing ドキュメント](https://help.okta.com/en-us/content/topics/security/ip-address-allow-listing.htm)

### まだ残っている論点：SSO の「帰り」の IP 制限

CDN を開けてログインが完走するようになっても、**コールバックでアプリに戻る脚**が別途 IP 制限で 403 になる可能性は残る（OIDC/SAML は アプリ → Okta → アプリ と往復するため）。ただし順番としては CDN 開放 → 再実行が先。

**重要：Endpoint validation が通っていても「ブロックされていない」証明にはならない。**
WAF の 403 は TCP + TLS + HTTP が成立した上で返る立派な HTTP レスポンス。到達性チェックは通り実アクセスは全部 403、という状態が普通に起こりうる。

切り分けは**受け側の記録**を見るのが確実（テスト実行時刻の窓で）:
- **ALB アクセスログ**（S3）→ `elb_status_code` と `client:port`。403 が並べばブロック確定。同時に**エージェントの送信元 IP が判明する**。
- **WAF ログ**（CloudWatch Logs / S3）→ `action: BLOCK` と `terminatingRuleId`。どのルールで落ちたかまで出る。

対処は VPC モードで実行し、**自分の NAT Gateway の Elastic IP 経由で出させて、その EIP を許可する**（非公開のエージェント IP を、自分が所有する既知・固定の IP に変換する。Q6 参照）:
- WAF Web ACL の IP set → NAT GW の EIP を追加
- ALB の Security Group → インバウンド 443 に NAT GW の EIP/32 を追加
