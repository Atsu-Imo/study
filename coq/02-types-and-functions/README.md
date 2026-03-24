# 02 - 帰納型と関数

## 概要

Rocqで自分の型を定義する方法（帰納型）と、パターンマッチ・再帰関数の書き方を学ぶ。標準ライブラリの `bool` や `nat` が実際にどう定義されているか理解する。

## 背景知識 — プログラマの視点

### 帰納型 = enum + 再帰

プログラミング言語のenumやunion typeに相当するが、**再帰的な定義**ができるのが特徴。

```go
// Goでは型を直接再帰的に定義できない
// Rocqの nat は概念的にはこういうもの:
type Nat struct {
    IsZero bool
    Pred   *Nat  // 前の数
}
```

### パターンマッチ = switch文

```coq
(* Rocq *)
match d with
| saturday => true
| sunday => true
| _ => false
end
```

```go
// Go
switch d {
case Saturday, Sunday:
    return true
default:
    return false
}
```

### Fixpoint = 再帰関数

Rocqでは再帰関数を `Fixpoint` で定義する。`Definition` は非再帰関数専用。

**重要な制約**: Rocqの再帰関数は**必ず停止しなければならない**。引数が構造的に小さくなっていることをコンパイラがチェックする。無限ループは書けない。

```coq
(* OK: n が S n' → n' と構造的に小さくなっている *)
Fixpoint evenb (n : nat) : bool :=
  match n with
  | O => true
  | S O => false
  | S (S n') => evenb n'
  end.

(* NG: 停止性が保証できない *)
(* Fixpoint loop (n : nat) : nat := loop n. *)
```

これはテストでは検出できない性質 — Rocqの型システムが**すべてのプログラムの停止を保証**する。

## コード解説

### Inductive（帰納型の定義）

```coq
Inductive day : Type :=
  | monday
  | tuesday
  | wednesday
  | thursday
  | friday
  | saturday
  | sunday.
```

`|` で区切られた各行が**コンストラクタ**（値を作る方法）。`day` 型の値はこの7つのどれか。

### nat（自然数）の定義

```coq
Inductive nat : Type :=
  | O            (* ゼロ *)
  | S (n : nat). (* n の次の数 *)
```

- `O` = 0
- `S O` = 1
- `S (S O)` = 2
- `S (S (S O))` = 3

これを**ペアノ数**という。すべての自然数を「ゼロ」と「+1」だけで表現する。Rocqは `3` と書くと自動的に `S (S (S O))` として扱う。

`S` は Successor（後続）の略。`S n` は「nの次の数」。

### Definition と Fixpoint

```coq
(* 非再帰関数 *)
Definition is_weekend (d : day) : bool := ...

(* 再帰関数 *)
Fixpoint evenb (n : nat) : bool := ...
```

### Example（テスト）

```coq
Example test_factorial1 : factorial 3 = 6.
Proof. simpl. reflexivity. Qed.
```

`Example` は `Theorem` と同じだが、「これはテストケースである」という意図を示す。`simpl. reflexivity.` で具体的な値の計算結果を確認できる。プログラミングのユニットテストに最も近い概念。

## 基本コマンド追加

| コマンド | 機能 | 備考 |
|---------|------|------|
| `Inductive` | 帰納型を定義 | enum + 再帰データ型 |
| `Definition` | 非再帰関数を定義 | |
| `Fixpoint` | 再帰関数を定義 | 停止性が必要 |
| `Example` | テストケース | Theorem と同じだが意図が異なる |
| `Compute` | 式を評価 | REPL的に使う |

## 演習

### 演習1: 基礎

`main.v` の `factorial` 関数の定義を読んで理解しよう。`test_factorial1` と `test_factorial2` が通ることを確認しよう。

さらに、以下のテストを追加して証明してみよう:

```coq
Example test_factorial3 : factorial 0 = 1.
```

### 演習2: 応用

`is_zero_correct` を証明しよう:

```coq
Theorem is_zero_correct : forall n : nat, is_zero n = true -> n = 0.
```

ヒント: `destruct n` で `O` と `S n'` に場合分け。`O` のケースは `reflexivity`。`S n'` のケースでは `simpl in H` してから `discriminate`。

### 演習3: チャレンジ

`S_n_not_zero` を証明しよう:

```coq
Theorem S_n_not_zero : forall n : nat, is_zero (S n) = false.
```

ヒント: 今までのタクティクだけで解ける。とてもシンプル。

## まとめ

- **Inductive** で帰納型（enum + 再帰データ型）を定義する
- **Definition** で非再帰関数、**Fixpoint** で再帰関数を定義する
- **nat** はゼロ (`O`) と後続 (`S`) で再帰的に定義されるペアノ数
- Rocqの再帰関数は**必ず停止する**ことが保証される
- **Example** でユニットテスト的に関数の動作を確認できる
- `match ... with ... end` でパターンマッチ（switch文に相当）
