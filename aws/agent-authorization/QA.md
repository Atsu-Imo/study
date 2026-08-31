# AIエージェント認可 Q&A

記録日: 2026-08-25

## Q1: AIエージェントへの許可設定をしたことがない。どのように利用するもの？ IAMのポリシーとはまた別？

**結論: IAMとは別レイヤ。両方必要で、守っている境界が違う。**

### 何が新しい問題なのか

従来の認可は「**人間 or サービス** が **AWSリソース** に触れるか」だった。
エージェントで増えるのは「**LLMが選んだツールを、LLMが生成した引数で呼んでよいか**」という判断。

LLMは非決定論的で、ハルシネーションもプロンプトインジェクションも起こす。
だから **LLM自身を「信頼できないアクター」として扱い、外側に決定論的な関所を置く**。
これが AgentCore Policy（Cedar）のやっていること。

### 挟まる場所

```
[エージェント(LLM)] --MCP--> [AgentCore Gateway] --> [ツール実体(Lambda等)] --> [AWSリソース]
                                    ↑                                              ↑
                              Cedar/Dogwood が評価                             IAM が評価
```

Gateway は MCPサーバのエンドポイントで、エージェントとツールの間に必ず挟まる。
Policy Engine を ENFORCE でアタッチすると、`tools/call` も `tools/list` も全部そこを通る。
**デフォルト全拒否**、`permit` にマッチしたものだけ通す。

### IAM との違い

| | IAM | AgentCore Policy (Cedar) |
|---|---|---|
| 守る対象 | **AWS APIの呼び出し** | **ツールの呼び出し** |
| 粒度 | 「このロールは Gateway を呼べるか」 | 「このユーザは `send_email` を、**この引数で**呼べるか」 |
| principal | IAMロール / ユーザ | **JWTの sub**（エンドユーザ or エージェント） |
| 条件に使えるもの | リクエストコンテキスト、タグ | **LLMが生成したツール引数の中身** |
| 語彙 | AWSのAPIアクション | **アプリケーションのドメイン語彙** |
| 効く場所 | AWSサービス境界 | Gateway（アプリ層のMCP境界） |

**要点は「語彙」**。IAMのポリシーは AWS APIアクションの語彙しか持たないので、
「返金額が1000ドル未満なら」「事前に承認を取っていたら」は**原理的に書けない**。
逆に Cedar は AWSリソースへのアクセスを守らない。**上下に積む関係で、置き換えではない。**

> IAM = インフラ層の認可 / Cedar = アプリケーション層の認可

なお Cedar 自体は AWS 非依存のOSS言語なので、AgentCore を使わず自前のMCPサーバに埋め込むこともできる。

### 実際にどう書くか

Gateway が **JWT** と **MCPツール呼び出し** から Cedar リクエストを自動生成する:

| Cedar要素 | 実体 | 信頼度 |
|---|---|---|
| principal | JWTの `sub`。他クレームは tag として `principal.getTag("role")` で参照 | **信頼できる**（IdP署名済み） |
| action | ツール名 `<Target名>___<ツール名>` | LLMが選ぶ |
| resource | Gateway の ARN | 固定 |
| context.input | **ツール引数 = LLMが生成した値** | **信頼できない** |

```cedar
permit (
  principal is AgentCore::OAuthUser,
  action == AgentCore::Action::"RefundTarget___process_refund",
  resource == AgentCore::Gateway::"arn:aws:bedrock-agentcore:..."
)
when {
  principal.getTag("department") == "finance"  // 信頼できる側（JWT由来）
  && context.input.amount < 1000               // 信頼できない側（LLM生成）を制約
};
```

**この2つを1つの式の中で突き合わせられるのが本質。**
「経理部の人が使うエージェントなら、LLMが何を言おうと1000ドル以上の返金は通らない」を
プロンプトではなくコードの外側で保証できる。

### 実務上の勘所

- **Cedar は `resource` にワイルドカードを許さない** → Gateway ARN が必要 → デプロイ→ARN取得→ポリシー追加→再デプロイ、の**2段階デプロイ**になる
- **`tools/list` も評価対象**。partial evaluation により、呼ぶ権限のないツールは**そもそも一覧から消える**。LLMは存在を知らないものを呼べない（プロンプトインジェクション対策としてこれが効く）
- **デフォルト全拒否**なので、既存エージェントに後付けすると何も動かなくなる。いきなり ENFORCE にせず、監査モードで CloudWatch のポリシー決定ログを見てから切り替える
- `forbid` が `permit` に勝つ（forbid-wins）。ここは IAM と同じ感覚でよい

### Dogwood はこの上のどこに入るか

Cedar は**単発の判定**しかできない。「事前に承認を取ったか」「直近1時間の合計額」といった
**履歴に依存する条件**が書けない。それを足したのが Dogwood（`when temporal { ... }` 節）。

AgentCore 側では `x-amzn-bedrock-agentcore-policy-session-id` ヘッダを渡すと、
Gateway がセッションの蓄積アクション履歴を認可リクエストに含め、時相ルールがそれに対して評価される。
**セッションIDを渡さないと時相条件は評価材料を持たない**のが運用上の注意点。

詳細は `README.md` を参照。
