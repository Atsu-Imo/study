// 10 - コレクションとイテレータ
// Vec, HashMap, HashSet とイテレータチェーンを学ぶ

use std::collections::{HashMap, HashSet};

fn main() {
    // === Vec<T> — 動的配列 ===
    // Goの []T (slice) に相当
    let mut v: Vec<i32> = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);
    println!("Vec: {v:?}");

    // vec! マクロで初期化（よく使う）
    let v2 = vec![10, 20, 30];
    println!("vec!: {v2:?}");

    // インデックスアクセス — 範囲内なら値を返す
    println!("v[0] = {}", v[0]);
    // 範囲外の場合はパニック（Goと同じ）
    //   println!("{}", v[10]);  // panic: index out of bounds

    // get() — Option<&T> を返す安全なアクセス
    // 範囲外でもパニックせず None が返る
    // Goにはない（自前で len チェックが必要）
    match v.get(10) {
        Some(x) => println!("v[10] = {x}"),
        None => println!("v[10] は範囲外（パニックせず None）"),
    }

    // === HashMap<K, V> — 連想配列 ===
    // Goの map[K]V に相当
    let mut scores: HashMap<String, i32> = HashMap::new();
    scores.insert("Alice".to_string(), 90);
    scores.insert("Bob".to_string(), 85);

    // 取得 — Option<&V>。Goの comma-ok idiom に相当
    if let Some(score) = scores.get("Alice") {
        println!("Alice: {score}");
    }

    // entry() API — 「なければ挿入」「あれば更新」を1行で
    // Goでは scores["Charlie"]++ で済むが、Rustは明示的
    *scores.entry("Charlie".to_string()).or_insert(0) += 1;
    *scores.entry("Charlie".to_string()).or_insert(0) += 1;
    println!("Charlie: {}", scores["Charlie"]); // 2

    // === HashSet<T> — 集合 ===
    // Goにはない（map[T]struct{} で代用するのが一般的）
    let mut tags: HashSet<&str> = HashSet::new();
    tags.insert("rust");
    tags.insert("go");
    tags.insert("rust"); // 重複は無視される
    println!("HashSet: {tags:?} (要素数: {})", tags.len());

    // === イテレータの3種類 ===
    let v = vec![1, 2, 3];

    // iter() — &T を返す（borrow）
    println!("iter():");
    for x in v.iter() {
        println!("  {x}");
    }
    println!("v はまだ使える: {v:?}");

    // iter_mut() — &mut T を返す（mutable borrow）
    let mut v_mut = vec![1, 2, 3];
    for x in v_mut.iter_mut() {
        *x *= 10;
    }
    println!("iter_mut() の結果: {v_mut:?}");

    // into_iter() — T を返す（所有権を奪う）
    // for x in v は実は for x in v.into_iter() の糖衣構文
    let v_owned = vec![1, 2, 3];
    let sum: i32 = v_owned.into_iter().sum();
    println!("into_iter() の合計: {sum}");
    // v_owned はもう使えない（所有権が奪われた）

    // === イテレータチェーン ===
    // Goでは for ループで書く処理を、Rustは宣言的に書ける
    let nums = vec![1, 2, 3, 4, 5];

    // map — 各要素を変換
    let doubled: Vec<i32> = nums.iter().map(|x| x * 2).collect();
    println!("doubled: {doubled:?}");

    // filter — 条件で絞り込み
    let evens: Vec<i32> = nums.iter().filter(|&&x| x % 2 == 0).copied().collect();
    println!("evens: {evens:?}");

    // チェーン — 偶数の2乗の合計
    let result: i32 = nums.iter().filter(|&&x| x % 2 == 0).map(|&x| x * x).sum();
    println!("偶数の2乗の合計: {result}"); // 2*2 + 4*4 = 20

    // === よく使うイテレータメソッド ===
    let words = vec!["apple", "banana", "cherry"];

    // enumerate() — (index, value) のペア
    // Goの `for i, v := range s` に相当
    for (i, w) in words.iter().enumerate() {
        println!("{i}: {w}");
    }

    // zip() — 2つのイテレータをペアにする（Goにはない）
    let nums = vec![1, 2, 3];
    let letters = vec!["a", "b", "c"];
    let zipped: Vec<(i32, &str)> = nums
        .iter()
        .zip(letters.iter())
        .map(|(&n, &l)| (n, l))
        .collect();
    println!("zipped: {zipped:?}");

    // any() / all() — 述語の存在/全称
    let nums = vec![1, 2, 3, 4];
    let has_even = nums.iter().any(|&x| x % 2 == 0);
    let all_positive = nums.iter().all(|&x| x > 0);
    println!("偶数あり: {has_even}, 全て正: {all_positive}");

    // find() — 最初にマッチする要素
    let first_even = nums.iter().find(|&&x| x % 2 == 0);
    println!("最初の偶数: {first_even:?}");

    // fold() — 畳み込み（reduce）
    // Goでは for ループでアキュムレータを使う
    let product: i32 = nums.iter().fold(1, |acc, &x| acc * x);
    println!("総積: {product}"); // 1*2*3*4 = 24

    // === 遅延評価 ===
    // Goにはない概念 — collect() を呼ぶまでチェーンは実行されない
    println!("--- 遅延評価のデモ ---");
    let v = vec![1, 2, 3];
    let iter = v.iter().map(|x| {
        println!("  計算中: {x}");
        x * 2
    });
    println!("collect() 前");
    let _result: Vec<i32> = iter.collect(); // ← ここで初めて map のクロージャが実行される
    println!("collect() 後");

    // === 演習 ===
    exercises();
}

