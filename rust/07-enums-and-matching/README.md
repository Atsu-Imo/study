# 07 - 列挙型とパターンマッチ

## 概要

Rustの列挙型（`enum`）とパターンマッチ（`match`）を学ぶ。Goの `const + iota` や `switch` に相当するが、Rustの enum はバリアントごとに異なるデータを持てる「代数的データ型」であり、はるかに強力。`Option` と `Result` は Rust のエラー処理と null 安全の要。

## Goとの比較

| 概念 | Go | Rust |
|---|---|---|
| 列挙型 | `const + iota` | `enum` |
| 分岐 | `switch` | `match`（網羅性チェックあり） |
| null/nil | `nil`（ポインタ、interface） | `Option<T>`（型で保証） |
| エラー処理 | `(value, error)` 多値返却 | `Result<T, E>` |
| エラー伝播 | `if err != nil { return err }` | `?` 演算子 |
| 型による分岐 | 型スイッチ `switch v := x.(type)` | `match` + enum バリアント |
| union型 | interface + 型アサーション | enum（コンパイル時に安全） |

### Goとの主な違い

```go
// Goの「列挙型」— 実態はただの整数定数
type Color int

const (
    Red Color = iota
    Green
    Blue
)

// switch に網羅性チェックはない
func printColor(c Color) {
    switch c {
    case Red:
        fmt.Println("赤")
    case Green:
        fmt.Println("緑")
    // Blue を忘れてもコンパイルエラーにならない！
    }
}
```

```rust
#[derive(Debug)]
enum Color {
    Red,
    Green,
    Blue,
}

fn print_color(c: &Color) {
    match c {
        Color::Red => println!("赤"),
        Color::Green => println!("緑"),
        // Blue を忘れるとコンパイルエラー！
        Color::Blue => println!("青"),
    }
}
```

主な違い:
- **網羅性チェック** — `match` は全バリアントを網羅する必要がある。バリアント追加時にハンドリング漏れをコンパイラが検出
- **データを持てる** — Goの enum はただの整数。Rustの enum は各バリアントが異なる型のデータを持てる
- **null安全** — Goは `nil` で実行時パニックの危険。Rustは `Option<T>` で型レベルで保証
- **エラー処理** — Goの `if err != nil` パターンを `?` 演算子で1文字に圧縮

## コード解説

### 基本的な enum

```rust
#[derive(Debug)]
enum Color {
    Red,
    Green,
    Blue,
}
```

Goの `const + iota` に相当するが、各バリアントは独立した値であり整数ではない。

### データを持つ enum（代数的データ型）

```rust
enum Shape {
    Circle { radius: f64 },              // 名前付きフィールド
    Rectangle { width: f64, height: f64 },
    Line(f64, f64, f64, f64),            // タプル形式
    Point,                               // データなし
}
```

Goでこれを再現するには interface + 複数の構造体が必要:

```go
type Shape interface {
    Area() float64
}

type Circle struct{ Radius float64 }
type Rectangle struct{ Width, Height float64 }

func (c Circle) Area() float64    { return math.Pi * c.Radius * c.Radius }
func (r Rectangle) Area() float64 { return r.Width * r.Height }
```

Rustの enum なら1つの型定義で完結し、`match` で安全に分岐できる。

### match 式

```rust
fn area(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle { radius } => std::f64::consts::PI * radius * radius,
        Shape::Rectangle { width, height } => width * height,
        Shape::Line(..) => 0.0,   // .. で残りを無視
        Shape::Point => 0.0,
    }
}
```

- `match` は**式**なので値を返せる（Goの `switch` は文）
- 全バリアントを網羅しないとコンパイルエラー
- `_` や `..` でワイルドカードマッチ

### Option\<T\>

標準ライブラリで定義された enum:

```rust
enum Option<T> {
    Some(T),
    None,
}
```

```rust
fn find_even(numbers: &[i32]) -> Option<i32> {
    for &n in numbers {
        if n % 2 == 0 {
            return Some(n);
        }
    }
    None
}
```

```go
// Goの場合: nil かポインタ、または多値返却
func findEven(numbers []int) (int, bool) {
    for _, n := range numbers {
        if n%2 == 0 {
            return n, true
        }
    }
    return 0, false
}
```

Option の便利メソッド:

