# AIエージェントの認可 — AgentCore Policy / Cedar / Dogwood

「エージェントがどのツールを、どんな条件で呼べるか」を**モデルの外側**で決定論的に制御する層についての学習ノート。

- **Cedar** — AWS製のオープンソース認可ポリシー言語（現在は CNCF 傘下）。単発リクエストの可否を判定する
- **Amazon Bedrock AgentCore Policy** — Cedar をエンジンにした、エージェントのツール呼び出しの認可サービス（re:Invent 2025 登場）
- **Dogwood** — 2026-08-06 に AWS がOSS公開した、Cedar に**時間・順序**の概念を足した拡張言語（Apache 2.0）

---

## なぜ必要か

エージェントの中心にあるLLMは**非決定論的**で、ハルシネーションもプロンプトインジェクションも起こす。
AWS のセキュリティブログはこれを明確に言い切っている:

> **LLMは、セキュリティの観点では「信頼できないアクター」として扱う**

したがって「プロンプトで禁止する」「ガードレールで検閲する」だけでは足りない。
**エージェントの外側に、決定論的に効く境界**を置く必要がある。それが Policy レイヤ。

| 手段 | 性質 | 保証 |
|---|---|---|
| システムプロンプトでの禁止 | LLM任せ | なし（injectionで剥がれる） |
| Bedrock Guardrails | 確率的（分類器） | 統計的 |
| **AgentCore Policy (Cedar)** | **決定論的（形式論理）** | **同じ入力なら常に同じ判定** |

---

## アーキテクチャ — どこで効くのか

```
[エージェント(LLM)]
      │  MCP (tools/list, tools/call)
      ▼
[AgentCore Gateway]  ← ここが強制ポイント (PEP)
      │  ├─ Policy Engine (Cedar/Dogwood) が全リクエストを評価
      │  └─ デフォルト全拒否。permit にマッチしたものだけ通す
      ▼
[ツール実体: Lambda / OpenAPI / Smithy ...]
      │  ここから先で初めて IAM が効く
      ▼
[DynamoDB / S3 / 外部API ...]
```

**Gateway = MCPサーバのエンドポイント**。エージェントとツールの間に必ず挟まる。
Policy Engine を ENFORCE モードで Gateway にアタッチすると、`tools/list` と `tools/call`
のすべてが Cedar 評価を通る。

`tools/list` も評価対象なのが効いていて、Cedar の **partial evaluation** により
**そもそも呼べないツールはツール一覧から消える**。LLMは存在を知らないものを呼べない。

---

## 認可リクエストの組み立て方

Gateway は「JWT」と「MCPツール呼び出し」の2つから Cedar のリクエストを自動生成する。

入力:
```json
// JWT (Cognito等のIdPが発行)
{ "sub": "1234-...", "username": "refund-agent", "role": "admin", "department": "finance" }

// MCP tools/call
{ "method": "tools/call",
  "params": { "name": "RefundTool___process_refund",
              "arguments": { "orderId": "12345", "amount": 450 } } }
```

生成される Cedar 認可リクエスト:
```json
{
  "principal": "AgentCore::OAuthUser::\"1234-...\"",       // JWTのsubクレーム
  "action":    "AgentCore::Action::\"RefundTool___process_refund\"", // ツール名
  "resource":  "AgentCore::Gateway::\"arn:aws:bedrock-agentcore:...\"", // Gateway ARN
  "context":   { "input": { "orderId": "12345", "amount": 450 } }      // ツール引数
}
```

**マッピングの要点**:

| Cedar | 実体 | 信頼度 |
|---|---|---|
| principal | JWTの `sub`。他のクレームは entity の **tag** として参照可 (`principal.getTag("role")`) | **信頼できる**（IdP署名済み） |
| action | ツール名（`<Target名>___<ツール名>`） | LLMが選ぶ |
| resource | Gateway の ARN | 固定 |
| context.input | **ツール引数 = LLMが生成した値** | **信頼できない** |

この「信頼できるprincipal属性」と「信頼できない引数」を**1つの式の中で突き合わせられる**のが
このモデルの本質。

```cedar
permit (
  principal is AgentCore::OAuthUser,
  action == AgentCore::Action::"ApplyBulkDiscount",
  resource
)
when {
  principal.getTag("customer_tier") == "Platinum"   // ← 信頼できる側
  && context.input.orderQuantity >= 50              // ← LLM生成の引数を制約
}
unless {
  context.input.productTypes.containsAny(["limited_edition", "seasonal_specials"])
};
```

