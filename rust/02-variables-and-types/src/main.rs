// 02 - 変数と型
// let/mut、型推論、シャドウイング、基本型、タプル、配列を学ぶ

fn main() {
    // === 不変変数（デフォルト） ===
    // Rustの変数はデフォルトで不変（immutable）
    // Goは常にmutable。Rustでは明示的に mut を付ける必要がある
    let x = 5;
    println!("x = {x}");
    // x = 10; // コンパイルエラー: cannot assign twice to immutable variable

    // === 可変変数（mut） ===
    let mut y = 10;
    println!("y = {y}");
    y = 20;
    println!("y（変更後） = {y}");

    // === 型推論 ===
    // Goと同様、Rustも型推論を持つ
    let a = 42;          // i32 に推論される
    let b = 2.71;        // f64 に推論される
    let c = true;        // bool
    let d = 'R';         // char（シングルクォート = 1文字）
    let e = "hello";     // &str（文字列スライス）
    println!("a={a}, b={b}, c={c}, d={d}, e={e}");

    // === 明示的な型注釈 ===
    // Goの var x int = 5 に相当
    let f: i64 = 100;
    let g: f32 = 2.5;
    println!("f={f}, g={g}");

    // === 整数型の一覧 ===
    // i8, i16, i32, i64, i128, isize（符号付き）
    // u8, u16, u32, u64, u128, usize（符号なし）
    // Goの int8, int16, ... に相当。isizeはGoの int に近い
    let unsigned: u8 = 255;
    let signed: i8 = -128;
    println!("u8最大値={unsigned}, i8最小値={signed}");

    // === シャドウイング ===
    // 同じ名前で新しい変数を宣言できる（Goにはない概念）
    let s = "123";           // &str型
    let s = s.len();         // usize型に変わる！（型の変換もOK）
    let s = s * 2;           // 値の計算
    println!("シャドウイング結果: {s}"); // 6

    // mutとの違い: シャドウイングは新しい変数を作る（型も変えられる）
    // mutは同じ変数の値を変えるだけ（型は変えられない）

    // === 定数 ===
    // const は Goの const に近いが、型注釈が必須
    const MAX_POINTS: u32 = 100_000;
    println!("定数: {MAX_POINTS}");
    // 数値リテラルの _ は桁区切り（可読性のため。Goの 100_000 と同じ）

    // === タプル ===
    // 異なる型の値をまとめる。Goにはタプル型がない
    let tup: (i32, f64, bool) = (500, 6.4, true);
    let (tx, ty, tz) = tup;  // 分割代入（destructuring）
    println!("タプル分割: tx={tx}, ty={ty}, tz={tz}");
    println!("インデックスアクセス: {}, {}, {}", tup.0, tup.1, tup.2);

    // === 配列 ===
    // 固定長。Goの配列 [5]int に相当
    let arr = [1, 2, 3, 4, 5];
    println!("配列: {:?}", arr);
    println!("配列の長さ: {}", arr.len());
    println!("最初の要素: {}", arr[0]);

    // 同じ値で初期化
    let zeros = [0; 5]; // [0, 0, 0, 0, 0]
    println!("ゼロ配列: {:?}", zeros);

    // === ユニット型 ===
    // () は「値がない」ことを表す型。Goにはない概念
    // 戻り値のない関数は暗黙的に () を返す
    let unit: () = ();
    println!("ユニット型: {:?}", unit);
}
