# 05 - 借用（Borrowing）

## 概要

借用は**所有権を移さずに値を使う仕組み**。04で感じた「所有権を毎回返す面倒さ」を解決する。`&T`（不変借用）と `&mut T`（可変借用）の2種類がある。

## 借用の4つのルール

1. **不変借用（`&T`）は同時に何個でもOK**
2. **可変借用（`&mut T`）は同時に1つだけ**
3. **不変借用と可変借用は同時に存在できない**
4. **参照は常に有効でなければならない**（ダングリング参照の禁止）

## Goとの比較

| 概念 | Go | Rust |
|---|---|---|
| ポインタ/参照 | `*T` / `&x` | `&T` / `&mut T` |
| ポインタ演算 | なし | なし |
| null | `nil`（実行時パニック） | 参照は常に有効（コンパイル時保証） |
| 可変性の制御 | なし（常に変更可能） | `&T`（読み取り専用）/ `&mut T`（変更可能） |
| データ競合防止 | `go run -race`（実行時） | 借用ルール（コンパイル時） |

### Goとの主な違い

```go
// Goでは同じデータを複数のgoroutineが同時に触れてしまう
s := []int{1, 2, 3}
go func() {
    s[0] = 99  // 書き込み
}()
fmt.Println(s[0])  // 読み込み → データ競合！
// 出力: 1 か 99 か不定（タイミング次第）
// go run -race で検出できるが、実行時チェック。気づかないこともある
```

```rust
// Rustでは同じデータの読み書きが同時に起きるとコンパイルエラー
let mut v = vec![1, 2, 3];
let r = &v[0];       // 不変借用（読み取り中）
v.push(99);          // 変更しようとする → コンパイルエラー！
println!("{r}");      // r がまだ生きているので衝突
// 出力: なし（そもそもコンパイルが通らない。実行前にバグが見つかる）
```

## コード解説

### 不変借用（`&T`）

```rust
fn calculate_length(s: &String) -> usize {
    s.len()
    // s.push_str("!"); // コンパイルエラー！不変借用では変更できない
}

let s = String::from("hello");
let len = calculate_length(&s);  // 参照を渡す
println!("{s}: {len}");          // s はまだ使える
```

04の演習2では所有権をタプルで返す必要があったが、借用ならシンプル。

### 可変借用（`&mut T`）

```rust
fn change(s: &mut String) {
    s.push_str(", world!");
}

let mut s = String::from("hello");  // mut が必要
change(&mut s);                      // &mut で可変借用を渡す
println!("{s}");                     // "hello, world!"
```

### 借用ルールの実例

```rust
let mut s = String::from("hello");

// OK: 不変借用は複数同時にできる
let r1 = &s;
let r2 = &s;
println!("{r1} {r2}");

// OK: r1, r2 の最後の使用が上のprintlnなので、ここでは借用が終わっている
let r3 = &mut s;
r3.push_str("!");

// NG: 不変借用と可変借用が同時に存在する
// let r1 = &s;
// let r2 = &mut s;
// println!("{r1} {r2}"); // コンパイルエラー！
```

### NLL（Non-Lexical Lifetimes）

借用の有効期間は「スコープの終わり」ではなく「最後に使われた場所」まで。

```rust
let mut s = String::from("hello");
let r1 = &s;
println!("{r1}");       // r1 の最後の使用 → ここで借用が終わる
let r2 = &mut s;        // OK（r1 はもう使われない）
println!("{r2}");
```

### スライス — 借用の実用例

スライスはデータの一部への借用。

```rust
let s = String::from("hello world");
let hello = &s[0..5];    // "hello"（String の一部を借用）
let world = &s[6..11];   // "world"

let arr = [1, 2, 3, 4, 5];
let slice = &arr[1..3];  // [2, 3]（配列の一部を借用）
```

```go
// Goのスライスに似ているが、Rustではコンパイラが有効性を保証
s := "hello world"
hello := s[0:5]   // "hello"

arr := [5]int{1, 2, 3, 4, 5}
slice := arr[1:3]  // [2, 3]
```

### `&str` vs `&String`

```rust
// &str を受け取る関数が慣習的（より汎用的）
fn print_str(s: &str) {
    println!("{s}");
}

let s = String::from("hello");
print_str(&s);       // &String → &str に自動変換
print_str("hello");  // 文字列リテラル（&str）もそのまま渡せる
```

`&String` より `&str` を受け取る方がよい理由:
- `String` も `&str`（文字列リテラル）もどちらも受け取れる
- Goで言うと `func f(s string)` が値もポインタも受け取れるようなもの

### ダングリング参照の防止

```rust
// コンパイルエラー！ダングリング参照
// fn dangle() -> &str {
//     let s = String::from("hello");
//     &s  // s は関数終了で drop → 無効なデータへの参照
// }

// 正しい方法: 所有権を返す
fn no_dangle() -> String {
    String::from("hello")  // 所有権を呼び出し元にムーブ
}
```

```go
// Goではこの問題は起きない（GCがあるから）
func dangle() *string {
    s := "hello"
    return &s  // OK（GCが s を生かしておく）
}
```

## 演習

### 演習1: 基礎

以下の関数を借用を使って書き直そう。所有権をムーブせずに文字列の長さを返す。

```rust
// 変更前（04の書き方）
fn string_length(s: String) -> (String, usize) {
    let len = s.len();
    (s, len)
}

// 変更後（借用を使う）
fn string_length(s: ???) -> usize {
    // ヒント: 引数の型を参照にする
}
```

### 演習2: 応用

以下のコードがコンパイルエラーになる理由を説明し、修正しよう。

```rust
fn main() {
    let mut s = String::from("hello");
    let r1 = &s;
    s.push_str(", world!");
    println!("{r1}");
}
```

### 演習3: チャレンジ

文字列の最初の単語を返す関数 `first_word` を実装しよう。戻り値は文字列スライス `&str` を使う。

```rust
fn first_word(s: &str) -> &str {
    // ヒント:
    // - s.as_bytes() でバイト列に変換
    // - .iter().enumerate() でインデックス付きイテレーション
    // - スペース (b' ') を見つけたら &s[..i] を返す
    // - 見つからなければ s 全体を返す
}

fn main() {
    let s = String::from("hello world");
    let word = first_word(&s);
    println!("最初の単語: {word}"); // "hello"
}
```

## まとめ

- **借用**は所有権を移さずに値を使う仕組み。04の不便さを解決する
- **`&T`**（不変借用）は読み取り専用。同時に複数OK
- **`&mut T`**（可変借用）は変更可能。同時に1つだけ
- 不変と可変の借用は**同時に存在できない**（データ競合を防止）
- **NLL** により借用の有効期間は「最後に使われた場所」まで
- **スライス** (`&str`, `&[T]`) はデータの一部への借用
- 関数は `&String` より **`&str`** を受け取るのが慣習的
- **ダングリング参照**はコンパイル時に防止される
