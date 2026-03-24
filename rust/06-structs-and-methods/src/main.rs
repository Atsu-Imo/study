// 06 - 構造体とメソッド
// struct、impl、メソッド、関連関数を学ぶ

fn main() {
    // === 構造体の定義とインスタンス化 ===
    let user = User {
        name: String::from("田中"),
        email: String::from("tanaka@example.com"),
        age: 30,
        active: true,
    };
    println!("ユーザー: {} ({}歳)", user.name, user.age);

    // === フィールド初期化の省略記法 ===
    // 変数名とフィールド名が同じなら省略できる（Goにはない）
    let name = String::from("佐藤");
    let email = String::from("sato@example.com");
    let user2 = User {
        name,  // name: name と同じ
        email, // email: email と同じ
        age: 25,
        active: true,
    };
    println!("ユーザー2: {} ({}歳)", user2.name, user2.age);

    // === 構造体更新構文 ===
    // 一部のフィールドだけ変えて新しいインスタンスを作る
    // Goにはない構文
    let user3 = User {
        name: String::from("鈴木"),
        ..user2 // 残りのフィールドは user2 から取る（ムーブが発生する）
    };
    println!("ユーザー3: {} ({}歳)", user3.name, user3.age);
    // println!("{}", user2.email); // エラー！email が user3 にムーブされた
    println!("user2.age は Copy なのでまだ使える: {}", user2.age);
    println!("user3: email={}, active={}", user3.email, user3.active);

    // === メソッド ===
    let rect = Rectangle {
        width: 30.0,
        height: 50.0,
    };
    println!("面積: {}", rect.area());
    println!("正方形？: {}", rect.is_square());
    println!("表示: {:?}", rect);

    // === &self と &mut self ===
    let mut rect2 = Rectangle {
        width: 10.0,
        height: 20.0,
    };
    println!("リサイズ前: {:?}", rect2);
    rect2.resize(15.0, 25.0); // &mut self で自身を変更
    println!("リサイズ後: {:?}", rect2);

    // === 関連関数（Associated Function） ===
    // Goにはない概念。:: で呼び出す
    let square = Rectangle::square(20.0);
    println!("正方形: {:?}, 面積: {}", square, square.area());

    // === タプル構造体 ===
    let origin = Point(0.0, 0.0);
    let target = Point(3.0, 4.0);
    println!("原点: ({}, {})", origin.0, origin.1);
    println!("距離: {:.2}", origin.distance_to(&target));

    // === ユニット構造体 ===
    // フィールドを持たない構造体。トレイト実装のマーカーとして使う
    let _marker = Marker;

    // === メソッドチェーン ===
    let rect3 = Rectangle::square(10.0).with_width(20.0).with_height(30.0);
    println!("チェーン結果: {:?}, 面積: {}", rect3, rect3.area());

    // === 演習 ===
    exercises();
}

// === 構造体の定義 ===
// Goの構造体とほぼ同じ
struct User {
    name: String, // &str ではなく String（所有する必要がある）
    email: String,
    age: u32,
    active: bool,
}
// Goとの違い: フィールド間はカンマ区切り（Goは改行）

// === derive で Debug を自動実装 ===
#[derive(Debug)]
struct Rectangle {
    width: f64,
    height: f64,
}

// === impl ブロック — メソッドを定義 ===
// Goのレシーバメソッドに相当
impl Rectangle {
    // メソッド: 第一引数が &self（Goの値レシーバに近い）
    fn area(&self) -> f64 {
        self.width * self.height
    }

    fn is_square(&self) -> bool {
        (self.width - self.height).abs() < f64::EPSILON
    }

    // &mut self: 自身を変更するメソッド（Goのポインタレシーバに近い）
    fn resize(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
    }

    // メソッドチェーン用: self を消費して新しいインスタンスを返す
    fn with_width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }

    fn with_height(mut self, height: f64) -> Self {
        self.height = height;
        self
    }

    // 関連関数: self を取らない（Goにはない概念）
    // 他の言語の「コンストラクタ」や「静的メソッド」に相当
    // :: で呼び出す（String::from と同じ）
    fn square(size: f64) -> Self {
        Self {
            width: size,
            height: size,
        }
    }

    fn can_hold(&self, other: &Rectangle) -> bool {
        self.width > other.width && self.height > other.height
    }
    fn scale(&mut self, factor: f64) {
        self.width *= factor;
        self.height *= factor;
    }
}

// === タプル構造体 ===
// フィールドに名前がなく、位置でアクセスする
struct Point(f64, f64);

impl Point {
    fn distance_to(&self, other: &Point) -> f64 {
        ((self.0 - other.0).powi(2) + (self.1 - other.1).powi(2)).sqrt()
    }
}

// === ユニット構造体 ===
struct Marker;

// ============================================================
// 演習
// ============================================================

// --- 演習1: 基礎 ---
// 本の情報を持つ構造体 Book を定義し、情報を表示するメソッドを実装しよう
//
#[derive(Debug)]
struct Book {
    title: String,
    author: String,
    pages: u32,
}

impl Book {
    // 関連関数: Book を作成する
    fn new(title: &str, author: &str, pages: u32) -> Self {
        Self {
            title: title.to_string(),
            author: author.to_string(),
            pages,
        }
    }

    // メソッド: "タイトル by 著者 (ページ数p)" の形式で返す
    fn summary(&self) -> String {
        format!("{} by {} ({}p)", self.title, self.author, self.pages)
    }
}

// --- 演習2: 応用 ---
// カウンターを構造体で実装しよう
//
struct Counter {
    count: i32,
}

impl Counter {
    fn new() -> Self {
        Self { count: 0 }
    }
    fn increment(&mut self) {
        self.count += 1;
    }
    fn decrement(&mut self) {
        self.count -= 1;
    }
    fn value(&self) -> i32 {
        self.count
    }
}

// --- 演習3: チャレンジ ---
// Rectangle に以下のメソッドを追加しよう
// - can_hold(&self, other: &Rectangle) -> bool
//   自分の中に other が収まるかを判定する（幅・高さ両方が大きければ true）
// - scale(&mut self, factor: f64)
//   幅と高さを factor 倍にする

fn exercises() {
    // 演習の動作確認をここに書く
    // 例:
    // let book = Book::new("Rustプログラミング", "著者名", 500);
    // println!("{}", book.summary());
    //
    // let mut counter = Counter::new();
    // counter.increment();
    // counter.increment();
    // counter.decrement();
    // println!("カウンター: {}", counter.value()); // 1
    //
    // let big = Rectangle { width: 30.0, height: 50.0 };
    // let small = Rectangle { width: 10.0, height: 20.0 };
    // println!("big は small を含む？: {}", big.can_hold(&small));
    // let mut r = Rectangle::square(10.0);
    // r.scale(2.0);
    // println!("スケール後: {:?}", r); // 20x20
}
