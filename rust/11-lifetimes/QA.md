# 11 - ライフタイム Q&A

日付: 2026-05-21

## Q1: `let r; { let x = 5; r = &x; } println!("{r}");` の例で、内側のブロックがなくて `let r; println!("{r}");` だとどうなる？

別のエラーになる。**ライフタイム以前に「未初期化変数の使用」エラーで弾かれる**。

```rust
let r;
println!("{r}");
```

このコードで起きる問題:

1. **型注釈が足りない** — `let r;` だけでは型推論の手がかりがなく `type annotations needed` (E0282)
2. **未初期化変数の使用** — 仮に型を書いても `use of possibly-uninitialized variable: r` (E0381)

### ライフタイムエラーとの違い

| パターン | エラー種別 | 問題 |
|---|---|---|
| `let r; { let x=5; r=&x; } println!("{r}");` | 借用チェックエラー (E0597) | `x` が `r` より先に解放される |
| `let r; println!("{r}");` | 未初期化エラー (E0381) | `r` に値が入っていない |

Rust の検査は段階的で、**「初期化されている」が先、「ライフタイムが妥当」が後**。最初の段でブロックされるので、ライフタイムの話までたどり着かない。

### Go との違い

Go の `var r *int` はゼロ値 `nil` で初期化されるので、`fmt.Println(r)` は `<nil>` を出力する（ランタイムでデリファレンスして初めて panic）。Rust は**ゼロ値という概念がなく**、未初期化のまま使うことをコンパイラが許さない。

```go
// Go — ゼロ値で初期化されるので OK (出力は <nil>)
var r *int
fmt.Println(r)
```

```rust
// Rust — そもそもコンパイルエラー
let r: &i32;
println!("{r}");  // error: use of possibly-uninitialized variable
```

これは Rust の「未定義動作を型システムで防ぐ」設計思想の現れ。借用チェッカーが活躍する前に、より基本的な定義済み性検査 (definite assignment) ですでに弾く。
