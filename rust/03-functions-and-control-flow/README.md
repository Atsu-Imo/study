# 03 - 関数と制御フロー

## 概要

Rustの関数定義、式ベースのreturn、`if`式、3種類のループ（`loop` / `while` / `for`）を学ぶ。Goとの最大の違いは**`if`やブロックが「式」として値を返せる**こと。

## Goとの比較

| 概念 | Go | Rust |
|---|---|---|
| 関数定義 | `func add(a, b int) int` | `fn add(a: i32, b: i32) -> i32` |
| 戻り値 | `return` 必須 | 最後の式がそのまま戻り値（`return` も使える） |
| if | 文（値を返さない） | **式**（値を返せる） |
| 三項演算子 | なし | なし（if式で代用） |
| 無限ループ | `for {}` | `loop {}` |
| 条件ループ | `for condition {}` | `while condition {}` |
| 範囲ループ | `for i, v := range slice` | `for v in &slice` / `for (i, v) in slice.iter().enumerate()` |
| ラベル付きbreak | `break Label` | `break 'label` |

### Goとの主な違い

- **式ベースの言語**: Rustでは `if`、`match`、ブロック `{}` が値を返す「式」。Goではこれらは全て「文」
- **Goは `for` のみ**: Goにはループが `for` しかない。Rustには `loop`、`while`、`for` の3種類がある
- **`loop` から値を返せる**: `break 値` でループの結果を返せる。Goにはこの機能がない
- **条件に括弧が不要**: `if x > 0 {}` のように括弧なしで書く（Goと同じ）
- **引数の型省略不可**: Goでは `func add(a, b int)` と型をまとめられるが、Rustでは各引数に型注釈が必須

## コード解説

### 関数定義と式ベースの戻り値

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b  // セミコロンなし → この式の値が戻り値
}
```

```go
func add(a, b int) int {
    return a + b  // return が必須
}
```

**重要**: セミコロンを付けると「文」になり、ブロックは `()` を返す。戻り値の型が `i32` なのに `()` を返すのでコンパイルエラーになる。

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b;  // セミコロンあり → () を返す → コンパイルエラー！
}
```

### if式

```rust
// if を式として使い、値を変数に束縛できる
let parity = if number % 2 == 0 { "偶数" } else { "奇数" };
```

```go
// Goでは if は文なので、事前に変数を用意する必要がある
var parity string
if number % 2 == 0 {
    parity = "偶数"
} else {
    parity = "奇数"
}
```

Rustには三項演算子（`? :`）がないが、if式で同じことができる。

### loop（無限ループ）

```rust
let result = loop {
    count += 1;
    if count == 5 {
        break count * 10;  // break で値を返す
    }
};
// result = 50
```

```go
// Goの for {} に相当。ただし break で値は返せない
for {
    count++
    if count == 5 {
        break
    }
}
```

### 範囲（Range）

```rust
for i in 1..5 {   // 1, 2, 3, 4（5を含まない）
for i in 1..=5 {   // 1, 2, 3, 4, 5（5を含む）
```

Goにはこの構文がなく、`for i := 1; i <= 5; i++` と書く。

### forでのイテレーション

```rust
let fruits = ["りんご", "みかん", "ぶどう"];

// 値だけ
for fruit in &fruits {
    println!("{fruit}");
}

// インデックス付き
for (i, fruit) in fruits.iter().enumerate() {
    println!("{i}: {fruit}");
}
```

```go
fruits := []string{"りんご", "みかん", "ぶどう"}
for i, fruit := range fruits {
    fmt.Printf("%d: %s\n", i, fruit)
}
```

### ラベル付きbreak

```rust
'outer: for i in 0..3 {
    for j in 0..3 {
        if i == 1 && j == 1 {
            break 'outer;  // 外側のループを抜ける
        }
    }
}
```

```go
Outer:
    for i := 0; i < 3; i++ {
        for j := 0; j < 3; j++ {
            if i == 1 && j == 1 {
                break Outer
            }
        }
    }
```

## 演習

### 演習1: 基礎

FizzBuzz を実装しよう。1から30まで繰り返し、3の倍数なら "Fizz"、5の倍数なら "Buzz"、両方の倍数なら "FizzBuzz"、それ以外は数字を表示する。

```rust
fn main() {
    for i in 1..=30 {
        // ヒント: if式を使って文字列を作り、println! で表示
    }
}
```

### 演習2: 応用

再帰関数でフィボナッチ数列の第n項を計算する関数 `fib(n: u32) -> u32` を作ろう。

```rust
fn fib(n: u32) -> u32 {
    // ヒント: fib(0)=0, fib(1)=1, fib(n)=fib(n-1)+fib(n-2)
}

fn main() {
    for i in 0..10 {
        println!("fib({i}) = {}", fib(i));
    }
}
```

### 演習3: チャレンジ

`loop` と `break 値` を使って、1から順に足していき合計が100を超えた時点の数を返すプログラムを書こう。

```rust
fn main() {
    let mut sum = 0;
    let n = loop {
        // ヒント: sum に数を足していき、100を超えたら break で返す
    };
    println!("合計が100を超えたのは {n} の時（合計: {sum}）");
}
```

## まとめ

- 関数は `fn 名前(引数: 型) -> 戻り値型` で定義。引数の型注釈は必須
- **最後の式**（セミコロンなし）がそのまま戻り値。早期リターンには `return` を使う
- **`if` は式**。値を返せるので三項演算子の代わりになる
- ループは3種類: `loop`（無限）、`while`（条件付き）、`for`（イテレーション）
- `loop` は `break 値` で値を返せる
- 範囲は `1..5`（exclusive）と `1..=5`（inclusive）
- ラベル付きbreakは `'label:` 記法を使う
