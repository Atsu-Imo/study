// 05 - 借用（Borrowing）
// 所有権を移さずに値を使う仕組み。&T（不変借用）と &mut T（可変借用）

fn main() {
    // === 不変借用（&T） ===
    // 所有権を移さずに「参照」を渡す
    let s = String::from("hello");
    let len = calculate_length(&s); // &s で参照を渡す
    println!("'{s}' の長さは {len}"); // s はまだ使える！

    // === 複数の不変借用は同時にOK ===
    let s = String::from("hello");
    let r1 = &s;
    let r2 = &s;
    println!("r1={r1}, r2={r2}"); // 読むだけなら何個でもOK

    // === 可変借用（&mut T） ===
    // 値を変更したい場合は &mut で借用する
    let mut s = String::from("hello");
    change(&mut s);
    println!("変更後: {s}"); // "hello, world!"

    // === 可変借用のルール: 同時に1つだけ ===
    let mut s = String::from("hello");
    let r1 = &mut s;
    // let r2 = &mut s; // コンパイルエラー！可変借用は1つだけ
    r1.push('!');
    println!("可変借用: {r1}");

    // === 不変借用と可変借用は同時に存在できない ===
    let mut s = String::from("hello");
    let r1 = &s; // 不変借用 OK
    let r2 = &s; // 不変借用 OK
    println!("{r1}, {r2}");
    // r1, r2 はここで最後に使われるので、ここで借用が終わる（NLL）
    let r3 = &mut s; // 可変借用 OK（r1, r2 はもう使われない）
    r3.push('!');
    println!("{r3}");

    // === NLL（Non-Lexical Lifetimes） ===
    // 借用の有効期間はスコープの終わりではなく「最後に使われた場所」まで
    // これにより上のコードが合法になる

    // === スライス — 借用の実用例 ===
    // 文字列スライス &str は String の一部への借用
    let s = String::from("hello world");
    let hello = &s[0..5]; // "hello" への借用
    let world = &s[6..11]; // "world" への借用
    println!("スライス: {hello} {world}");

    // 配列のスライスも同様
    let arr = [1, 2, 3, 4, 5];
    let slice = &arr[1..3]; // [2, 3] への借用
    println!("配列スライス: {:?}", slice);

    // === &str vs &String ===
    // 関数は &str を受け取るのが慣習的（より汎用的）
    let s = String::from("hello");
    print_str(&s); // &String → &str に自動変換（deref coercion）
    print_str("hello"); // 文字列リテラル &str もそのまま渡せる

    // === ダングリング参照の防止 ===
    // Rustはダングリング参照（無効な参照）をコンパイル時に防ぐ
    // let reference = dangle(); // コンパイルエラー！
    let owned = no_dangle(); // 所有権を返すのが正しい
    println!("ダングリング防止: {owned}");

    // === 借用ルールのまとめ ===
    println!("\n--- 借用ルールのまとめ ---");
    println!("1. 不変借用（&T）は同時に何個でもOK");
    println!("2. 可変借用（&mut T）は同時に1つだけ");
    println!("3. 不変と可変は同時に存在できない");
    println!("4. 参照は常に有効でなければならない（ダングリング禁止）");

    // === 演習 ===
    exercises();
}

// 不変借用を受け取る関数
// &str で参照を受け取るので、所有権はムーブされない
fn calculate_length(s: &str) -> usize {
    s.len()
} // s はここでスコープを抜けるが、所有権を持っていないので何も起きない

// 可変借用を受け取る関数
fn change(s: &mut String) {
    s.push_str(", world!");
}

// &str を受け取る関数（より慣習的）
// &String も &str も受け取れる
fn print_str(s: &str) {
    println!("print_str: {s}");
}

// ダングリング参照の例（コンパイルエラーになる）
// fn dangle() -> &String {
//     let s = String::from("hello");
//     &s  // s はこの関数の終わりで drop される
//         // → 存在しないデータへの参照を返すことになる
// }

// 正しい方法: 所有権を返す
fn no_dangle() -> String {
    String::from("hello")
}

// ============================================================
// 演習
// ============================================================

// --- 演習1: 基礎 ---
// 借用を使って文字列の長さを返す関数に書き直そう
// （04では所有権をタプルで返していたのをシンプルにする）
fn string_length(s: &str) -> usize {
    s.len()
}

// --- 演習2: 応用 ---
// 以下のコードがコンパイルエラーになる理由を説明し、修正しよう
fn exercise2() {
    let mut s = String::from("hello");
    let r1 = &s;
    println!("{r1}");
    s.push_str(", world!");
}

// --- 演習3: チャレンジ ---
// 文字列の最初の単語を返す関数を実装しよう
fn first_word(s: &str) -> &str {
    s.as_bytes()
        .iter()
        .enumerate()
        .find(|&(_, &byte)| byte == b' ')
        .map(|(i, _)| &s[..i])
        .unwrap_or(s)
}

// 演習の動作確認用
fn exercises() {
    // 演習1
    let s = String::from("hello, ownership!");
    let len = string_length(&s);
    println!("{s} の長さは {len}");

    // 演習2
    exercise2();

    // 演習3
    let s = String::from("hello world");
    let word = first_word(&s);
    println!("最初の単語: {word}");
}
