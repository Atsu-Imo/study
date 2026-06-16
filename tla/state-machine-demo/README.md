# 状態機械の安全性をモデル検査する (TLA+)

`coq/state-machine-safety/` と**同じ題材**（注文の状態機械「未払いで発送に到達しない」）を、
Rocq の手証明ではなく **TLA+ のモデル検査(TLC)** で検証するデモ。
「手で証明する vs ツールに全状態を総当たりさせる」の違いを体感するためのもの。

## TLA+ とは（ざっくり）

- Leslie Lamport 作の**仕様記述言語**。並行・分散システムの設計検証で使われる。
- 中核ツール **TLC** が、到達可能な状態を**片っ端から探索**して不変条件を検査する。
- Rocq と違い**証明を書かない**。不変条件を宣言して実行するだけ。
- バグがあると**反例トレース**（bad に至る具体的な手順）を出してくれるのが最大の利点。
- 弱点: 検査するのは**有限の状態空間**（＝モデルを小さく区切る）。「全入力の数学的証明」ではない。

## Rocq 版との対応

| Rocq (`main.v`) | TLA+ (`Order.tla`) |
|---|---|
| `Record State` の3フィールド | `VARIABLES paid, shipped, cancelled` |
| `Definition init` | `Init` |
| `step_pay` / `step_ship` / `step_cancel` | `Pay` / `Ship` / `Cancel` |
| `step`（許される遷移の総体） | `Next == Pay \/ Ship \/ Cancel` |
| `reachable` | `Spec == Init /\ [][Next]_vars` |
| `invariant` / `~ bad` / `safety` | `Safety`（TLC が検査する不変条件） |
| 手証明 `init_inv`+`step_preserves_inv`+`reachable_inv` | **不要**（TLC が全状態探索で確認） |

記法メモ:
- `'`（プライム）= 「次の状態での値」。Rocq の `mkState ...`（新状態を作る）に相当。
- `UNCHANGED <<x, y>>` = `x' = x /\ y' = y`。**書き忘れると TLC はその変数を任意の値とみなす**ので注意。

## 実行方法

### 1. TLC を取得（初回のみ。`.gitignore` 済みでコミットされない）

```bash
cd tla/state-machine-demo
curl -sSL -o tla2tools.jar \
  https://github.com/tlaplus/tlaplus/releases/latest/download/tla2tools.jar
```

Java が必要（`java -version` で確認。OpenJDK 11+ でOK）。

### 2. 正しい版を検査（成功する）

```bash
java -cp tla2tools.jar tlc2.TLC Order.tla
```

期待される出力（抜粋）:
```
Model checking completed. No error has been found.
10 states generated, 5 distinct states found, 0 states left on queue.
```
→ 到達可能な5状態を総当たりして Safety 成立を確認。

### 3. 壊れた版を検査（反例トレースが出る）

`OrderBug.tla` は `Ship` から `paid = TRUE` ガードを抜いたもの。

```bash
java -cp tla2tools.jar tlc2.TLC OrderBug.tla
```

期待される出力（抜粋）:
```
Error: Invariant Safety is violated.
State 1: <Initial predicate>   paid=FALSE, shipped=FALSE, cancelled=FALSE
State 2: <Ship ...>            paid=FALSE, shipped=TRUE,  cancelled=FALSE   ← 未払い発送！
```
→ 「init → Ship 1回 → bad」という**再現手順**を自動提示。

## まとめ

- 同じ状態機械でも、Rocq＝**手で証明**、TLA+＝**全状態を自動探索**。
- TLA+ は軽くて反例トレースが出るので、「設計に bad state が無いか／並行で壊れないか」を
  素早く確かめる用途で実務的（ワークフロー・認可モデルの設計検証など）。
- 一方「全入力を数学的に完全保証」したいクリティカルな純粋ロジックの核は Rocq。
- 正しさ検証のコスパ階段: 型で表現不能化 → プロパティテスト → **TLA+(モデル検査)** → Rocq(定理証明)。