| メソッド | 説明 | 例 |
|---|---|---|
| `unwrap()` | Some の中身を取得（None ならパニック） | `Some(5).unwrap()` → `5` |
| `unwrap_or(default)` | None のときデフォルト値 | `None.unwrap_or(0)` → `0` |
| `map(f)` | Some の中身を変換 | `Some(5).map(\|n\| n * 2)` → `Some(10)` |
| `and_then(f)` | Some の中身から新しい Option を返す | チェーン処理に |
| `is_some()` / `is_none()` | 判定 | `None.is_none()` → `true` |

### Result\<T, E\>

標準ライブラリで定義された enum:

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

```rust
fn parse_age(input: &str) -> Result<u32, String> {
    let age: u32 = input
        .parse()
        .map_err(|_| format!("'{}'は数値ではありません", input))?;
    if age > 150 {
        return Err("年齢が大きすぎます".to_string());
    }
    Ok(age)
}
```

```go
// Goの場合
func parseAge(input string) (uint32, error) {
    age, err := strconv.ParseUint(input, 10, 32)
    if err != nil {
        return 0, fmt.Errorf("'%s'は数値ではありません", input)
    }
    if age > 150 {
        return 0, errors.New("年齢が大きすぎます")
    }
    return uint32(age), nil
}
```

### ? 演算子

`?` は Result（または Option）のエラー伝播を簡潔にする:

```rust
fn process_age(input: &str) -> Result<String, String> {
    let age = parse_age(input)?;  // エラーなら即 return Err(...)
    Ok(format!("{}歳", age))
}
```

```go
// Goの場合 — 毎回 if err != nil が必要
func processAge(input string) (string, error) {
    age, err := parseAge(input)
    if err != nil {
        return "", err
    }
    return fmt.Sprintf("%d歳", age), nil
}
```

`?` は以下と同等:
```rust
let age = match parse_age(input) {
    Ok(v) => v,
    Err(e) => return Err(e),
};
```

### if let と while let

1つのバリアントだけ処理したいとき、`match` の代わりに使える:

```rust
// match で書く場合
match find_even(&numbers) {
    Some(n) => println!("見つかった: {}", n),
    None => {},  // 何もしない腕が必要
}

// if let で書く場合（簡潔）
if let Some(n) = find_even(&numbers) {
    println!("見つかった: {}", n);
}

// while let — None になるまで繰り返す
let mut stack = vec![1, 2, 3];
while let Some(top) = stack.pop() {
    println!("{}", top);  // 3, 2, 1
}
```

## 演習

### 演習1: 基礎

信号機を表す enum `TrafficLight` を定義し、各状態に対応するメッセージを返すメソッド `message` を実装しよう。

```rust
#[derive(Debug)]
enum TrafficLight {
    Red,
    Yellow,
    Green,
}

impl TrafficLight {
    fn message(&self) -> &str {
        // Red → "止まれ", Yellow → "注意", Green → "進め"
    }
}
```

### 演習2: 応用

コマンドを表す enum を定義しよう。各バリアントは異なるデータを持つ。

```rust
enum Command {
    Quit,
    Echo(String),
    Move { x: i32, y: i32 },
    Color(u8, u8, u8),
}

fn execute(cmd: &Command) -> String {
    // match で各コマンドを処理して結果文字列を返す
    // Quit → "終了します"
    // Echo(msg) → msg をそのまま返す
    // Move { x, y } → "(x, y) に移動"
    // Color(r, g, b) → "色: rgb(r, g, b)"
}
```

### 演習3: チャレンジ

文字列を数値に変換し、その数値が正の偶数かどうかを判定する関数を実装しよう。`Result` と `Option` を組み合わせる。

```rust
fn check_even_positive(input: &str) -> Result<Option<i32>, String> {
    // 1. input を i32 にパース（失敗したら Err）
    // 2. 正の偶数なら Some(n)、そうでなければ None を Ok で返す
}
// check_even_positive("4")   => Ok(Some(4))
// check_even_positive("3")   => Ok(None)
// check_even_positive("-2")  => Ok(None)
// check_even_positive("abc") => Err(...)
```

## まとめ

- `enum` はバリアントごとに異なるデータを持てる（代数的データ型）
- `match` は全バリアントの網羅が必要（コンパイラがチェック）
- `match` は**式**なので値を返せる
- `Option<T>` は null/nil の型安全な代替。`Some(T)` か `None`
- `Result<T, E>` は Go の `(value, error)` パターンの代替。`Ok(T)` か `Err(E)`
- `?` 演算子で `if err != nil { return err }` を1文字に圧縮
- `if let` / `while let` は1つのバリアントだけ処理する簡潔な構文
- `unwrap_or`, `map`, `and_then` で Option/Result を関数的に処理できる
