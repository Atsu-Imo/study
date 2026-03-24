// 07 - 列挙型とパターンマッチ
// enum、match、Option、Result、? 演算子を学ぶ

fn main() {
    // === 基本的な enum ===
    // Goの iota + const に相当するが、はるかに強力
    let red = Color::Red;
    let green = Color::Green;
    let blue = Color::Blue;
    println!("赤: {:?}, 緑: {:?}, 青: {:?}", red, green, blue);

    // === match 式 ===
    // Goの switch に相当するが、網羅性チェックがある
    print_color(&red);
    print_color(&green);
    print_color(&blue);

    // === データを持つ enum（代数的データ型） ===
    // Goにはない概念。各バリアントが異なる型のデータを持てる
    let circle = Shape::Circle { radius: 5.0 };
    let rect = Shape::Rectangle {
        width: 10.0,
        height: 20.0,
    };
    let line = Shape::Line(0.0, 0.0, 3.0, 4.0); // タプル形式
    let dot = Shape::Point; // データなし

    println!("円の面積: {:.2}", area(&circle));
    println!("矩形の面積: {:.2}", area(&rect));
    println!("線: {:?}, 面積: {:.2}", line, area(&line));
    println!("点の面積: {:.2}", area(&dot));

    // === Option<T> ===
    // Goの nil / ポインタ返却の代替。null安全を型で保証する
    let numbers = vec![1, 3, 5, 7, 9];
    println!("最初の偶数: {:?}", find_even(&numbers)); // None
    println!("最初の偶数: {:?}", find_even(&[2, 4, 6])); // Some(2)

    // match で Option を処理
    match find_even(&numbers) {
        Some(n) => println!("見つかった: {}", n),
        None => println!("偶数なし"),
    }

    // if let — 1つのバリアントだけ処理したいとき
    if let Some(n) = find_even(&[10, 20]) {
        println!("if let で取得: {}", n);
    }

    // unwrap_or — デフォルト値を指定
    let value = find_even(&numbers).unwrap_or(0);
    println!("unwrap_or: {}", value);

    // map — Some の中身を変換
    let doubled = find_even(&[2, 4, 6]).map(|n| n * 2);
    println!("map で2倍: {:?}", doubled); // Some(4)

    // === Result<T, E> ===
    // Goの (value, error) 返却パターンの代替
    match parse_age("25") {
        Ok(age) => println!("年齢: {}", age),
        Err(e) => println!("エラー: {}", e),
    }
    match parse_age("abc") {
        Ok(age) => println!("年齢: {}", age),
        Err(e) => println!("エラー: {}", e),
    }
    match parse_age("-5") {
        Ok(age) => println!("年齢: {}", age),
        Err(e) => println!("エラー: {}", e),
    }

    // === ? 演算子 ===
    // Goの if err != nil { return err } を1文字で書ける
    match process_age("30") {
        Ok(msg) => println!("{}", msg),
        Err(e) => println!("処理失敗: {}", e),
    }
    match process_age("xyz") {
        Ok(msg) => println!("{}", msg),
        Err(e) => println!("処理失敗: {}", e),
    }

    // === while let ===
    let mut stack = vec![1, 2, 3];
    while let Some(top) = stack.pop() {
        println!("スタック: {}", top);
    }

    // === 演習 ===
    exercises();
}

// === 基本的な enum ===
#[derive(Debug)]
enum Color {
    Red,
    Green,
    Blue,
}

fn print_color(color: &Color) {
    // match は全バリアントを網羅する必要がある（網羅性チェック）
    // Goの switch にはこの保証がない
    match color {
        Color::Red => println!("赤色です"),
        Color::Green => println!("緑色です"),
        Color::Blue => println!("青色です"),
    }
}

// === データを持つ enum ===
// Goでは interface + 型アサーションで代用するパターン
#[derive(Debug)]
enum Shape {
    Circle { radius: f64 }, // 名前付きフィールド
    Rectangle { width: f64, height: f64 },
    Line(f64, f64, f64, f64), // タプル形式
    Point,                    // データなし
}

