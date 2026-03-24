# 03 - 帰納法による証明

## 概要

`destruct`（場合分け）では証明できない性質を、**帰納法 (induction)** で証明する方法を学ぶ。自然数に関する多くの性質は帰納法が必要になる。

## 背景知識 — プログラマの視点

### なぜ destruct では足りないのか

`destruct n` は n を `O` と `S n'` に分けるが、`n'` については何も言えない。bool は true/false の2つで全部だったが、nat は無限にある。

```coq
destruct n.
- (* n = O : 証明できる *)
- (* n = S n' : n' はまだ任意。行き詰まる *)
```

### 帰納法 = ドミノ倒し

高校数学の数学的帰納法と同じ：

1. **基底ケース**: n = 0 のとき成り立つ
2. **帰納ステップ**: n = k で成り立つ**と仮定して**、n = k+1 でも成り立つことを示す

プログラミングで例えると、再帰関数の正しさを示すのに近い：
- ベースケース（再帰の終了条件）が正しい
- 再帰呼び出しの結果が正しいと仮定して、全体が正しいことを示す

### destruct vs induction

| | destruct | induction |
|---|---|---|
| やること | 場合分け | 場合分け + 帰納法の仮定 |
| O のケース | ゴールの n が O になる | 同じ |
| S n' のケース | n が S n' になるだけ | **IHn': n' のとき成り立つ** という仮定が追加される |
| 使い所 | 有限の場合分けで済むとき | 「すべての n」について示すとき |

## コード解説

### induction の基本

```coq
Theorem plus_n_O : forall n : nat, n + 0 = n.
Proof.
  intro n.
  induction n as [| n' IHn'].
  - (* 基底ケース: n = O *)
    simpl. reflexivity.
  - (* 帰納ステップ: n = S n' *)
    (* IHn' : n' + 0 = n'  ← 帰納法の仮定！ *)
    simpl.           (* ゴール: S (n' + 0) = S n' *)
    rewrite IHn'.    (* n' + 0 を n' に書き換え *)
    reflexivity.
Qed.
```

`induction n as [| n' IHn']` の `[| n' IHn']` は名前の指定：
- `|` の左（空）: 基底ケース O には変数がない
- `|` の右: 帰納ステップで `n'` と仮定 `IHn'` を使う

`IHn'` は **Induction Hypothesis**（帰納法の仮定）の略。

### 補題を使う証明

```coq
Theorem plus_comm : forall a b : nat, a + b = b + a.
```

この証明では、先に証明した `plus_n_O` と `plus_n_Sm` を `rewrite` で使っている。大きな定理を小さな補題に分解して積み上げるのが Rocq の基本戦略。プログラミングで関数を小さく分割するのと同じ。

### どの変数について帰納法をかけるか

`+` の定義が**左側**をパターンマッチしているので、左側の変数について帰納法をかけるのが自然。

```coq
(* + の定義: 左側(n)を分解する *)
Fixpoint plus (n m : nat) : nat :=
  match n with
  | O => m
  | S n' => S (plus n' m)
  end.
```

`a + b` の性質を証明するとき、基本的に `a` について帰納法をかける。

## 新しいタクティク

| タクティク | 機能 |
|-----------|------|
| `induction n as [| n' IHn']` | n について帰納法。IHn' が帰納法の仮定 |
| `Abort.` | 証明を中断して捨てる（行き詰まったとき用） |

## 演習

### 演習1: 基礎

`plus_n_n_eq_double` を証明しよう:

```coq
Theorem plus_n_n_eq_double : forall n : nat, n + n = 2 * n.
```

ヒント: `n` について `induction`。`simpl` と `rewrite` で進める。途中で `plus_n_Sm`（本文で証明済み）を使う必要があるかもしれない。

### 演習2: 応用

`plus_Sn_m` を証明しよう:

```coq
Theorem plus_Sn_m : forall a b : nat, S a + b = S (a + b).
```

ヒント: 帰納法が必要かどうか、まず `simpl` してみよう。`+` の定義を思い出すと答えが見えるかもしれない。

### 演習3: チャレンジ

`mult_n_1` を証明しよう:

```coq
Theorem mult_n_1 : forall n : nat, n * 1 = n.
```

ヒント: `n` について `induction`。帰納ステップで `plus_n_O`（本文で証明済み）を補題として使う。

## まとめ

- **destruct** は場合分けだけ。**induction** は場合分け + 帰納法の仮定
- `induction n as [| n' IHn']` で帰納法。`IHn'` が「n' のとき成り立つ」という仮定
- 帰納法の流れ: 基底ケース → 帰納ステップで `rewrite IHn'`
- `+` の定義が左側を分解するので、左側の変数について帰納法をかけるのが自然
- 大きな定理は小さな**補題に分解**して積み上げる
