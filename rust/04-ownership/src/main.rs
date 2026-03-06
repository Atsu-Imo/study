// 04 - 所有権（Ownership）
// Rust最大の特徴。GCなしでメモリ安全を実現する仕組み

fn main() {
    // === 所有権の基本ルール ===
    // 1. 各値には「所有者」となる変数が1つだけ存在する
    // 2. 所有者がスコープを抜けると、値は自動的に解放される（drop）
    // 3. 所有権は「ムーブ」によって別の変数に移る

    // === スコープとdrop ===
    {
        let s = String::from("hello"); // s がこの String の所有者
        println!("スコープ内: {s}");
    } // ← ここで s がスコープを抜け、String のメモリが解放される（drop）
      // Goではここでは何も起きない。GCが後で回収する

    // === ムーブ（Move） ===
    // String のようなヒープデータは代入時に所有権が移る
    let s1 = String::from("hello");
    let s2 = s1; // s1 の所有権が s2 にムーブされる
                 // println!("{s1}"); // コンパイルエラー！s1 はもう使えない
    println!("ムーブ後の s2: {s2}");

    // なぜムーブが必要か？
    // もし s1 と s2 が同じヒープメモリを指していたら、
    // 両方がスコープを抜けた時に二重解放（double free）が起きる。
    // ムーブにより所有者を1つに限定して二重解放を防ぐ。

    // === コピー（Copy） ===
    // 整数などスタック上のデータは Copy トレイトを持つ
    // 代入してもムーブではなくコピーされる
    let x = 42;
    let y = x; // コピーされる（x はまだ使える）
    println!("コピー: x={x}, y={y}"); // どちらも使える

    // Copy される型: i32, f64, bool, char, タプル（中身がCopyなら）
    // Copy されない型: String, Vec, その他ヒープを使う型

    // === 関数と所有権 ===
    // 関数に値を渡すとムーブが起きる（Copyでない場合）
    let s = String::from("hello");
    takes_ownership(s);
    // println!("{s}"); // コンパイルエラー！所有権は関数にムーブされた

    let n = 42;
    makes_copy(n);
    println!("関数呼び出し後も使える: {n}"); // OK（i32 は Copy）

    // === 所有権を返す ===
    // 関数から戻り値で所有権を返せる
    let s1 = gives_ownership();
    println!("所有権を受け取った: {s1}");

    let s2 = String::from("world");
    let s3 = takes_and_gives_back(s2);
    // println!("{s2}"); // コンパイルエラー！s2 はムーブされた
    println!("所有権を返してもらった: {s3}");

    // === clone ===
    // 所有権を渡したくない場合、明示的にコピーできる
    let original = String::from("deep copy");
    let cloned = original.clone(); // ヒープデータを複製
    println!("original={original}, cloned={cloned}"); // 両方使える

    // === スタックとヒープ ===
    // この違いを理解すると所有権のルールが腑に落ちる
    //
    // スタック: サイズが固定の値（i32, bool, char, タプル等）
    //   → コピーが高速なので Copy トレイトを持つ
    //
    // ヒープ: サイズが可変の値（String, Vec 等）
    //   → コピーが重いのでムーブがデフォルト
    //   → 必要なら .clone() で明示的にコピー

    // === 所有権とGoの違い（まとめ） ===
    println!("\n--- 所有権のまとめ ---");
    println!("Go: GCがメモリを管理。変数は自由にコピー・共有できる");
    println!("Rust: 所有権システムがコンパイル時にメモリ管理を保証");
    println!("  → GCなしで安全。実行時のオーバーヘッドなし");
}

// 所有権を受け取る関数（String はムーブされる）
fn takes_ownership(s: String) {
    println!("所有権を受け取った: {s}");
} // ← ここで s が drop される

// コピーを受け取る関数（i32 は Copy される）
fn makes_copy(n: i32) {
    println!("コピーを受け取った: {n}");
}

// 所有権を返す関数
fn gives_ownership() -> String {
    String::from("新しい文字列")
}

// 所有権を受け取って返す関数
fn takes_and_gives_back(s: String) -> String {
    s // そのまま返す（所有権が呼び出し元に戻る）
}

fn string_length(s: String) -> (String, usize) {
    let length = s.len();
    (s, length) // 所有権を返す
}

fn main() {
    let s = String::from("hello");
    let s = print_string(s);
    let _ = print_string(s); // 2回目の呼び出し
}

fn print_string(s: String) -> String {
    println!("{s}");
    s
}
