# Q&A — 状態機械の安全性証明

記録日: 2026-06-15

## Q1: `Record State` を flags(bool 3つ)で定義しているが、実務では状態を enum 的に扱うはず。この定義は enum と互換性があるのか？

厳密には**等価ではない**。enum は「ちょうど1つの状態」を表す**直和型**で、状態数はちょうど列挙した数だけ。一方 flags の `Record` は bool の**直積**なので 2³ = 8通りあり、`shipped=true & paid=false`（＝今回の `bad`）や `shipped & cancelled` のような**enum には存在しない不正状態も表現できてしまう**。

これは意図的な設計。

- **enum 流**（`Inductive Status := Pending | Paid | Shipped | Cancelled.`）は「make illegal states unrepresentable（不正状態を型で表現不可にする）」王道で、本来こちらが強い。不正状態が存在しないので証明すら要らない。
- ただし「不正状態が到達不能」を**定理として示すデモ**には、不正状態が**書ける**必要がある。だから flags にして `bad` を表現可能にし、「表現はできるが到達はできない」を `safety` で証明した。

| | enum（直和） | flags Record（直積） |
|---|---|---|
| 不正状態 | 表現できない（型で排除） | 表現できるが到達しないと証明 |
| 保証の出どころ | コンストラクション（型） | 定理（safety） |
| Go/DB での対応 | `type Status int` + iota | `paid_at`/`shipped_at`/`cancelled_at` の nullable カラム群 |

補足: DB スキーマでは nullable timestamp カラムの組み合わせ（flags 的）がむしろ典型で、その「組み合わせで不正状態が生まれうる」問題を到達不能と保証する、という意味で flags モデルも十分実務的。

**使い分け**: 排他的な1フェーズなら enum 一択（理想）。複数の独立属性の組み合わせなら flags + 不変条件の証明（今回の形）。

## Q2: `step_pay : forall s, cancelled s = false -> step s (mkState true (shipped s) (cancelled s))` の構文と各ステップの意味は？

`step` は `Inductive step : State -> State -> Prop` で定義した**2引数の関係**。`step a b` は「a から b への遷移が許される」という**命題**。`Inductive` なので「列挙したコンストラクタ（遷移ルール）でしか `step a b` を作れない」＝不正遷移は証明不能。

`step_pay` の分解:

- `forall s,` … 任意の状態 s についてのルール（s はパラメータ）。
- `cancelled s = false ->` … **前提条件（ガード）**。`->` は「ならば」。
- `step s (mkState true (shipped s) (cancelled s))` … **結論**。s から新状態への遷移が許される。

推論規則として読むと:
```
        cancelled s = false
  ──────────────────────────────────────  (step_pay)
  step s {paid:=true; shipped=元のまま; cancelled=元のまま}
```

新状態 `mkState true (shipped s) (cancelled s)` は、`mkState` の位置引数（定義順 paid, shipped, cancelled）に対応し、「s をコピーして **paid だけ true** にした新しい状態」。Coq はイミュータブルなので s を書き換えず新 State を作る（Go の「struct コピーして1フィールドだけ変更」と同じ）。

**Curry-Howard 視点**: コンストラクタは証明を作る関数。
`step_pay : (s : State) -> (cancelled s = false) -> step s (mkState true (shipped s) (cancelled s))`
→「状態 s と『cancelled s = false の証明』を渡すと『step s (...) の証明』が返る関数」。
使用例: `apply (step_pay init). reflexivity.`（s=init を入れ、残る前提 `cancelled init = false` を reflexivity で証明）。

他のルールも同じ読み方:
- `step_ship`: 前提が2つ（`paid s = true` と `cancelled s = false`）→ shipped だけ true に。この `paid s = true` ガードが「未払い発送」を原理的に作れなくする安全性の肝。
- `step_cancel`: 前提 `shipped s = false` → cancelled だけ true に。

## Q3: 「step は Prop を返す」のか「s と証明を受け取って step の証明を返す関数」なのか、どっち？ そもそも step は関数？