評価セマンティクス: **デフォルト拒否** + **forbid が permit に勝つ**（forbid-wins）。IAM と同じ発想。

---

## IAM との関係 — 別物だが、二重で必要

**別のレイヤ。片方だけでは穴が開く。**

| | IAM | AgentCore Policy (Cedar) |
|---|---|---|
| 守る対象 | **AWS APIの呼び出し** | **ツールの呼び出し** |
| 粒度 | `bedrock-agentcore:InvokeGateway` を呼べるか | `send_email` ツールを、この引数で呼べるか |
| principal | IAMロール/ユーザ | **JWTのsub（エンドユーザやエージェント）** |
| 条件に使えるもの | リクエストコンテキスト、タグ | **LLMが生成したツール引数の中身** |
| 動く場所 | AWSサービス境界 | Gateway（アプリ層のMCP境界） |
| セッションの履歴 | 見ない | Dogwood なら**見る** |

具体的にどう並ぶか:

1. **IAM（入口）** — 誰がその Gateway を呼べるか / Gateway実行ロールが Lambda を呼べるか
2. **Gateway inbound auth** — JWT (OAuth) か IAM。ここでエンドユーザの身元が確定する
3. **Cedar / Dogwood** — そのユーザが、そのツールを、その引数で呼んでよいか ← **ここがエージェント固有**
4. **ツール実体の IAM** — Lambda が DynamoDB を触れるか

**IAM では 3 は書けない。** IAM のポリシーは「AWSのAPIアクション」の語彙しか持たず、
「`amount` が 1000 未満なら」「事前に承認を取っていたら」という**アプリケーションの語彙**を表現できない。
逆に Cedar は AWS リソースへのアクセスを守らない。**役割が違う。**

考え方としては、**IAM = インフラ層の認可 / Cedar = アプリケーション層の認可**。
Cedar 自体は AWS 非依存で、AgentCore を使わず自前の MCP サーバに組み込むこともできる。

---

## Dogwood — 時間軸の追加

### 解決する課題

1回のツール呼び出しは、**単体では完全に正当なのに、文脈の中では間違っている**ことがある。

| 状況 | Cedar（単発評価） | 本当に必要な判断 |
|---|---|---|
| 送金ツールを呼ぶ | 権限あり → 許可 | 「事前に人間の承認を取ったか？」 |
| $2,000 送金 | 上限内 → 許可 | 「直近1時間の**合計**が上限を超えないか？」 |
| 外部にメール送信 | 権限あり → 許可 | 「**直前に機密データを読んでいないか**？」 |

Cedar は「その瞬間」しか見ない。Dogwood は**同一セッション内の過去イベント履歴**を参照してから決める。

### 言語仕様

| 項目 | 内容 |
|---|---|
| ベース | Cedar。**構文的に正しいCedarポリシーはそのままDogwoodポリシー**（移行コストゼロ） |
| 追加構文 | `when temporal { ... }` 節。従来の `when { ... }` と併用する |
| 理論 | MFOTL（Metric First-Order Temporal Logic / 時間付き一階時相論理） |
| イベントモデル | ツール呼び出しを request / response のイベント列として扱う |
| アクションスキーマ | エージェントの **MCPツールマニフェストから自動生成**できる |
| ライセンス | Apache 2.0（`github.com/dogwood-policy/dogwood`） |

**標準ライブラリの4マクロ**（言語プリミティブではなく、コア時相演算子の上のマクロ）:

| マクロ | 意味 | 用途 |
|---|---|---|
| `formerly` | ウィンドウ内にそのイベントが起きたか | 人間の承認ゲート |
| `count_within` | ウィンドウ内の発生回数 | レート制限（1時間5回まで） |
| `count_distinct_within` | ウィンドウ内の異なる値の個数 | 送金先を1時間に3宛先まで |
| `sum_within` | ウィンドウ内の合計値 | 累計送金額 $5,000 上限 |

```cedar
permit(principal, action, resource)
when { context.input.amount < 1000 }
when formerly within 1h {
  Action::"Approve"::request{ approver: context.input.approver }
};
```

### AgentCore 側での有効化

`x-amzn-bedrock-agentcore-policy-session-id` ヘッダを付けると、Gateway が
そのセッションの蓄積アクション履歴を認可リクエストに含める。temporal ルールはこの履歴に対して評価される。
**セッションIDを渡さないと時相条件は評価材料を持たない**、という点が運用上の要。

