# 01 - はじめての証明

## 概要

Rocq (旧Coq) の基本的な操作方法を学び、最初の証明を体験する。Rocqは「定理証明支援系」と呼ばれるツールで、数学の定理やプログラムの性質を対話的に証明できる。

## 背景知識 — プログラマの視点で見るRocq

### Rocqとは何か

普段のプログラミングでは、テストを書いて「いくつかの入力で正しく動くこと」を確認する。Rocqでは**すべての入力に対して正しいことを数学的に証明**できる。

```
テスト:   assert(sort([3,1,2]) == [1,2,3])  ← この入力では正しい
証明:     forall l, is_sorted(sort(l))        ← 任意の入力で正しい
```

### 対話的な証明

Coqの証明は、IDEの中で対話的に組み立てる。「今の証明の状態」を見ながら、**タクティク**（証明コマンド）を1つずつ適用して証明を完成させる。ゲームのターン制バトルのようなイメージ。

```
証明すべきこと（ゴール）が表示される
  ↓
タクティクを1つ適用
  ↓
ゴールが変化する（簡単になる or 分割される）
  ↓
最終的にゴールがなくなれば証明完了
```

### RocqIDEの使い方

RocqIDE (旧CoqIDE) で `.v` ファイルを開いて対話的に証明を進める：
- ツールバーの ▶ ボタン、またはショートカットで1ステップずつ進む/戻る
  - Ctrl+↓ : 1ステップ進む
  - Ctrl+↑ : 1ステップ戻る
  - Ctrl+→ : カーソル位置まで進む
  - Ctrl+End : 最後まで進む
- 画面右側に現在のゴール（証明すべきこと）が表示される
- 画面右下にCoqの応答メッセージが表示される

## コード解説

### Check と Compute

```coq
Check 0.          (* 0 の型を確認 → nat *)
Check true.       (* true の型を確認 → bool *)
Compute 1 + 2.    (* 式を評価 → 3 *)
```

プログラミングでいう `typeof` や REPL の評価に近い。Rocqの世界に慣れるための最初のステップ。

### 最初の証明: 1 + 1 = 2

```coq
Theorem one_plus_one : 1 + 1 = 2.
Proof.
  simpl.         (* 1 + 1 を計算して 2 にする *)
  reflexivity.   (* 2 = 2 なので証明完了 *)
Qed.
```

- `Theorem 名前 : 命題.` — 証明したい定理を宣言
- `Proof.` — 証明の開始
- `simpl.` — 式を簡約（計算）するタクティク
- `reflexivity.` — 左辺と右辺が同じなら証明完了するタクティク
- `Qed.` — 証明の終了（ラテン語で「証明終わり」）

### forall（全称量化）

```coq
Theorem plus_O_n : forall n : nat, 0 + n = n.
Proof.
  intro n.       (* 「任意の n」を具体的な n として導入 *)
  simpl.         (* 0 + n → n *)
  reflexivity.
Qed.
```

`forall n : nat, ...` は「すべての自然数 n について」という意味。プログラミングでいう「ジェネリクスの型パラメータ」に似た感覚だが、値レベルで使える。

### rewrite（書き換え）

```coq
Theorem plus_eq : forall n m : nat, n = m -> n + n = m + m.
Proof.
  intros n m H.  (* n, m, 仮定 H: n = m を導入 *)
  rewrite H.     (* H を使って n を m に書き換え → m + m = m + m *)
  reflexivity.
Qed.
```

`->` は「ならば」。`rewrite` は等式の仮定を使ってゴールを書き換えるタクティク。

### destruct（場合分け）

```coq
Theorem negb_involutive : forall b : bool, negb (negb b) = b.
Proof.
  intro b.
  destruct b.     (* b = true と b = false の2つに場合分け *)
  - simpl. reflexivity.  (* true の場合 *)
  - simpl. reflexivity.  (* false の場合 *)
Qed.
```

`destruct` はパターンマッチの証明版。bool なら true/false の2つ、nat なら 0/S n の2つに分かれる。

## 基本タクティク一覧

| タクティク | 機能 | プログラミングで例えると |
|---|---|---|
| `simpl` | 式を簡約（計算） | 定数畳み込み最適化 |
| `reflexivity` | 左辺 = 右辺で証明完了 | `assert(x == x)` |
| `intro x` | forall の変数を導入 | 関数の引数を受け取る |
| `intros x y H` | 複数を一度に導入 | 複数引数を受け取る |
| `rewrite H` | 仮定 H で書き換え | 変数の置換 |
| `destruct x` | 場合分け | switch/match文 |
| `simpl in H` | 仮定 H の中を簡約 | — |
| `discriminate` | 矛盾する等式（`true = false` 等）から証明完了 | `panic("unreachable")` |

## 演習

### 演習1: 基礎

`main.v` の `andb_true_l` の証明を読んで理解しよう。同じ要領で以下を証明してみよう:

```coq
Theorem orb_false_l : forall b : bool, orb false b = b.
```

### 演習2: 応用

以下の定理を証明してみよう。`destruct` を使う必要がある:

```coq
Theorem orb_commutative : forall a b : bool, orb a b = orb b a.
```

### 演習3: チャレンジ

以下の定理を証明してみよう。仮定の使い方（`rewrite`）と場合分け（`destruct`）の組み合わせが必要:

```coq
Theorem andb_eq_orb :
  forall b c : bool, andb b c = orb b c -> b = c.
```

ヒント: まず `b` で場合分けし、その後 `c` で場合分けする。矛盾するケースでは `simpl in H` で仮定を簡約し、`discriminate` タクティクを使う。

## まとめ

- Rocqは**対話的に証明を組み立てる**ツール
- `Theorem 名前 : 命題. Proof. ... Qed.` が基本構造
- **タクティク**を使ってゴールを変形し、最終的にゴールをなくす
- 基本タクティク: `simpl`, `reflexivity`, `intro(s)`, `rewrite`, `destruct`
- `Admitted` で証明を飛ばせる（未証明マーク。テストの `t.Skip()` のようなもの）
