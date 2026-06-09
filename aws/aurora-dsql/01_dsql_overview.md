# Aurora DSQL 概要と使い分け

> Aurora DSQL は名前こそ「Aurora」だが、中身は従来の Aurora とは別アーキテクチャの分散SQL。
> PostgreSQL 16 互換だが機能はサブセット。

## 3つの位置づけ比較

| | **Aurora (Provisioned)** | **Aurora Serverless v2** | **Aurora DSQL** |
|---|---|---|---|
| アーキテクチャ | 1台のライター＋リードレプリカ | 同左（容量を自動伸縮するだけ） | **完全分散・リーダー無し** |
| 書き込みノード | 単一リージョンに1つ | 単一リージョンに1つ | **全リージョンで同時書き込み可（active-active）** |
| 同時実行制御 | 悲観的（ロック） | 悲観的（ロック） | **楽観的（OCC）** |
| スケール to ゼロ | 不可 | 可（復帰に約15秒） | 可（**コールドスタート無しで即時**）|
| キャパシティ単位 | インスタンスクラス（db.r6g等）| ACU（自動伸縮） | **完全サーバーレス（インスタンス概念が無い）** |
| 互換性 | PostgreSQL/MySQL フル | 同左 | **PostgreSQL 16 互換だが機能サブセット** |
| 可用性 | — | — | 単一リージョン 99.99% / マルチリージョン 99.999% |

## 「普通の Aurora」との本質的な違い

### 1. リーダーが無い（active-active 分散）
- 通常の Aurora は「ライター1台＋リードレプリカ」。ライターが落ちたら**フェイルオーバー**が走る。
- DSQL には**書き込みの中心ノードが存在せず**、複数リージョンが同時に書ける。
- コミット時の**レプリケーション遅延が無く**、どのエンドポイントから読んでも同じデータ（**強整合 / snapshot isolation**）。
- フェイルオーバーという概念自体が無く、障害時は健全なインフラへ自動ルーティング。

### 2. ロックではなく楽観的同時実行制御（OCC）— 一番の落とし穴
- 通常の Postgres は**行/テーブルをロック**して競合を防ぐ（悲観的）。
- DSQL は**ロックを取らず**、コミット時に競合を検出したら**即エラーを返す**。
- → **アプリ側でトランザクションのリトライ処理が必須**。接続リトライではなく「**競合したらやり直す**」ロジックが要る。
- 高競合（同じ行を多数が叩く）ワークロードでは**リトライのペナルティ**が出る。
- Go で言えば、DB が `serialization_failure` 相当を返してくるので自分でリトライループを巻く設計になる。

### 3. 機能制約（移行時に最初にぶつかる壁）
- **外部キー（FK）が無い** ← 顧客が最初に気づくギャップ。整合性はアプリ側で担保。
- **長時間トランザクション不可**、ストアドプロシージャ・トリガーに制約、**pgvector 無し**。
- **VACUUM や統計情報の仕組みが従来と違う** → プランナ挙動に依存したクエリは書き直しが要ることがある。
- マルチリージョンは**同一 Region Set 内のみ**（例: us-east-1 と eu-west-1 は組めない）。**大陸跨ぎ不可**。

## 使い分けの指針

**DSQL を選ぶ**
- マルチリージョンで**強整合の active-active** が欲しい（複数拠点で同時書き込み＆どこで読んでも同じ）
- **スパイキー／予測不能なトラフィック**で、瞬時スケール・ゼロスケールしたい
- microservice / serverless / イベント駆動の**新規アプリ**で、インフラ管理をゼロにしたい
- FK・トリガー・pgvector 等に**依存しない**設計にできる

**Aurora Serverless v2 を選ぶ**
- **単一リージョン**で十分
- **PostgreSQL/MySQL のフル機能**が要る（FK、トリガー、拡張、pgvector など）
- 既存アプリの移行で**アプリ改修を最小**にしたい（ロックベースのまま動かしたい）
- 15秒のコールドスタートが許容範囲

**普通の Provisioned Aurora を選ぶ**
- 負荷が**定常的・予測可能**で、容量を固定して**コスト/性能を読みたい**
- 最大性能やレプリカ構成を細かく握りたい

### 一言まとめ
- **DSQL** = 「DynamoDB の運用感（サーバーレス・無限スケール・マルチリージョン）を、SQL とACIDで実現したい」ときの選択肢。代償は **OCC リトライ前提＋機能制約**。
- **Serverless v2** = 「普通の Aurora の全機能はそのまま、容量だけ自動伸縮させたい」とき。

---

## 「外部キーが無い」の意味

