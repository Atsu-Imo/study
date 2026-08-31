# AWS学習 — Claude向け指示書

## 目的

使ったことがない、またはあまり理解していないAWSサービスについて学ぶ。

## ディレクトリ構成

- `ROADMAP.md` — 次に学ぶサービスの優先度リスト。新しいトピックを始めるときはここを参照・更新する
- `networking/` — ネットワーク基礎 + VPC + VPC間接続（PrivateLink, Transit Gateway, VPC Lattice）
- `mediaconvert/` — AWS MediaConvert + 映像基礎知識
- `aurora-dsql/` — Aurora DSQL（分散SQL）の概要・使い分け
- `security-agent/` — AWS Security Agent（自動ペネトレーションテスト）実施手順
- `compute-isolation/` — ECS / Lambda / Fargate のマルチテナント分離（Nitro, Firecracker, VPCオーバーレイ）
- `dynamodb/` — DynamoDB と分散KVストアの原理（コンシステントハッシング, クォーラム, パーティション）
- `agent-authorization/` — AIエージェントの認可（AgentCore Policy / Cedar / Dogwood）と IAM との役割分担

## 進め方

- サービスごとにサブディレクトリを作成して学習ノートやサンプルコードを管理する
- 概念の理解 → ハンズオン（可能なもの）の順で進める

## Q&A記録ルール

- ユーザーが学習中に質問してきたら、回答後にそのQ&Aを各サービスディレクトリの `QA.md` に追記する
- フォーマット:
  ```
  ## QN: (質問内容)
  (回答内容)
  ```
- 質問番号は通し番号 (Q1, Q2, ...)
- 日付はQ&Aファイルの先頭に記録する
