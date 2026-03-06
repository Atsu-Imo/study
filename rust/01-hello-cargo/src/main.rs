// 01 - Hello, Cargo!
// Rustの最初のプログラムとCargoの基本的な使い方を学ぶ

fn main() {
    // println! はマクロ（関数ではない）。末尾の ! がマクロであることを示す
    println!("Hello, Cargo!");

    // フォーマット文字列: Goの fmt.Printf に似ているが、{} を使う
    let name = "Yamada";
    let date = "2026-03-06";
    println!("{name} edition {date} へようこそ！");

    // 複数の値を表示
    println!("1 + 2 = {}", 1 + 2);

    let n = 42;
    println!("10進: {n}");
    println!("2進: {n:b}");
    println!("16進: {n:x}");
    println!("右寄せ: {:>10}", "hello");
    println!("小数: {:.2}", 1.23456);

    // デバッグ表示: {:?} で Debug トレイトを使った表示ができる
    // Goの fmt.Printf("%v", ...) に近い
    println!("デバッグ表示: {:?}", (1, "hello", true));

    eprintln!("Error: Something went wrong!"); // 標準エラー出力に表示
}