### リレーションと外部キー制約は別物
- **リレーション（関連そのもの）= DSQL でも持てる。** テーブルを分けて片方の列にIDを入れ、JOINで引く、というデータモデル上の関連は普通に作れる。

```sql
CREATE TABLE users  ( id uuid PRIMARY KEY, name text );
CREATE TABLE orders ( id uuid PRIMARY KEY, user_id uuid, amount int );
-- JOIN も普通にできる
SELECT o.id, u.name FROM orders o JOIN users u ON u.id = o.user_id;
```

- **外部キー制約（FK constraint）= DSQL では書けない。** `user_id uuid REFERENCES users(id)` の `REFERENCES` が不可。
  通常の Postgres なら DB が自動で「存在しない user_id の INSERT を弾く」「子が残る親の DELETE を弾く（or CASCADE）」をやってくれる。**この"参照整合性をDBが保証する機能"が無い**、というのが「外部キーが無い」の意味。

### なぜ分散DBはFKを落とすのか — **ノードが分散しているから**
- FK制約のチェックは「**別テーブルの行が存在するか**を、書き込みのたびに確認・ロックする」必要がある。
- 通常の Postgres は**単一ノード**なのでこれがローカルに完結する。
- **DSQL はデータが複数ノード/リージョンに分散している。** `orders` への INSERT のたびに、別ノードにある `users` を見にいってロックする…となると、**分散ノード間の協調（クロスノードのロック・整合確認）が必要**になり、DSQL の売りである「リーダー無し・OCC・無限スケール」と真っ向から衝突する。
- なので**意図的に外している**（DynamoDB に FK が無いのと同じ理屈）。要望は認識しつつ優先度を下げている状況。

### 整合性はアプリ側で担保する
FKをDBに任せられない分、アプリの責任になる:
1. **アプリのコードで担保** — 注文作成前にユーザー存在を確認、トランザクション内でまとめて書く等
2. **孤児レコード（orphan）を許容する設計** — 親が消えても子が残る前提で、参照先が無いケースを読み取り側でハンドリング
3. **論理削除（soft delete）** — 物理削除せずフラグを立て、「親が突然消える」状況を作らない
4. **定期的なクリーンアップ** — バッチで孤児レコードを掃除

| | リレーション（関連・JOIN） | FK制約（DBによる整合性保証） |
|---|---|---|
| 普通の Aurora | ✅ | ✅ |
| Aurora DSQL | ✅ | ❌（アプリ側で担保） |

→ 失うのは「DBが自動で参照整合性を守ってくれる安全網」だけ。その代わり**スケールと分散書き込み**を得ている、というトレードオフ。

---

## 補足: JSON 対応（2026-05-04 追加）

DSQL は元々 JSON 型が無かったが、`json` 型（圧縮付き）が追加された。

- **格納できるのは `json`（テキストベース）のみ**。`jsonb` での格納は不可。
- ただし **`jsonb` は「クエリ処理時のランタイム型」として動作**し、PostgreSQL の JSONB 関数・演算子（マニュアル 9.16節）が**全て同一挙動**で使える（`->`, `->>`, `@>`, `jsonb_path_query` 等）。
- → 「**格納は json、処理は jsonb**」のハイブリッド。既存 Postgres アプリ/ORM が `json` 型依存でも無改修で載せやすい。

### 圧縮と 1 MiB 制限の関係（実務的に重要）
- 圧縮は**デフォルト有効**。`json` カラムの大きな値を INSERT/UPDATE 時に自動圧縮。
- DSQL の**行/値サイズ 1 MiB 上限は「圧縮後のサイズ」に適用**される。
- → 圧縮後が 1 MiB 未満なら、非圧縮で 1 MiB を大きく超えるペイロードも格納可能。
- 注意: `jsonb` 格納＋**GINインデックス**による高頻度な部分検索はできない。そういう用途は通常の Aurora PostgreSQL が向く。

---

### 参考リンク
- [What is Aurora DSQL（AWS docs）](https://docs.aws.amazon.com/aurora-dsql/latest/userguide/what-is-aurora-dsql.html)
- [Supported data types in Aurora DSQL](https://docs.aws.amazon.com/aurora-dsql/latest/userguide/working-with-postgresql-compatibility-supported-data-types.html)
- [Aurora DSQL now supports the JSON data type with compression（2026-05-04）](https://aws.amazon.com/about-aws/whats-new/2026/05/aurora-dsql-json-support/)
- [DoiT: DSQL vs Serverless v2 コスト比較](https://www.doit.com/blog/comparing-aurora-distributed-sql-vs-aurora-serverless-v2-a-practical-cost-analysis)