// ============================================================
// 演習
// ============================================================

// --- 演習1: 基礎 ---
// Vec<i32> を受け取り、奇数だけを抽出して2乗した結果を返す関数を作ろう
// odd_squares(&[1, 2, 3, 4, 5]) => vec![1, 9, 25]
//
fn odd_squares(nums: &[i32]) -> Vec<i32> {
    nums.iter()
        .filter(|&&x| x % 2 != 0)
        .map(|&x| x * x)
        .collect()
}

// --- 演習2: 応用 ---
// 文字列を受け取り、各文字の出現回数を HashMap<char, i32> として返す関数を作ろう
// char_count("hello") => {'h': 1, 'e': 1, 'l': 2, 'o': 1}
//
fn char_count(s: &str) -> HashMap<char, i32> {
    let mut map = HashMap::new();
    for c in s.chars() {
        *map.entry(c).or_insert(0) += 1;
    }
    map
}

// --- 演習3: チャレンジ ---
// (String, i32) のスライスを受け取り、以下を返す関数を作ろう
// 1. 平均点以上の人だけ抽出
// 2. 名前のアルファベット順にソート
// 3. "名前: 点数" の形式の文字列の Vec を返す
//
fn top_students(scores: &[(String, i32)]) -> Vec<String> {
    let total: i32 = scores.iter().map(|(_, s)| *s).sum();
    let avg = total / scores.len() as i32;

    let mut filtered: Vec<&(String, i32)> = scores.iter().filter(|(_, s)| *s >= avg).collect();
    filtered.sort_by(|a, b| a.0.cmp(&b.0));

    filtered
        .iter()
        .map(|(name, score)| format!("{name}: {score}"))
        .collect()
}

fn exercises() {
    // 演習の動作確認をここに書く
    // 例:
    // println!("{:?}", odd_squares(&[1, 2, 3, 4, 5]));
    // println!("{:?}", char_count("hello"));
    // let scores = vec![
    //     ("Alice".to_string(), 90),
    //     ("Bob".to_string(), 70),
    //     ("Charlie".to_string(), 85),
    // ];
    // println!("{:?}", top_students(&scores));
}
