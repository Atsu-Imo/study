# AWS Security Agent — 自動ペネトレーションテスト 実施手順

AWS Security Agent（2026年3月GA）を使って、自社アプリに対する自動ペネトレーションテストを
実施するための作業ノート。マルチエージェント構成で、手動なら2〜6週間かかるテストを1〜2日に短縮する。

対象アプリの前提：
- 認証は **Okta SSO**
- MFA は **TOTP**（認証アプリのワンタイムコード）
- 資格情報は **AWS Secrets Manager** に格納

---

## サービス概要（なぜ動くか）

- ヘッドレスブラウザをLLMで操作し、人間と同じようにアプリを操作・攻撃する
- **OAuth / SAML / Okta / MFA を含むログインフロー**をエージェント自身が辿れる（SSO対応）
- ソースコード・アーキテクチャ図・API仕様を事前に渡すと、単発の脆弱性が連鎖する
  攻撃チェーンまで検出する（従来スキャナとの差別化ポイント）

### 対応リージョン（6つ）
バージニア北部 / オレゴン / アイルランド / フランクフルト / シドニー / **東京**

### 料金
**$50 / task-hour（秒単位課金）**。平均的なアプリで約24 task-hour ≒ **約$1,200**。
小規模 $400 〜 大規模 $2,400 程度。

---

## 役割分担 ⚠️ 最重要

ペネトレーションテストの**作業者（＝テストを設定・実行する人）だけでは完結しない**。
以下は事前に**別の担当者に依頼**が必要なタスク。

| タスク | 担当 | 内容 | 依存 |
|--------|------|------|------|
| **① ドメイン所有権の検証** | DNS / インフラ管理者 | 対象ドメインに DNS TXT レコードを追加（or 検証ファイル設置）。**検証済みドメインしかテスト対象にできない** | 最初にやる。これが無いとテスト作成不可 |
| **② Okta テスト用ユーザー作成** | Okta 管理者 | 代表的な権限を持つ**テスト専用アカウント**を作成。**MFAはTOTPに設定**（プッシュ/SMS/FIDOは非対応）。TOTPシークレット（`otpauth://` or `JBSWY3...`）を控える | 個人/管理者アカウントは使わない |
| **③ Secrets Manager への資格情報登録** | AWSアカウント管理者 | ②のID/パス/TOTPシークレットを Secrets Manager に登録。**Security Agent と同じAWSアカウント**に作る（クロスアカウント不可） | ②の完了後 |
| **④ IAMロール構成** | AWSアカウント管理者 | Agent Space 用ロールに Secrets Manager / VPC / CloudWatch Logs 権限を付与 | Agent Space 作成時 |
| **⑤ Okta 条件付きアクセスの調整** | Okta 管理者 | テストアカウントに対しデバイストラスト/新規デバイスチャレンジ/IP制限/ボット検知が自動ログインを妨げないよう緩和 | 必要に応じて |
| **⑥ テスト作成・実行** | **ペネトレーションテスト作業者（本人）** | 下記「作業者本人の手順」 | ①〜⑤が揃った後 |

> **要点**：作業者本人が動く前に、**DNS管理者・Okta管理者・AWSアカウント管理者**の3者に
> ①②③④⑤を段取りしておく必要がある。

---

## Secrets Manager はどのアカウントに作るか

**AWS Security Agent（Agent Space）をセットアップしたアカウントと同じアカウント**に作る。

> 公式：「Secrets Manager secrets and Lambda functions must be in the **same AWS account as
> your AWS Security Agent setup**. Cross-account credentials are not currently supported.」

⚠️ **テスト対象アプリが別アカウントで動いていても、シークレットは Security Agent 側の
アカウントに作る**。IAMロールには `secretsmanager:GetSecretValue` と
`secretsmanager:DescribeSecret` を付与する。

### 格納するシークレット（JSON形式）

```json
{
  "username": "pentest-user@example.com",
  "password": "secure-password-here",
  "totpSecret": "JBSWY3DPEHPK3PXP"
}
```

- `totpSecret` は生シークレットでも `otpauth://totp/...` URI でも可
- TOTPを入れておくと、2FAプロンプト検出時に**エージェントがワンタイムコードを自動生成・入力**する

---

## 作業者本人の手順（⑥）

前提：役割分担表の ①〜⑤ が完了していること。

