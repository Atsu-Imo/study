# 07 - 列挙型とパターンマッチ Q&A

日付: 2026-03-07

## Q1: Optionってつまりnullableってこと？

本質的には「値があるかないか」を表す点で同じだが、重要な違いがある。

- **nullable（Go の nil など）**: チェックを忘れても コンパイルが通り、実行時にパニックする（開発者の注意力頼み）
- **Option\<T\>**: `Some(T)` か `None` を処理しないとコンパイルエラーになる（型システムが強制する）

```go
// Go — nil チェック忘れでも コンパイル通る
u := findUser(1)       // *User が nil かもしれない
fmt.Println(u.Name)    // 実行時パニック！
```

```rust
// Rust — 中身を取り出さないと使えない
let u = find_user(1);  // Option<User>
// u.name はコンパイルエラー（Option に name はない）
if let Some(user) = u {
    println!("{}", user.name);  // ここでは確実に存在
}
```

Kotlin の `String?` や TypeScript の `string | undefined` に近い発想だが、Rust は `match` の網羅性チェックでさらに厳格。

## Q2: Result<T, E>のT, Eってなに？Tはジェネリクス型でよく使われるけどEはエラーのE？

その通り。どちらもジェネリクスの型パラメータで、慣習的な命名。

- **T** — Type の T
- **E** — Error の E

```rust
enum Result<T, E> {
    Ok(T),   // 成功時の値の型
    Err(E),  // エラー時の値の型
}
```

`Result<u32, String>` なら「成功したら `u32`、失敗したら `String`」。

Rustでよく使われる型パラメータ名:

| 名前 | 由来 | 例 |
|---|---|---|
| `T` | Type | `Vec<T>`, `Option<T>` |
| `E` | Error | `Result<T, E>` |
| `K` | Key | `HashMap<K, V>` |
| `V` | Value | `HashMap<K, V>` |

ただの名前なので `Result<Success, Failure>` でも動くが、慣習に従うのが普通。

## Q3: Result<u32, String>を返す関数が複数あって関係し合っているとき、型として定義するのはRust的にどう？

推奨されるパターン。型エイリアス（`type`）を使う。

```rust
type AgeResult = Result<u32, String>;
fn parse_age(input: &str) -> AgeResult { ... }
fn validate_age(age: u32) -> AgeResult { ... }
```

さらに一般的なのは、エラー型だけ固定して `T` は自由にするパターン:

```rust
type Result<T> = std::result::Result<T, MyError>;
fn foo() -> Result<u32> { ... }
fn bar() -> Result<String> { ... }
```

標準ライブラリの `std::io::Result<T>` がまさにこのパターン。09（エラーハンドリング）で詳しく扱う。

## Q4: `let age = parse_age(input)?;` のエラーについてもう少し詳しく

`parse_age` は2つのケースで `Err(String)` を返す:

1. **`parse()` 失敗**: `"abc"` → `.parse::<u32>()` が `Err(ParseIntError)` → `.map_err()` で `Err("'abc'は数値ではありません".to_string())` に変換
2. **範囲チェック**: `"200"` → parse は成功するが `age > 150` で `Err("年齢が大きすぎます".to_string())`

呼び出し側の `?` は以下と同等:

```rust
let age = match parse_age(input) {
    Ok(v) => v,               // 成功 → v を age に入れる
    Err(e) => return Err(e),  // 失敗 → e をそのまま呼び出し元に返す
};
```

`parse_age` が返した `Err(String)` がそのまま `process_age` の `Err(String)` として伝播する。Goの `if err != nil { return "", err }` と同じ。

## Q5: `.map_err()` がなければどうなるの？

`parse()` は `Result<u32, ParseIntError>` を返すが、関数の戻り値は `Result<u32, String>`。`?` はエラー型をそのまま返そうとするので、`ParseIntError` → `String` の変換ができずコンパイルエラーになる。

```
error: the trait `From<ParseIntError>` is not implemented for `String`
```

`map_err` はエラー型を変換する役割:

```rust
input.parse()
// → Ok(25) or Err(ParseIntError)

input.parse().map_err(|_| format!("..."))
// → Ok(25) or Err(String)  ← 戻り値の型と一致する
```

`|_|` の `_` は `ParseIntError` を受け取るが使わないので無視している。Goで言えば `err` を受け取って別のエラーメッセージに差し替える処理に相当。

## Q6: `|_|` これなに

クロージャ（無名関数）。Goの `func(引数) { 本体 }` に相当。

```rust
|引数| 本体
```

- `|_|` — 引数1つを受け取るが `_` で捨てている
- `|e| format!("エラー: {}", e)` — 引数を使う場合
- `|a, b| a + b` — 引数2つ
- `|| println!("hello")` — 引数なし

クロージャは 12 で詳しく扱うが、`map`, `map_err`, `find` などで頻出するので先に構文だけ覚えておくとよい。

## Q7: `match self` の `self` は `&self` じゃなくていいの？

メソッド引数の `&self` で受け取った時点で `self` の型は `&TrafficLight`（参照）になっている。本体内で `self` を使うときは既に参照なので、改めて `&` を付ける必要はない。

```rust
fn message(&self) -> &str {
    // self の型はすでに &TrafficLight
    match self {
        TrafficLight::Red => "止まれ",
        ...
    }
}
```
