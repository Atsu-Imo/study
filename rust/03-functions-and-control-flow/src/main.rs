// 03 - 関数と制御フロー
// 関数定義、式ベースのreturn、if式、ループ（loop/while/for）を学ぶ

fn main() {
    // === 関数呼び出し ===
    greet("Rust");
    let sum = add(3, 5);
    println!("3 + 5 = {sum}");

    // === 式ベースの戻り値 ===
    // Rustでは最後の式がそのまま戻り値になる（セミコロンなし）
    let doubled = double(21);
    println!("21の2倍 = {doubled}");

    // === if式 ===
    // Rustの if は「式」なので値を返せる（Goの if は文）
    let number = 7;
    let parity = if number % 2 == 0 { "偶数" } else { "奇数" };
    println!("{number}は{parity}");

    // if-else if チェーン
    let score = 85;
    let grade = if score >= 90 {
        "A"
    } else if score >= 80 {
        "B"
    } else if score >= 70 {
        "C"
    } else {
        "D"
    };
    println!("スコア{score} → 評価{grade}");

    // === loop（無限ループ） ===
    // Goの for {} に相当
    let mut count = 0;
    let result = loop {
        count += 1;
        if count == 5 {
            break count * 10; // break で値を返せる（Goにはない）
        }
    };
    println!("loopの結果: {result}"); // 50

    // === while ===
    // Goでは for condition {} を使う
    let mut n = 3;
    while n > 0 {
        println!("カウントダウン: {n}");
        n -= 1;
    }
    println!("発射！");

    // === for ===
    // Goの for range に相当
    // 範囲（Range）を使った繰り返し
    print!("for(範囲): ");
    for i in 1..=5 {
        // 1..=5 は 1,2,3,4,5（inclusive）
        // 1..5  は 1,2,3,4  （exclusive）
        print!("{i} ");
    }
    println!();

    // コレクションのイテレーション
    let fruits = ["りんご", "みかん", "ぶどう"];
    for fruit in &fruits {
        println!("果物: {fruit}");
    }

    // インデックス付きイテレーション（Goの for i, v := range に相当）
    for (i, fruit) in fruits.iter().enumerate() {
        println!("  {i}: {fruit}");
    }

    // === ネストしたループとラベル ===
    // Goにもラベル付きbreakがあるが、Rustは 'label 記法
    'outer: for i in 0..3 {
        for j in 0..3 {
            if i == 1 && j == 1 {
                println!("({i},{j})でouter breakします");
                break 'outer;
            }
            print!("({i},{j}) ");
        }
        println!();
    }
    println!();

    // === 関数から早期リターン ===
    println!("abs(-5) = {}", abs(-5));
    println!("abs(3) = {}", abs(3));

    // === 何もしないブロック ===
    // Goの _ と同様、値を無視できる
    let _ = add(1, 2); // 戻り値を明示的に捨てる

    // === FizzBuzz ===
    println!("FizzBuzz:");
    fizz_buzz(15);

    // === フィボナッチ数列 ===
    println!("フィボナッチ数列:");
    for n in 0..10 {
        println!("fib({n}) = {}", fib(n));
    }

    let mut sum = 0;
    let mut i = 1;
    let n = loop {
        i += 1;
        sum += i;
        if sum >= 100 {
            break i; // ループを抜けるときに合計を返す
        }
    };
    println!("ループで合計が100以上になったときの合計: {sum}");
    println!("ループで合計が100以上になったときのi: {n}");
}

// === 関数定義 ===
// fn 関数名(引数: 型) -> 戻り値の型
// Goの func に相当。引数の型注釈は必須（推論されない）
fn greet(name: &str) {
    println!("こんにちは、{name}さん！");
}

// 戻り値のある関数
fn add(a: i32, b: i32) -> i32 {
    a + b // セミコロンなし = この式の値を返す
}

// return キーワードも使えるが、最後の式で返すのがRust流
fn double(x: i32) -> i32 {
    x * 2 // セミコロンを付けると () を返すことになりエラー
}

// 早期リターンには return を使う
fn abs(x: i32) -> i32 {
    if x < 0 {
        return -x; // 早期リターン
    }
    x // 最後の式で返す
}

fn fizz_buzz(n: i32) {
    for i in 1..=n {
        if i % 15 == 0 {
            println!("FizzBuzz");
        } else if i % 3 == 0 {
            println!("Fizz");
        } else if i % 5 == 0 {
            println!("Buzz");
        } else {
            println!("{i}");
        }
    }
}

fn fib(n: u32) -> u32 {
    if n == 0 {
        0
    } else if n == 1 {
        1
    } else {
        fib(n - 1) + fib(n - 2)
    }
}
