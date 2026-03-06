# 01-first-proof Q&A

2026-03-07

## Q1: `Theorem negb_involutive : forall b : bool, negb (negb b) = b.` が何を証明しようとしているのか分からない

`negb` は bool の否定関数（not bool の略）。

- `negb true = false`
- `negb false = true`

なので `negb (negb b) = b` は「boolを2回否定すると元に戻る」という性質。プログラミングでいう `!!b == b`。

`forall b : bool` がついているので、「true でも false でも成り立つ」ことを証明している。証明では `destruct b` で true/false に場合分けし、それぞれ `simpl` で計算して `reflexivity` で締める。

## Q2: `orb` とは何か

`or` + `b` (bool) で、boolの論理OR関数。`orb true _ = true`, `orb false b = b`。

Rocq標準ライブラリのbool関数の命名規則は末尾に `b` をつけてbool版であることを示す:

- `negb` = NOT (neg + b)
- `andb` = AND (and + b)
- `orb` = OR (or + b)

`and`, `or` は命題(Prop)の論理演算に使われているため、bool版は `andb`, `orb` と区別される。プログラミングでいう `&&`, `||` に相当。

## Q3: Rocqは実システムの検証に使えるのか？インテグレーションはできるか

できるし実例もある。主なアプローチ:

1. **Extraction**: Rocq内で書いた関数をOCaml/Haskell/Schemeに自動変換。証明済みの性質がそのまま保証される。CompCert（検証済みCコンパイラ）がこの方式。
2. **既存Cコードの検証**: VST (Verified Software Toolchain) で既存のCコードに仕様を書いてRocqで検証。
3. **プロトコル/設計のモデル検証**: 分散合意アルゴリズム等をモデル化して性質を証明。Amazon S3 ShardStoreの検証にも使われた。

現実的には「システム全体」ではなく「一番壊れたら困る部分だけ証明する」のが実用的。クリティカルなアルゴリズムをRocqで書いてExtraction、またはセキュリティ上重要なCコードだけVSTで検証、など。

## Q4: システムのコードをRocqに落とし込む必要があるのか？モデルと実装のギャップが心配

その懸念は正しく、形式検証の最大の実務的課題。解決アプローチは3つ:

1. **Extraction** — Rocqで実装して証明し、OCaml等に自動変換。モデルと実装が同一なのでギャップゼロ。ただしRocqで書ける範囲に限定。
2. **VST等の自動変換** — ツールがCソースコードからRocqの証明義務を自動生成。手動の落とし込み不要。ギャップ小。
3. **手動モデル化** — 一番危ない。モデルが実装と合っている保証がない。

現実的にはAかBを使うのが筋。手動モデル化は研究レベルでは使うが実務的にはリスクが大きい。

## Q5: `intros n m H` の意味と引数の順番の関係

`intros` は `forall` と `->` で束縛されたものを左から順に導入する。例えば `forall b c : bool, andb b c = orb b c -> b = c` の場合:

1. `b` : bool（forall の1番目）
2. `c` : bool（forall の2番目）
3. `H` : `andb b c = orb b c` という仮定（`->` の左側）

`->` の右側 (`b = c`) がゴールになる。`H` は自分で好きな名前をつけられる（慣習的に仮定は H, H1, H2 等）。

`->` はプログラミングの「if ... then ...」に近い。「もし andb b c = orb b c ならば b = c」を証明せよ、という意味。

## Q6: `->` がない定理は仮定がないということか

はい。`->` がなければ仮定なしで「無条件に成り立つ」という主張。`intros` で導入されるのは変数だけ。`->` がある場合は変数に加えて仮定 H も導入される。

## Q7: `simpl in H` で何が起きるのか、証明の進め方について

`simpl in H` は仮定 H の中の式を計算する。例えば `b = true, c = false` のケースで `H : andb true false = orb true false` なら:

- `andb true false` → `false`
- `orb true false` → `true`

結果 `H : false = true` になり、矛盾するので `discriminate` で証明完了。

証明の進め方は「先に全体を考えてから書く」のではなく、RocqIDEで1ステップずつ進めて右パネルのゴール・仮定を見ながら対話的に手探りで進めるのが基本。慣れてくるとパターンが見えて先読みできるようになる。