別々のものの話なので**両方正しい**。混同しやすいのは2つの異なる関数があるから。

- `step : State -> State -> Prop` … **型(Prop)を作る関数**。`step a b` は「a→b の遷移が許される」という**命題(型)**。
- `step_pay : forall s, cancelled s = false -> step s (...)` … その**命題の証明(値)を作る関数**（コンストラクタ）。

list との対応で理解すると早い:
| データの世界 | 証明の世界 |
|---|---|
| `list : Type -> Type` / `list nat` は型 | `step : State->State->Prop` / `step a b` は型(命題) |
| `cons` は `list nat` の値を作る関数 | `step_pay` は `step a b` の証明(値)を作る関数 |

さらに重要な気づき: **`step` は「呼ぶと true/false が返る計算する関数」ではない**。`Inductive step ... := ...` は「step とは何か」の**定義そのもの**で、列挙したコンストラクタが「`step a b` を成立させる唯一の手段」。成立は `Compute` で計算するのではなく**証明を構成**して示す。

bool 版（計算する関数）と対比:
```coq
Definition can_pay (s:State) : bool := negb (cancelled s).  (* Compute できる。値が返る *)
Inductive  step : State -> State -> Prop := ...             (* Compute できない。証明で示す *)
```
この「計算する(bool) vs 証明する(Prop)」がカリキュラム 07 のテーマ。

## Q4: `State -> State -> Prop` の `->` とは？ 続いたりする？ 1つのときは？ 戻り値が複数のときは？

`->` は**関数の矢印**で**右結合**。`A -> B -> C` は `A -> (B -> C)` と括られる。

- 手前が引数の型、**最後が戻り値の型**。
- 引数を増やす → **矢印を足す**。1つなら `A -> B`、2つなら `A -> B -> C`。
- 例: `invariant : State -> Prop`（1引数）、`step : State -> State -> Prop`（2引数）、`mkState : bool -> bool -> bool -> State`（3引数）。

**カリー化**: 関数は本質的に引数を1つずつ取る。だから部分適用できる:
```
step       : State -> State -> Prop
step init  :          State -> Prop   (* 1つ食べて矢印が1本減る *)
step init s2 :                 Prop
```
（`Check step.` `Check (step init).` で実際に確認した）

戻り値は常に1つ。複数返したいときは1つの型にまとめる（タプル `A * B`、`list`、Record など）。`mkState` がまさに「3つの bool → 1つの State」。

同じ `->` が命題どうしなら「ならば(含意)」になる（Curry-Howard により統一）。

## Q5: reachable も Inductive。`reach_init : reachable init` の意味、特に init とは？

`reachable : State -> Prop`。`reachable s` =「s は初期状態から許された遷移を辿って到達できる」という1引数の述語。

`reach_init : reachable init` は**引数も前提もない**コンストラクタ＝「`reachable init` の無条件の証明」。意味は **「出発点 init は（0ステップで）到達可能」** という**土台(base case)**。

`init` 限定なのが肝: もし `forall s, reachable s` にすると「全状態が到達可能」で意味が消える。起点を1点(init)に固定するから「到達可能＝init から辿れるものだけ」になる。

nat と同じ構造（土台 + 伸ばす規則）:
- `O : nat`（ゼロ＝土台） ↔ `reach_init : reachable init`（出発点＝土台）
- `S : nat -> nat`（次を作る） ↔ `reach_step : reachable s -> step s s' -> reachable s'`（遷移1回で到達範囲を広げる）

使うときは前提ゼロなので `apply reach_init.` だけで `reachable init` が証明できる。

## Q6: `/\` とは？

論理の**「かつ(AND)」＝連言(conjunction)**。`A /\ B` は「A かつ B」（`and A B` の記法）。`bad s := shipped s = true /\ paid s = false` は「発送済み**かつ**未払い」。

