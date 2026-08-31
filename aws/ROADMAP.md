# AWS 学習ロードマップ

「アーキテクチャを理解して設計判断に活かす」ことを目的に、次に学ぶべきサービスを優先度順に整理したもの。

## 学習済み / 進行中

| ディレクトリ | トピック | 状態 |
|---|---|---|
| `mediaconvert/` | MediaConvert + 映像基礎 | 済 |
| `aurora-dsql/` | Aurora DSQL（分散SQL） | 済 |
| `security-agent/` | AWS Security Agent（自動ペネトレーションテスト） | 済 |
| `compute-isolation/` | ECS / Lambda / Fargate のマルチテナント分離 | 済 |
| `dynamodb/` | DynamoDB と分散KVストアの原理 | 済 |
| `rds/` | RDS（IAM DB認証、コネクション管理） | 済 |
| `agent-authorization/` | AIエージェントの認可（Cedar / Dogwood / AgentCore Policy） | 済 |
| `networking/` | ネットワーク基礎 + VPC + 上位レイヤ | **進行中** |

---

## 最優先

### 1. ネットワーキング（VPC / PrivateLink / Transit Gateway / VPC Lattice） ← 進行中

`compute-isolation/` で ENI と Hyperplane まで踏み込んだので、その真上のレイヤ。
PrivateLink は実質「Hyperplane 上の NLB を経由して他アカウントのサービスを自 VPC の ENI として生やす」仕組みで、compute-isolation の Q8〜Q11 の知識がそのまま効く。

マルチアカウント構成・SaaS 提供・オンプレ接続の設計判断がここで決まるため、実務インパクトが最も大きい。

→ `networking/README.md` にカリキュラム

### 2. IAM のポリシー評価ロジック

「IAM ポリシーの書き方」ではなく **評価順序** と **STS** に絞る。

- 明示 Deny > SCP > Resource Policy > Identity Policy > Permission Boundary の論理積
- `AssumeRole` / 信頼ポリシー / 外部ID
- OIDC フェデレーション（GitHub Actions から鍵なしで AWS を触る仕組み）
- ABAC（タグベースのアクセス制御）

クロスアカウント設計・CI/CD の権限設計・「なぜか通らない / 通ってしまう」のデバッグが自力でできるようになる。`security-agent/` の学習とも噛み合う。

### 3. メッセージング・ストリーミングの使い分け（SQS / SNS / EventBridge / Kinesis）

- 配信保証: at-least-once、FIFO の順序保証と重複排除、可視性タイムアウト、DLQ
- スケール単位の違い: キュー（SQS）vs シャード（Kinesis）
- EventBridge のルーティングとスキーマ、Pipes
- DynamoDB Streams → Lambda

非同期アーキテクチャの選定はここを知らないと勘で決めることになる。分散システムの原理として `dynamodb/` と繋がる領域。

---

## 次点

### 4. S3 の内部挙動

- プレフィックス単位のパーティション分割とスケーリング（なぜキー設計が性能に効くのか）
- 2020年に強一貫性がどう実現されたか
- S3 Express One Zone がなぜ速いのか（メタデータの持ち方が違う）
- ストレージクラスとライフサイクル、Intelligent-Tiering

ストレージ設計は後から変えづらいので早めが得。

### 5. Aurora（無印）のストレージ層

「redo log だけをストレージノードに送り、6コピーを3AZに配置、書き込みクォーラム 4/6・読み込み 3/6」というログ構造ストレージ。

`dynamodb/` のクォーラムを学んだ直後だと理解が速く、`aurora-dsql/` との比較（DSQL は楽観的並行制御 + Firecracker 上のトランザクション処理）が立体的になる。`rds/` の続きとしても自然。

### 6. KMS のエンベロープ暗号化

- データキー / CMK の2段構造とその理由
- Grants
- S3 / RDS / EBS の暗号化が実際どう繋がっているか

概念は小さいのに設計上の登場頻度が高い、コスパの良いトピック。

---

## 余力があれば

- **CloudFront + MediaPackage / MediaLive** — `mediaconvert/` をやったので配信側まで繋げると「1本のパイプライン」として絵が完成する
- **Route 53 のルーティングポリシーとヘルスチェック** — フェイルオーバー設計の要（ネットワーキング学習と一部重なる）
- **Organizations / Control Tower** — マルチアカウント戦略。設計に活かす目的なら遅かれ早かれ通る道
- **可観測性（CloudWatch / X-Ray / OpenTelemetry）** — 設計というより運用寄りだが、分散システムの障害切り分けには必須

---

## 優先順位の考え方

1 と 2 は「どのサービスを使うにも効く土台」、3 は個別サービスの選定の話。
土台を先に固めたほうが、後続の学習の吸収が速い。
