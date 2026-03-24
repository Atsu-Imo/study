# 06 - 構造体とメソッド

## 概要

Rustの構造体（`struct`）とメソッド（`impl`）を学ぶ。Goの構造体+レシーバメソッドに近いが、所有権が絡む点と「関連関数」という概念が異なる。

## Goとの比較

| 概念 | Go | Rust |
|---|---|---|
| 構造体定義 | `type User struct { ... }` | `struct User { ... }` |
| インスタンス化 | `User{Name: "田中"}` | `User { name: String::from("田中") }` |
| メソッド | `func (u User) Name() string` | `fn name(&self) -> &str`（impl内） |
| ポインタレシーバ | `func (u *User) SetName(...)` | `fn set_name(&mut self, ...)` |
| コンストラクタ | `func NewUser() User`（慣習） | `fn new() -> Self`（関連関数） |
| 静的メソッド | なし | `fn square(size: f64) -> Self`（関連関数） |
| フィールド可視性 | 大文字/小文字 | `pub` キーワード |

### Goとの主な違い

```go
// Goの構造体とメソッド
type Rectangle struct {
    Width  float64
    Height float64
}

// 値レシーバ（コピーを受け取る）
func (r Rectangle) Area() float64 {
    return r.Width * r.Height
}

// ポインタレシーバ（変更可能）
func (r *Rectangle) Resize(w, h float64) {
    r.Width = w
    r.Height = h
}

// コンストラクタは慣習的に New... 関数
func NewSquare(size float64) Rectangle {
    return Rectangle{Width: size, Height: size}
}
```

```rust
struct Rectangle {
    width: f64,
    height: f64,
}

impl Rectangle {
    // &self: Goの値レシーバに近い（ただし借用なのでコピーではない）
    fn area(&self) -> f64 {
        self.width * self.height
    }

    // &mut self: Goのポインタレシーバに近い
    fn resize(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
    }

    // 関連関数: self を取らない。:: で呼び出す
    // Goでは関数として定義するが、Rustでは型に紐づく
    fn square(size: f64) -> Self {
        Self { width: size, height: size }
    }
}
```

主な違い:
- **メソッドは `impl` ブロック内で定義**（Goはどこでも定義可能）
- **`&self` は借用**（Goの値レシーバはコピー）
- **関連関数（`self` を取らない）** はGoにない概念。`String::from()` や `Vec::new()` がこれ
- **複数の `impl` ブロック**を持てる（トレイト実装で便利。08で学ぶ）

## コード解説

### 構造体の定義

```rust
struct User {
    name: String,   // フィールドは String（所有する）
    email: String,
    age: u32,
    active: bool,
}
```

```go
type User struct {
    Name   string  // Goでは string でOK（GCがある）
    Email  string
    Age    int
    Active bool
}
```

- Rustではフィールド間を**カンマ**で区切る（Goは改行）
- フィールドに `String` を使う（`&str` だとライフタイムが必要。11で学ぶ）

### フィールド初期化の省略記法

```rust
let name = String::from("田中");
let user = User {
    name,        // name: name の省略
    email: String::from("tanaka@example.com"),
    age: 30,
    active: true,
};
```

Goにはこの省略記法がない。JavaScriptのオブジェクトリテラルに似ている。

### 構造体更新構文

```rust
let user2 = User {
    name: String::from("佐藤"),
    ..user  // 残りは user から取る
};
// 注意: user.email は String なのでムーブされる
// user.age は u32（Copy）なので使える
```

Goにはこの構文がない。スプレッド演算子に似ているが、所有権のムーブが発生する点に注意。

### メソッドの self パターン

`&self` は省略記法で、展開すると `self: &Self` になる。`Self` は impl 対象の型のエイリアス。

```rust
impl Rectangle {
    // 省略形（実際のコードではこちらを使う）
    fn area(&self) -> f64 {
        self.width * self.height
    }

    fn resize(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
    }

    fn into_parts(self) -> (f64, f64) {
        (self.width, self.height)
    }

    // 上の3つは以下と同じ意味（展開形）:
    // fn area(self: &Rectangle) -> f64
    // fn resize(self: &mut Rectangle, width: f64, height: f64)
    // fn into_parts(self: Rectangle) -> (f64, f64)
}
```

| 省略形 | 展開形 | Go | 意味 |
|---|---|---|---|
| `&self` | `self: &Self` | 値レシーバ `(r Rect)` | 読み取りのみ（Rustは借用、Goはコピー） |
| `&mut self` | `self: &mut Self` | ポインタレシーバ `(r *Rect)` | 変更可能 |
| `self` | `self: Self` | なし | 所有権を消費（呼び出し後は使えない） |

### 関連関数

```rust
impl Rectangle {
    // self を取らない → メソッドではなく関連関数
    fn square(size: f64) -> Self {
        Self { width: size, height: size }
    }
}

// :: で呼び出す
let sq = Rectangle::square(10.0);
```

Goでは `NewRectangle()` のようなパッケージレベル関数で代用するが、Rustでは型に紐づく関連関数として定義する。`String::from()`、`Vec::new()` も関連関数。

### タプル構造体

```rust
struct Point(f64, f64);  // フィールド名なし

let p = Point(3.0, 4.0);
println!("{}", p.0);  // インデックスでアクセス
```

型に意味を持たせたいが名前付きフィールドは不要なときに使う。`struct Meters(f64)` のように単位を型で表現する「ニュータイプパターン」でよく使われる。

### メソッドチェーン

```rust
impl Rectangle {
    fn with_width(mut self, width: f64) -> Self {
        self.width = width;
        self  // 自身を返すことでチェーン可能にする
    }
}

let rect = Rectangle::square(10.0)
    .with_width(20.0)
    .with_height(30.0);
```

ビルダーパターンとしてRustでよく使われる。

## 演習

### 演習1: 基礎

本の情報を持つ構造体 `Book` を定義し、関連関数 `new` とメソッド `summary` を実装しよう。

```rust
#[derive(Debug)]
struct Book {
    title: String,
    author: String,
    pages: u32,
}

impl Book {
    fn new(title: &str, author: &str, pages: u32) -> Self {
        // title, author を String に変換して構造体を作る
    }

    fn summary(&self) -> String {
        // "タイトル by 著者 (ページ数p)" 形式で返す
    }
}
```

### 演習2: 応用

カウンターを構造体で実装しよう。`new`, `increment`, `decrement`, `value` メソッドを持つ。

```rust
struct Counter { count: i32 }
// increment, decrement は &mut self
// value は &self
```

### 演習3: チャレンジ

`Rectangle` に以下のメソッドを追加しよう。

- `can_hold(&self, other: &Rectangle) -> bool` — 自分の中に other が収まるか
- `scale(&mut self, factor: f64)` — 幅と高さを factor 倍にする

## まとめ

- 構造体は `struct` で定義。Goとほぼ同じだがカンマ区切り
- メソッドは **`impl` ブロック**内で定義する
- `&self`（読み取り）、`&mut self`（変更）、`self`（消費）の3パターン
- **関連関数**（`self` を取らない）は `::` で呼び出す。コンストラクタに使う
- フィールド省略記法と構造体更新構文（`..other`）がある
- タプル構造体はフィールド名なしの構造体。ニュータイプパターンに便利
- メソッドチェーンは `self` を返すことで実現する