- `=` の方が `/\` より強く結合するので `(shipped s = true) /\ (paid s = false)` と読まれる。
- 命題(Prop)どうしを繋ぐもの。bool の値を繋ぐ「かつ」は別物で `&&`(`andb`)。
- 仲間: `\/`(または), `->`(ならば), `~`(でない)。
- 証明での扱い: ゴールが `A /\ B` なら `split` で2ゴールに分割。前提 `H : A /\ B` なら `destruct H as [HA HB]` で2つに分解。safety 冒頭の `destruct Hbad as [Hsh Hpa]` がこれ。

## Q7: `Lemma`、`unfold` とは？（init_inv の流れ）

- `Lemma` … 定理を宣言して証明を始めるキーワード。`Theorem`/`Example` 等と**機能は同じ（同義語）**。慣習で `Lemma`=部品(補題)、`Theorem`=主役の結論、と使い分けるだけ。
- `unfold X` … 定義された名前 `X` を中身に展開する。`unfold invariant, init.` のようにカンマで複数まとめて展開可。畳まれて中身が見えない状態を開いて操作するため。

init_inv のゴール変化:
```
invariant init
→(unfold invariant, init) shipped (mkState false false false) = true -> paid (...) = true
→(simpl)                  false = true -> false = true
→(intros H)               前提 H: false=true、ゴール false=true
→(discriminate H)         H が矛盾(false≠true)なので閉じる
```

## Q8: init は shipped=false なのに、invariant を「証明した」ことになるのか？（vacuous truth）

なる。よくある誤解は「init だから `shipped s = false -> paid s = false` を証明する」だが**違う**。`= true` は書き換わらない。変わるのは式 `shipped init`/`paid init` が計算されて `false` になるところだけ。ゴールは:
```coq
false = true -> false = true     (* ← false = false ではない！ *)
```
含意 `A -> B` が**偽になるのは `真 -> 偽` の1パターンだけ**。ここは前提 `false = true` が**偽**なので、含意は自動的に真。

直感: invariant =「発送したなら必ず支払い済み」という約束。init はまだ発送していない＝「発送済みなのに未払い」な反例を1つも出せない → 約束は破れない → 真。これが **vacuous truth（空虚な真）**。`discriminate` は「違反の前提 `false = true` が絶対成立しない」ことを使ってゴールを閉じている。

（「白いカラスはすべて飛べる」は白いカラスが1羽もいなければ真、と同じ理屈。）

## Q9: `apply` と `exact Hsh` とは？（step_preserves_inv のキャンセルケース `intros Hsh. apply Hinv. exact Hsh.`）

状況（intros Hsh の直後）:
```
Hinv : shipped s = true -> paid s = true
Hsh  : shipped s = true
ゴール: paid s = true
```

- `apply Hinv.` … **後ろ向き推論**。`Hinv` の**結論** `paid s = true` がゴールに一致するので、ゴールを `Hinv` の**前提** `shipped s = true` に差し替える（前提が複数なら複数の新ゴール）。
- `exact Hsh.` … `Hsh` の型がゴール `shipped s = true` と**完全一致**するので、その場で証明完了。

| タクティク | 条件 | 効果 |
|---|---|---|
| `exact H` | H の型がゴールと完全一致 | 即完了 |
| `apply H` | H の結論がゴールに一致 | ゴールを H の前提に差し替え |

Curry-Howard 視点: `Hinv` は関数（`A -> B` ＝ A の証明から B の証明を作る関数）。`apply Hinv` ＝ Hinv を呼ぶ → 引数 `shipped s = true` を要求、`exact Hsh` ＝ その引数を渡す。よって `apply Hinv. exact Hsh.` は **`exact (Hinv Hsh)` と同じ**（関数適用1個）。

## Q10: タクティク証明はかなり手続き的に見える

正しい直感。ただし**2層構造**になっている。
- **タクティクのスクリプト**（書くもの）＝手続き的：`intros. apply. exact.` …
- **証明項**（実際に保存されるもの）＝純粋関数：`Print init_inv.` `Print reachable_inv.` で見ると `intros`/`apply`/`induction` は消え、`fun ... => ...` や `reachable_ind ...` への関数適用だけが残る。

タクティクは「証明項を組み立てる作業手順」。だから `apply Hinv. exact Hsh.`（手続き2手）＝ `exact (Hinv Hsh)`（関数適用1個）。term mode で関数的に直接書くこともできる（Agda 流）が、Rocq はタクティク主流。トレードオフ: タクティク証明は書きやすいが、実行しないとゴールが見えず読みにくい。Jane Street 記事に繋がる点＝この手続き的な探索（証明の職人芸）こそ AI が自動化しやすく、宣言的な仕様（Theorem の中身）が人間の仕事。

## Q11: reachable_inv は「到達状態はどれも invariant を満たす」という理解で合っている？

合っている（言葉を精密化すると）。`reachable` は Inductive で**2つのコンストラクタしか持たない**ので、到達可能な状態は次の2通りでしか作れない:
1. `init` そのもの（reach_init）
2. 別の到達可能状態から step で1歩進んだ先（reach_step）

到達手段がこの2通りだけなので、(1) init で成立（init_inv）+ (2) どの step でも保たれる（step_preserves_inv）を示せば、`induction Hr` で全到達状態をカバーできる（nat の O/S 帰納法と同じ）。

注意: 「どの状態も」ではなく「どの**到達**状態も」。State は 2³=8通りあり bad も値としては存在する。reachable_inv は「到達可能な範囲に bad が入らない」を示すもの。

## Q12: reachable_inv が証明できれば safety は証明しなくてよいのでは？

「ほぼタダで出る系(corollary)」だが、証明は必要。理由:
1. **命題が別物**: `invariant s`（`shipped=true -> paid=true`、含意）と `~ bad s`（`~(shipped=true /\ paid=false)`、連言の否定）は論理的に同値でも Coq 上は別の項。同値だからと自動では繋がらず、橋渡しの数手が要る。
2. safety の証明は実際は短く、主役は `apply reachable_inv in Hr`。あとは paid=true と paid=false をぶつけて矛盾を出すだけ。

なぜ最初から `~ bad` で帰納法しないか: `invariant`（含意）は**帰納法と相性が良い**（各 step で保存を素直に示せる）。`~bad`（連言の否定）のままだと扱いにくい。検証の定石＝「帰納で回しやすい不変条件」を経由し、欲しい性質（safety）は系として導く。複雑な系では欲しい性質より**強い**不変条件が必要なこともある（invariant strengthening）。今回は invariant と ~bad が等価なので言い換えだけで済んだ。

## Q13: `intros s Hr Hbad.` と `destruct Hbad as [Hsh Hpa].` がわからない（Hr/Hbad とは、`as [Hsh Hpa]` とは）

`~ X` は `X -> False` の記法。なので `safety : forall s, reachable s -> ~ bad s` は実質 `forall s, reachable s -> bad s -> False`。`->` が2本あるので intros で3つ剥がせる:
- `s : State`
- `Hr : reachable s` … 「s は到達可能」という前提
- `Hbad : bad s` … 「s は bad」という前提

`H` は hypothesis の慣習接頭辞（ただの名前）。**否定 `~P` の証明は「P を仮定して False を導く」**のが定石なので、`Hbad : bad s` を仮定し、ゴールは `False` になる。

- `unfold bad in Hbad.` … Hbad 内の `bad` を定義 `shipped s = true /\ paid s = false` に展開（`in Hbad` ＝ ゴールでなく Hbad を対象に展開）。
- `destruct Hbad as [Hsh Hpa].` … 連言 `A /\ B`（内部は `conj 左 右`）を2つに分解し命名:
  - `Hsh : shipped s = true`
  - `Hpa : paid s = false`

intro パターン `[...]`: `/\`（コンストラクタ1個・引数2つ）→ `[Hsh Hpa]`（スペース区切り）。`\/`（コンストラクタ2個）なら `[H1 | H2]`（縦棒で場合分け）。

これで「発送済み(Hsh)かつ未払い(Hpa)」の反例材料が揃い、後段で reachable_inv から paid=true を引き出して Hpa(paid=false) と矛盾させ False を導く。
