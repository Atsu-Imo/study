# 08 - トレイトとジェネリクス

## 概要

Rustのトレイト（`trait`）とジェネリクスを学ぶ。トレイトはGoのインターフェースに近い概念だが、明示的に実装する点、デフォルト実装を持てる点、ジェネリクスのトレイト境界として使える点が異なる。

## Goとの比較

| 概念 | Go | Rust |
|---|---|---|
| インターフェース定義 | `type Shape interface { ... }` | `trait Shape { ... }` |
| 実装 | 暗黙的（ダックタイピング） | 明示的（`impl Shape for Circle`） |
| デフォルト実装 | なし | あり |
| ジェネリクス | `[T any]` (1.18+) | `<T>` |
| 型制約 | `[T constraints.Ordered]` | `<T: PartialOrd>`（トレイト境界） |
| インターフェース変数 | `var s Shape = circle` | `&dyn Shape` / `Box<dyn Shape>` |
| ディスパッチ | 常に動的 | 静的（`impl`）/ 動的（`dyn`）を選べる |
| 自動実装 | なし | `#[derive(...)]` |
| String表示 | `String() string` メソッド | `Display` トレイト |

### Goとの主な違い

```go
// Go — 暗黙的なインターフェース実装
type Shape interface {
    Area() float64
}

type Circle struct{ Radius float64 }

// メソッドを定義するだけで Shape を満たす（暗黙的）
func (c Circle) Area() float64 {
    return math.Pi * c.Radius * c.Radius
}

// インターフェース型の変数（常に動的ディスパッチ）
var s Shape = Circle{Radius: 5}
```

```rust
trait Shape {
    fn area(&self) -> f64;

    // デフォルト実装（Goにはない）
    fn describe(&self) -> String {
        format!("面積: {:.2}", self.area())
    }
}

struct Circle { radius: f64 }

// 明示的に impl する
impl Shape for Circle {
    fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }
    // describe() はデフォルト実装を使う
}

// 静的ディスパッチ（コンパイル時に型が決まる）
fn print_area(s: &impl Shape) { ... }

// 動的ディスパッチ（実行時に型が決まる、Goに近い）
let s: &dyn Shape = &Circle { radius: 5.0 };
```

主な違い:
- **明示的 vs 暗黙的** — Rustは `impl Trait for Type` と書く。Goはメソッドがあれば自動的に満たす
- **デフォルト実装** — トレイトにメソッドの既定実装を書ける。実装側で上書き可能
- **静的/動的ディスパッチの選択** — `impl Trait`（静的、高速）と `dyn Trait`（動的、柔軟）を選べる。Goは常に動的
- **derive** — `Debug`, `Clone`, `PartialEq` などを `#[derive(...)]` で自動実装

## コード解説

### トレイトの定義と実装

```rust
trait Shape {
    fn area(&self) -> f64;       // 必須メソッド
    fn perimeter(&self) -> f64;  // 必須メソッド

    // デフォルト実装（上書き可能）
    fn describe(&self) -> String {
        format!("面積: {:.2}", self.area())
    }
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }
    fn perimeter(&self) -> f64 {
        2.0 * std::f64::consts::PI * self.radius
    }
    // describe() はデフォルト実装を使う
}
```

Goではメソッドを持つだけで暗黙的にインターフェースを満たすが、Rustでは `impl Trait for Type` を書き忘れるとコンパイルエラー。これにより「意図せずインターフェースを満たしてしまう」事故を防げる。

### トレイト境界（型制約）

```rust
// 省略記法
fn print_info(shape: &impl Shape) { ... }

// 完全な記法（上と同じ意味）
fn print_info<T: Shape>(shape: &T) { ... }

// 複数のトレイト境界
fn process<T: Shape + ToJson>(item: &T) { ... }

// where 句（境界が長くなるとき）
fn process<T>(item: &T)
where
    T: Shape + ToJson + std::fmt::Debug,
{ ... }
```

```go
// Go の型制約
func PrintInfo[T Shape](s T) { ... }

// 複数の制約
type ShapeAndJson interface {
    Shape
    ToJson
}
func Process[T ShapeAndJson](item T) { ... }
```

### ジェネリクス

```rust
fn max_value<T: PartialOrd>(a: T, b: T) -> T {
    if a >= b { a } else { b }
}
```

```go
func Max[T constraints.Ordered](a, b T) T {
    if a >= b { return a }
    return b
}
```

構文は異なるが概念は同じ。Rustの `PartialOrd` がGoの `constraints.Ordered` に相当。