### 1. Agent Space / テストの下準備
- AWS Security Agent Web アプリにログイン
- **Penetration tests** → **Create a penetration test**

### 2. スコープ設定
| 項目 | 値 |
|------|-----|
| **Penetration test name** | 分かりやすい名前（アプリ/環境が判別できる） |
| **Target URLs** | 検証済みの対象アプリドメイン（サブドメインは追加検証不要） |
| **Accessible domains** | **Okta のドメイン**（例：`your-org.okta.com`）を追加 ← SSO必須。所有権検証は不要 |
| **Out-of-scope URLs**（任意） | 破壊的操作・管理機能のパスを除外 |

### 3. 認証設定
- **Authentication credentials** → **Advanced setting** → **AWS Secrets Manager**
- 上で作ったシークレットを選択
- **Access URL** に、この資格情報を使う対象ドメインを指定

### 4. login prompt（成功率に直結、必ず書く）
```
対象アプリにアクセスするとOktaのSSOログイン画面にリダイレクトされる。
1. Usernameフィールドに提供されたusernameを入力してNextをクリック
2. Passwordフィールドに提供されたpasswordを入力してVerifyをクリック
3. TOTPコードの入力を求められたら、提供されたコードを入力してVerify
4. 認証成功後、元のアプリ画面にリダイレクトされることを確認
```

### 5. IAMロール / VPC
- **Service roles** で Secrets Manager 参照権限を持つロールを選択
- **CloudWatch log group**（任意。未指定なら `/aws/securityagent` プレフィックスで自動作成）
- 対象がプライベート（VPC内）なら **VPC / Subnets / Security group** を設定。公開なら不要

### 6. コンテキスト添付（任意・推奨）
- GitHubリポジトリ / API仕様（OpenAPI/Swagger）/ アーキテクチャ図 を添付すると
  網羅性が上がり誤検知が減る
- Auto-remediation を有効にすると修正PRを自動作成（read権限のある全員に見える点に注意）

### 7. 実行
- **Create and execute** で開始
- 初回は **Login Optimization**（デフォルト有効）がログインフローを学習 → 次回以降は高速化
- 進捗は **Penetration test runs** で監視（規模により数時間）

---

## AWS ペネトレーションテストポリシー上の注意

- テスト対象は**自社所有リソースのみ**。AWS基盤そのもの（サービス内部・物理基盤・テナント分離）
  へのテストは禁止（責任共有モデルの「Security OF the cloud」はAWS責任）
- 大半のサービスは事前承認不要だが、**C2 / DoSシミュレーション / Red・Blue・Purpleチーム演習は
  事前承認が必須**
- テスト対象の全ドメインについて、**自社がセキュリティテストを行う権限を持っていること**が前提

---

## 準備チェックリスト

- [ ] ①対象ドメインの所有権検証（DNS TXT）← DNS管理者
- [ ] ②Oktaテスト用ユーザー作成、MFA=TOTP設定、シークレット控え ← Okta管理者
- [ ] ③Secrets Manager にID/パス/TOTPを登録（Security Agentと同一アカウント）← AWS管理者
- [ ] ④IAMロールに Secrets Manager / VPC / CloudWatch Logs 権限 ← AWS管理者
- [ ] ⑤Okta 条件付きアクセスがテストアカウントの自動ログインを妨げないか確認 ← Okta管理者
- [ ] ⑥Accessible domains に Okta ドメインを追加
- [ ] ⑥login prompt に手順を詳細記述
- [ ] （任意）GitHub / API仕様 / アーキテクチャ図を添付

---

## 参考リンク

- [AWS Security Agent 製品ページ](https://aws.amazon.com/security-agent/)
- [オンデマンドペネトレーションテスト GA発表](https://aws.amazon.com/blogs/security/aws-security-agent-on-demand-penetration-testing-now-generally-available/)
- [Create a penetration test（公式ドキュメント）](https://docs.aws.amazon.com/securityagent/latest/userguide/perform-penetration-test.html)
- [認証情報の設定（Secrets Manager / TOTP / login prompt）](https://docs.aws.amazon.com/securityagent/latest/userguide/provide-testing-credentials.html)
- [マルチエージェント構成の解説](https://aws.amazon.com/blogs/security/inside-aws-security-agent-a-multi-agent-architecture-for-automated-penetration-testing/)
- [AWS ペネトレーションテストポリシー](https://aws.amazon.com/security/penetration-testing/)
