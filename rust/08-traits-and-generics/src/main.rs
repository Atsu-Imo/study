// 08 - トレイトとジェネリクス
// trait、ジェネリクス、トレイト境界、derive を学ぶ

fn main() {
    // === トレイトの基本 ===
    // Goのインターフェースに相当するが、明示的に impl する
    let circle = Circle { radius: 5.0 };
    let rect = Rectangle {
        width: 10.0,
        height: 20.0,
    };
    print_shape_info(&circle);
    print_shape_info(&rect);

    // === デフォルト実装 ===
    // Goのインターフェースにはない機能
    println!("円の説明: {}", circle.describe());
    println!("矩形の説明: {}", rect.describe());

    // === 複数のトレイトを実装 ===
    let circle2 = Circle { radius: 3.0 };
    println!("{}", circle2.to_json());
    println!("{}", rect.to_json());

    // === ジェネリクス ===
    // Goの [T any] に相当
    println!("最大値(i32): {}", max_value(3, 7));
    println!("最大値(f64): {}", max_value(3.5, 2.7));
    println!("最大値(str): {}", max_value("apple", "banana"));

    // === ジェネリックな構造体 ===
    let int_pair = Pair::new(1, 2);
    let str_pair = Pair::new("hello", "world");
    println!("int pair: {:?}, 最大: {}", int_pair, int_pair.max());
    println!("str pair: {:?}, 最大: {}", str_pair, str_pair.max());

    // === トレイトオブジェクト（動的ディスパッチ） ===
    // Goのインターフェース変数に最も近い
    let shapes: Vec<&dyn Shape> = vec![&circle, &rect];
    for shape in &shapes {
        println!("面積: {:.2}, 説明: {}", shape.area(), shape.describe());
    }

    // === derive マクロ ===
    let p1 = Point { x: 1, y: 2 };
    let p2 = Point { x: 1, y: 2 };
    let p3 = p1.clone(); // Clone
    println!("{:?}", p1); // Debug
    println!("p1 == p2: {}", p1 == p2); // PartialEq
    println!("p1 == p3: {}", p1 == p3);

    // === 標準トレイト: Display ===
    let c = Color {
        r: 255,
        g: 128,
        b: 0,
    };
    println!("Display: {}", c); // Display トレイト
    println!("Debug: {:?}", c); // Debug トレイト

    // === 演習 ===
    exercises();
}

// === トレイトの定義 ===
// Goの interface に相当
//
// Go:
//   type Shape interface {
//       Area() float64
//       Perimeter() float64
//   }
//
// Rust:
trait Shape {
    fn area(&self) -> f64;
    fn perimeter(&self) -> f64;

    // デフォルト実装 — Goのインターフェースにはない機能
    // 実装側で上書きも可能
    fn describe(&self) -> String {
        format!("面積: {:.2}, 周囲長: {:.2}", self.area(), self.perimeter())
    }
}

// === 構造体にトレイトを実装 ===
// Goとの違い: 明示的に impl TraitName for TypeName と書く
// Goはメソッドを定義すれば暗黙的に満たす（ダックタイピング）
struct Circle {
    radius: f64,
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

struct Rectangle {
    width: f64,
    height: f64,
}

impl Shape for Rectangle {
    fn area(&self) -> f64 {
        self.width * self.height
    }

    fn perimeter(&self) -> f64 {
        2.0 * (self.width + self.height)
    }