### ジェネリックな構造体

```rust
#[derive(Debug)]
struct Pair<T> {
    first: T,
    second: T,
}

impl<T> Pair<T> {
    fn new(first: T, second: T) -> Self {
        Self { first, second }
    }
}

// 特定のトレイトを満たす T にだけメソッドを追加
impl<T: PartialOrd + Copy> Pair<T> {
    fn max(&self) -> T {
        if self.first >= self.second { self.first } else { self.second }
    }
}
```

`Pair<i32>` には `max()` が使えるが、`Pair<Vec<i32>>` には使えない（`Copy` を満たさないため）。Goのジェネリクスでは同様の制約を型セットで表現する。

### 静的ディスパッチ vs 動的ディスパッチ

```rust
// 静的ディスパッチ — コンパイル時に具体的な型が決まる
// コンパイラが型ごとに専用コードを生成する（モノモーフィゼーション）
fn print_area(s: &impl Shape) {
    println!("{:.2}", s.area());
}

// 動的ディスパッチ — 実行時に vtable を通じてメソッドを呼ぶ
// Goのインターフェースと同じ仕組み
let shapes: Vec<&dyn Shape> = vec![&circle, &rect];
```

| | 静的（`impl Trait`） | 動的（`dyn Trait`） |
|---|---|---|
| 速度 | 高速（インライン化可能） | vtable のオーバーヘッド |
| バイナリサイズ | 型ごとにコード生成 | 共通コード |
| 柔軟性 | 1つの型しか受けられない | 異なる型を混在できる |
| Go | — | こちらに相当 |

### derive マクロ

```rust
#[derive(Debug, Clone, PartialEq)]
struct Point { x: i32, y: i32 }
```

| derive | 機能 | Go相当 |
|---|---|---|
| `Debug` | `{:?}` でデバッグ表示 | `%+v` |
| `Clone` | `.clone()` で明示的コピー | 値型は自動コピー |
| `Copy` | 暗黙的コピー（小さい型向け） | 値型は自動コピー |
| `PartialEq` | `==` で比較 | `==`（構造体は自動比較可能） |
| `Hash` | HashMap のキーに使える | マップのキーは `comparable` |
| `Default` | `Default::default()` でゼロ値 | ゼロ値は自動 |

### Display トレイト

```rust
impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgb({}, {}, {})", self.r, self.g, self.b)
    }
}
```

```go
func (c Color) String() string {
    return fmt.Sprintf("rgb(%d, %d, %d)", c.R, c.G, c.B)
}
```

`Display` は `derive` で自動実装できない。表示形式はプログラマが決める必要があるため。

## 演習

### 演習1: 基礎

`Printable` トレイトを定義し、`Article` 構造体に実装しよう。

```rust
trait Printable {
    fn print_info(&self);
}

struct Article {
    title: String,
    body: String,
}

impl Printable for Article {
    fn print_info(&self) {
        // "タイトル: {title}" と "本文: {bodyの先頭20文字}..." を表示
    }
}
```

### 演習2: 応用

ジェネリックな関数 `longest` を実装しよう。2つのスライスのうち長い方を返す。

```rust
fn longest<'a, T>(a: &'a [T], b: &'a [T]) -> &'a [T] {
    // ヒント: .len() で比較
}
// longest(&[1, 2, 3], &[4, 5]) => [1, 2, 3]
```

### 演習3: チャレンジ

`Summary` トレイトを定義し、`NewsArticle` と `Tweet` に実装しよう。`Vec<Box<dyn Summary>>` で異なる型を1つのコレクションに入れる。

```rust
trait Summary {
    fn summarize(&self) -> String;
}

struct NewsArticle { title: String, author: String }
struct Tweet { username: String, content: String }

// それぞれに Summary を実装し、Vec<Box<dyn Summary>> に入れて
// ループで summarize() を呼ぶ
```

## まとめ

- **トレイト** はGoのインターフェースに相当。ただし明示的に `impl` する
- **デフォルト実装** を持てる（Goにはない）
- **トレイト境界** `<T: Trait>` でジェネリクスの型を制約する
- **静的ディスパッチ**（`impl Trait`）と**動的ディスパッチ**（`dyn Trait`）を選べる
- Goは常に動的ディスパッチ。Rustはデフォルトが静的で高速
- `#[derive(...)]` でよく使うトレイトを自動実装できる
- `Display` トレイトはGoの `String()` メソッドに相当
- 複数の `impl` ブロックで異なるトレイトを1つの型に実装できる