fn area(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle { radius } => std::f64::consts::PI * radius * radius,
        Shape::Rectangle { width, height } => width * height,
        // 線分の長さを計算（面積は0だが、フィールドは使える）
        Shape::Line(x1, y1, x2, y2) => {
            let _length = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
            0.0
        }
        Shape::Point => 0.0,
    }
}

// === Option<T> ===
// 標準ライブラリで定義:
// enum Option<T> {
//     Some(T),
//     None,
// }
fn find_even(numbers: &[i32]) -> Option<i32> {
    // イテレータで最初の偶数を探す
    // Goなら: return nil（ポインタ） or return 0, false（多値返却）
    numbers.iter().copied().find(|&n| n % 2 == 0)
}

// === Result<T, E> ===
// 標準ライブラリで定義:
// enum Result<T, E> {
//     Ok(T),
//     Err(E),
// }
fn parse_age(input: &str) -> Result<u32, String> {
    // Goなら: age, err := strconv.Atoi(input)
    let age: u32 = input
        .parse()
        .map_err(|_| format!("'{}'は数値ではありません", input))?;

    if age > 150 {
        return Err("年齢が大きすぎます".to_string());
    }

    Ok(age)
}

// === ? 演算子 ===
// エラーを自動的に返す。Goの if err != nil { return ..., err } に相当
fn process_age(input: &str) -> Result<String, String> {
    let age = parse_age(input)?; // エラーなら即 return Err(...)
                                 // Goなら:
                                 // age, err := parseAge(input)
                                 // if err != nil {
                                 //     return "", err
                                 // }
    Ok(format!(
        "{}歳は{}です",
        age,
        if age >= 18 { "成人" } else { "未成年" }
    ))
}

// ============================================================
// 演習
// ============================================================

// --- 演習1: 基礎 ---
// 信号機を表す enum を定義し、各状態に対応するメッセージを返す関数を実装しよう
//
#[derive(Debug)]
enum TrafficLight {
    Red,
    Yellow,
    Green,
}

impl TrafficLight {
    fn message(&self) -> &str {
        match self {
            TrafficLight::Red => "止まれ",
            TrafficLight::Yellow => "注意",
            TrafficLight::Green => "進め",
        }
    }
}

// --- 演習2: 応用 ---
// コマンドを表す enum を定義しよう。各バリアントは異なるデータを持つ
//
enum Command {
    Quit,                    // データなし
    Echo(String),            // 文字列
    Move { x: i32, y: i32 }, // 名前付きフィールド
    Color(u8, u8, u8),       // タプル（RGB）
}

fn execute(cmd: &Command) -> String {
    match cmd {
        Command::Quit => "終了します".to_string(),
        Command::Echo(text) => format!("エコー: {}", text),
        Command::Move { x, y } => format!("移動: x={}, y={}", x, y),
        Command::Color(r, g, b) => format!("色: R={}, G={}, B={}", r, g, b),
    }
}

// --- 演習3: チャレンジ ---
// 文字列を数値に変換し、その数値が正の偶数かどうかを判定する関数を実装しよう
// Result と Option を組み合わせる
//
fn check_even_positive(input: &str) -> Result<Option<i32>, String> {
    // 1. input を i32 にパース（失敗したら Err）
    // 2. 正の偶数なら Some(n)、そうでなければ None を Ok で返す
    input
        .parse::<i32>()
        .map_err(|_| format!("'{}'は数値ではありません", input))
        .map(|n| if n > 0 && n % 2 == 0 { Some(n) } else { None })
}
// 期待する動作:
// check_even_positive("4")  => Ok(Some(4))
// check_even_positive("3")  => Ok(None)      正だが奇数
// check_even_positive("-2") => Ok(None)       偶数だが負
// check_even_positive("abc") => Err(...)

fn exercises() {
    // 演習の動作確認をここに書く
    // 例:
    // let light = TrafficLight::Red;
    // println!("{:?}: {}", light, light.message());
    //
    // let cmd = Command::Move { x: 10, y: 20 };
    // println!("{}", execute(&cmd));
    //
    // println!("{:?}", check_even_positive("4"));   // Ok(Some(4))
    // println!("{:?}", check_even_positive("3"));   // Ok(None)
    // println!("{:?}", check_even_positive("abc")); // Err(...)
}