    // デフォルト実装を上書き
    fn describe(&self) -> String {
        format!(
            "{}x{} の矩形 (面積: {:.2})",
            self.width,
            self.height,
            self.area()
        )
    }
}

// === トレイト境界を使った関数 ===
// 「Shape を実装した任意の型」を受け取る
// Go: func printShapeInfo(s Shape) — インターフェース型を引数に取る
fn print_shape_info(shape: &impl Shape) {
    // &impl Shape は impl Shape: の省略記法
    // 完全な形: fn print_shape_info<T: Shape>(shape: &T)
    println!(
        "面積: {:.2}, 周囲長: {:.2}",
        shape.area(),
        shape.perimeter()
    );
}

// === 複数のトレイトを実装 ===
// 1つの型は複数のトレイトを実装できる（Goと同じ）
trait ToJson {
    fn to_json(&self) -> String;
}

impl ToJson for Circle {
    fn to_json(&self) -> String {
        format!(r#"{{"type": "circle", "radius": {}}}"#, self.radius)
    }
}

impl ToJson for Rectangle {
    fn to_json(&self) -> String {
        format!(
            r#"{{"type": "rectangle", "width": {}, "height": {}}}"#,
            self.width, self.height
        )
    }
}

// === ジェネリクス ===
// Goの [T any] に相当
// PartialOrd はトレイト境界（比較できる型に制限する）
//
// Go:
//   func Max[T constraints.Ordered](a, b T) T {
//       if a > b { return a }
//       return b
//   }
fn max_value<T: PartialOrd>(a: T, b: T) -> T {
    if a >= b {
        a
    } else {
        b
    }
}

// === ジェネリックな構造体 ===
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

// トレイト境界付きの impl
// PartialOrd + Copy を満たす T にだけ max() を提供する
impl<T: PartialOrd + Copy> Pair<T> {
    fn max(&self) -> T {
        if self.first >= self.second {
            self.first
        } else {
            self.second
        }
    }
}

// === トレイトオブジェクト（動的ディスパッチ） ===
// &dyn Shape — Goのインターフェース変数に最も近い
//
// 静的ディスパッチ（impl Shape）: コンパイル時に型が決まる。高速
// 動的ディスパッチ（dyn Shape）: 実行時に型が決まる。柔軟
//
// Go:
//   var shapes []Shape = []Shape{circle, rect}  ← 常に動的ディスパッチ
//
// Rust:
//   Vec<&dyn Shape>  ← 明示的に動的を選ぶ
//   impl Shape       ← デフォルトは静的

// === derive マクロ ===
// よく使うトレイトを自動実装する
// Go には相当する機能がない（手動実装が必要）
#[derive(Debug, Clone, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

// === Display トレイト ===
// println!("{}", x) で表示するためのトレイト
// GoのString() string メソッドに相当
// derive では自動実装できない — 手動で実装する必要がある
#[derive(Debug)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgb({}, {}, {})", self.r, self.g, self.b)
    }
}
// Go:
//   func (c Color) String() string {
//       return fmt.Sprintf("rgb(%d, %d, %d)", c.R, c.G, c.B)
//   }

// ============================================================
// 演習
// ============================================================

// --- 演習1: 基礎 ---
// Printable トレイトを定義し、Article 構造体に実装しよう
//
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
        println!("タイトル: {}", self.title);
        let body_preview: String = self.body.chars().take(20).collect();
        if self.body.chars().count() > 20 {
            println!("本文: {}...", body_preview);
        } else {
            println!("本文: {}", body_preview);
        }
    }
}

// --- 演習2: 応用 ---
// ジェネリックな関数 longest を実装しよう
// 2つのスライスのうち長い方を返す
//
fn longest<'a, T>(a: &'a [T], b: &'a [T]) -> &'a [T] {
    // ヒント: .len() で比較
    if a.len() >= b.len() {
        a
    } else {
        b
    }
}
// longest(&[1, 2, 3], &[4, 5]) => [1, 2, 3]

// --- 演習3: チャレンジ ---
// Summary トレイトを定義し、複数の型に実装しよう
// そして Vec<Box<dyn Summary>> で異なる型を1つのコレクションに入れよう
//
trait Summary {
    fn summarize(&self) -> String;
}

struct NewsArticle {
    title: String,
    author: String,
}
struct Tweet {
    username: String,
    content: String,
}

impl Summary for NewsArticle {
    fn summarize(&self) -> String {
        format!("{} by {}", self.title, self.author)
    }
}

impl Summary for Tweet {
    fn summarize(&self) -> String {
        format!("@{}: {}", self.username, self.content)
    }
}
//
// // それぞれに Summary を実装し、以下のように使う:
// // let items: Vec<Box<dyn Summary>> = vec![
// //     Box::new(NewsArticle { ... }),
// //     Box::new(Tweet { ... }),
// // ];
// // for item in &items {
// //     println!("{}", item.summarize());
// // }

fn exercises() {
    // 演習の動作確認をここに書く
    // 例:
    let article = Article {
        title: "Rust入門".into(),
        body: "Rustは安全で高速な...".into(),
    };
    article.print_info();

    let a = [1, 2, 3];
    let b = [4, 5];
    println!("長い方: {:?}", longest(&a, &b));

    let items: Vec<Box<dyn Summary>> = vec![
        Box::new(NewsArticle {
            title: "速報".into(),
            author: "記者".into(),
        }),
        Box::new(Tweet {
            username: "user1".into(),
            content: "Hello!".into(),
        }),
    ];
    for item in &items {
        println!("{}", item.summarize());
    }
}