---

## 注意点・限界

| 論点 | 内容 |
|---|---|
| **形式検証を失う** | Cedar最大の武器だった自動推論（矛盾検出・網羅性の機械証明）が temporal 条件には効かない |
| **並行性の罠** | AWS自身が指摘。レート制限を **response イベント基準**で書くと並行実行で破られる。$2,000×3本を同時に投げると「まだ完了していない」ため $5,000 上限をすり抜ける。→ **request イベント基準で書く** |
| ステートフル評価のコスト | イベント履歴の追跡が必要で、Cedarのステートレス評価より計算量・運用負荷が増える |
| 検証範囲は safety のみ | 「悪いことが起きない」のみ。liveness（「良いことがいずれ起きる」）は将来対応 |
| コミュニティ体制 | 現時点では**外部からの直接コントリビューションは受付停止**。段階的に開放予定 |
| Cedar側の制約 | ポリシーの `resource` にワイルドカード不可 → Gateway ARN を書く必要があり、**2段階デプロイ**（デプロイ→ARN取得→ポリシー追加→再デプロイ）になる |

**「Cedarの上位互換」ではなく「用途が違う別の刃」**と捉えるのが正確。
形式検証が要るところは Cedar、順序・累計が要るところだけ Dogwood。

---

## ハンズオン手順（AgentCore CLI）

```bash
npm install -g @aws/agentcore
agentcore create --name PolicyDemo --defaults && cd PolicyDemo

# 1. Gateway（= MCPサーバ）を作る。本番では --authorizer-type に IAM か JWT を指定
agentcore add gateway --name PolicyGateway --authorizer-type NONE --runtimes PolicyDemo

# 2. Lambdaをツールとして登録（tool schema JSON で引数の型を宣言）
agentcore add gateway-target --name RefundTarget --type lambda-function-arn \
  --lambda-arn <LAMBDA_ARN> --tool-schema-file refund_tools.json --gateway PolicyGateway

# 3. ポリシーエンジンを ENFORCE でアタッチ（この時点で全拒否になる）
agentcore add policy-engine --name RefundPolicyEngine \
  --attach-to-gateways PolicyGateway --attach-mode ENFORCE

# 4. デプロイ → Gateway ARN を取得（ワイルドカード不可のため先にARNが要る）
agentcore deploy && agentcore status

# 5. ポリシーを追加して再デプロイ
#    自然言語からCedarを生成させることもできる（--generate）
agentcore add policy --name RefundLimit --engine RefundPolicyEngine \
  --generate "Only allow refunds under 1000 dollars" --gateway PolicyGateway
agentcore deploy
```

動作確認（`$500` は通り、`$2000` は拒否される）:
```bash
curl -X POST <GATEWAY_URL>/mcp -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call",
       "params":{"name":"RefundTarget___process_refund","arguments":{"amount":500}}}'
```

**運用のコツ**: いきなり ENFORCE にせず、まず監査モードで CloudWatch のポリシー決定ログを眺め、
何が拒否されるかを見てから ENFORCE に切り替える。デフォルト全拒否なので、既存エージェントに
後付けすると何も動かなくなる。

---

## 参考

- [Introducing Dogwood: runtime verification for AI agents (AWS Open Source Blog)](https://aws.amazon.com/blogs/opensource/introducing-dogwood-runtime-verification-for-ai-agents/)
- [The Dogwood Guide（公式ドキュメント）](https://dogwood-policy.github.io/dogwood/index.html)
- [dogwood-policy/dogwood (GitHub)](https://github.com/dogwood-policy/dogwood)
- [Why Policy in Amazon Bedrock AgentCore chose Cedar (AWS Security Blog)](https://aws.amazon.com/blogs/security/why-policy-in-amazon-bedrock-agentcore-chose-cedar-for-securing-agentic-workflows/)
- [Getting started with Policy in AgentCore (AWS Docs)](https://docs.aws.amazon.com/bedrock-agentcore/latest/devguide/policy-getting-started.html)
- [Authorization flow (AWS Docs)](https://docs.aws.amazon.com/bedrock-agentcore/latest/devguide/policy-authorization-flow.html)
- [AWS Open-Sources Dogwood (InfoQ)](https://www.infoq.com/news/2026/08/aws-dogwood-agent-policy/)
