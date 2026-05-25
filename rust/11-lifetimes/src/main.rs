// 11 - ライフタイム
// 参照が指す先のデータが生存している保証をコンパイル時に検証する仕組みを学ぶ

fn main() {
    // === なぜライフタイムが必要か ===
    // 借用 (05章) のおさらい: 参照は元のデータより長く生きてはいけない
    // ライフタイムは「この参照は少なくともこの範囲で有効」を型に書く仕組み
    //
    // 以下はコンパイルエラー (dangling reference)
    //   let r;
    //   {
    //       let x = 5;
    //       r = &x;  // x は内側のスコープで解放される
    //   }
    //   println!("{r}");  // 解放済みのデータを指す参照

    // === ライフタイム注釈なしで済むケース ===
    // 後述する「省略規則」により、単純なシグネチャでは注釈を書かなくて良い
    let s = String::from("hello world");
    let word = first_word(&s);
    println!("first word: {word}");

    // === 複数の参照を扱う関数 — 注釈が必要 ===
    // longest は2つの &str を取り、長い方を返す
    // 戻り値の参照がどちらの入力に紐づくかコンパイラには分からないので、
    // 'a で「同じライフタイムである」と教える
    let s1 = String::from("long string is long");
    let s2 = String::from("short");
    let result = longest(&s1, &s2);
    println!("longest: {result}");

    // === ライフタイムは「最小公倍数」ではなく「共通区間」 ===
    // 戻り値は2つの入力の「短い方」の寿命に制限される
    let s1 = String::from("long string is long");
    {
        let s2 = String::from("short");
        let result = longest(&s1, &s2);
        println!("scoped longest: {result}");
        // result はこの内側スコープでしか使えない (s2 の寿命に律速)
    }

    // === 'static — プログラム全体で有効 ===
    // 文字列リテラルはバイナリに埋め込まれるので 'static
    let s: &'static str = "I have a static lifetime";
    println!("static: {s}");

    // === 構造体に参照を持たせる ===
    let novel = String::from("Call me Ishmael. Some years ago...");
    let first_sentence = novel.split('.').next().unwrap();
    let excerpt = Excerpt {
        part: first_sentence,
    };
    println!("excerpt: {}", excerpt.part);
    println!("announce: {}", excerpt.announce_and_return_part("attention"));

    // === ライフタイム省略規則 (Lifetime Elision) ===
    // 以下の3つの規則で「書かなくて良い」場面が決まる:
    //
    // 規則1: 入力の各参照に異なるライフタイムを自動付与
    //   fn foo(x: &i32, y: &i32) は fn foo<'a, 'b>(x: &'a i32, y: &'b i32)
    //
    // 規則2: 入力ライフタイムが1つだけなら、それを全ての出力に割り当てる
    //   fn first_word(s: &str) -> &str は省略形 (規則2 が適用される)
    //
    // 規則3: メソッド (&self を持つ) なら、self のライフタイムが出力に割り当てられる
    //   fn method(&self, x: &str) -> &str は &self のライフタイムが返り値に
    //
    // longest は規則1〜3 のどれも当てはまらない (入力が2つ、selfもない) ので明示が必要

    // === 演習 ===
    exercises();
}

// ============================================================
// 関数定義
// ============================================================

// 省略規則2 が働くので、注釈なしで書ける
// 実体は fn first_word<'a>(s: &'a str) -> &'a str
fn first_word(s: &str) -> &str {
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b' ' {
            return &s[..i];
        }
    }
    s
}

// 入力が2つあるので省略規則2 が使えない → 明示が必要
// 'a は「x と y のうち短い方の寿命」を表し、戻り値もそれに従う
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}

// ============================================================
// 構造体定義
// ============================================================

// 参照フィールドを持つ構造体は必ずライフタイム注釈が必要
// 「この構造体は参照先のデータより長く生きてはいけない」を型で表す
struct Excerpt<'a> {
    part: &'a str,
}

impl<'a> Excerpt<'a> {
    // 省略規則3 が働くので &self の寿命が戻り値に割り当てられる
    fn announce_and_return_part(&self, announcement: &str) -> &str {
        println!("announcement: {announcement}");
        self.part
    }
}

// ============================================================
// 演習
// ============================================================

// --- 演習1: 基礎 ---
// 2つの &str のうち、短い方を返す関数を作ろう
// shortest("hello", "hi") => "hi"
//
fn shortest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() < y.len() {
        x
    } else {
        y
    }
}

// --- 演習2: 応用 ---
// 文字列スライスを受け取り、最初の単語と残りをタプルで返す関数
// split_first_word("hello world foo") => ("hello", "world foo")
// スペースがない場合は ("入力全体", "")
//
fn split_first_word(s: &str) -> (&str, &str) {
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b' ' {
            return (&s[..i], &s[i + 1..]);
        }
    }
    (s, "")
}

// --- 演習3: チャレンジ ---
// テキストとその中の重要部分への参照を保持する構造体を作ろう
// new() で文字列を受け取り、最も長い単語への参照を part に持たせる
//
struct Highlight<'a> {
    text: &'a str,
    part: &'a str,
}

impl<'a> Highlight<'a> {
    fn new(text: &'a str) -> Self {
        let part = text
            .split_whitespace()
            .max_by_key(|w| w.len())
            .unwrap_or("");
        Highlight { text, part }
    }
}

fn exercises() {
    // 動作確認の例:
    // println!("{}", shortest("hello", "hi"));
    // let (head, tail) = split_first_word("hello world foo");
    // println!("head={head:?}, tail={tail:?}");
    // let h = Highlight::new("the quick brown fox jumps over the lazy dog");
    // println!("text={}, part={}", h.text, h.part);
}
