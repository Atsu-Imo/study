# 10 - コレクションとイテレータ

## 概要

Rustの標準コレクション（`Vec`, `HashMap`, `HashSet`）と、それらを操作する**イテレータ**を学ぶ。

Goの `slice` と `map` に相当する型を扱いつつ、**Goにはない強力な機能**であるイテレータチェーン（`map`/`filter`/`collect` などのメソッドチェーン）を中心に深掘りする。

## Goとの比較

| 概念 | Go | Rust |
|---|---|---|
| 動的配列 | `[]T` (slice) | `Vec<T>` |
| 連想配列 | `map[K]V` | `HashMap<K, V>` |
| 集合 | (なし、`map[T]struct{}` で代用) | `HashSet<T>` |
| 要素追加 | `append(s, x)` | `v.push(x)` |
| 要素取得 | `s[i]` (パニック) | `v[i]` (パニック) / `v.get(i)` (`Option`) |
| イテレーション | `for i, x := range s` | `for (i, x) in v.iter().enumerate()` |
| マップ操作 | for ループで自前実装 | `iter().map(...).collect()` |
| フィルタ | for ループで自前実装 | `iter().filter(...).collect()` |
| 合計 | for ループで自前実装 | `iter().sum()` |

### Goとの主な違い

```go
// Go — 明示的なループで処理
func doubleEvens(nums []int) []int {
    var result []int
    for _, n := range nums {
        if n % 2 == 0 {
            result = append(result, n * 2)
        }
    }
    return result
}
```

```rust
// Rust — イテレータチェーンで宣言的に書ける
fn double_evens(nums: &[i32]) -> Vec<i32> {
    nums.iter()
        .filter(|&&n| n % 2 == 0)
        .map(|&n| n * 2)
        .collect()
}
```

主な違い:
- **Goは命令的** — 「どうやるか」を毎回書く。シンプルだが繰り返しが多い
- **Rustは宣言的** — 「何をしたいか」をチェーンで書ける（関数型スタイル）
- **遅延評価** — Rustのイテレータは `collect()` などの終端メソッドを呼ぶまで実行されない
- **ゼロコスト抽象化** — チェーンはコンパイラに最適化され、手書きの for ループと同等の性能になる

## コード解説

### Vec<T> — 動的配列

```rust
let mut v: Vec<i32> = Vec::new();
v.push(1);
v.push(2);
v.push(3);

let v2 = vec![1, 2, 3];  // マクロで初期化

println!("{}", v[0]);              // 1
println!("{}", v[10]);             // panic! (index out of bounds)
println!("{:?}", v.get(0));        // Some(1)
println!("{:?}", v.get(10));       // None (パニックしない)
```

Goとの比較:
```go
v := []int{}
v = append(v, 1)
v = append(v, 2)
v = append(v, 3)

v2 := []int{1, 2, 3}
fmt.Println(v[0])   // 1
fmt.Println(v[10])  // panic: runtime error: index out of range
// v.get(10) 相当はない — 自前で len チェック
```

主な違い:
- **`append` は新しい slice を返すが、`push` は所有権を持って自身を変更**
- **`v.get(i)` は `Option<&T>` を返す** — 範囲外でもパニックしない安全な取得方法

### HashMap<K, V> — 連想配列

```rust
use std::collections::HashMap;

let mut scores: HashMap<String, i32> = HashMap::new();
scores.insert("Alice".to_string(), 90);
scores.insert("Bob".to_string(), 85);

// 取得 — Option<&V> を返す
if let Some(score) = scores.get("Alice") {
    println!("Alice: {score}");
}

// エントリAPI — 「なければ挿入」「あれば更新」
*scores.entry("Charlie".to_string()).or_insert(0) += 1;
```

Goとの比較:
```go
scores := map[string]int{}
scores["Alice"] = 90
scores["Bob"] = 85

// 取得 — comma ok idiom
if score, ok := scores["Alice"]; ok {
    fmt.Println("Alice:", score)
}

// 「なければ初期化」は自前
scores["Charlie"]++  // ゼロ値が初期値になるので Go ではこれで済む
```

主な違い:
- **キーは所有権が必要** — `String` を入れるので `.to_string()` が要る（Goの `string` は値型なので不要）
- **`entry().or_insert()` パターン** — Goより冗長だが、明示的に「初期値」を指定できる
- **ゼロ値の概念がない** — Rustは「なければ `None`」を明示的に扱う

### イテレータの3種類

```rust
let v = vec![1, 2, 3];

// 1. iter() — &T を返す（不変参照）
for x in v.iter() {
    println!("{x}");
}
println!("{v:?}");  // v はまだ使える

// 2. iter_mut() — &mut T を返す（可変参照）
let mut v = vec![1, 2, 3];
for x in v.iter_mut() {
    *x *= 2;
}
println!("{v:?}");  // [2, 4, 6]

// 3. into_iter() — T を返す（所有権を奪う）
let v = vec![1, 2, 3];
for x in v.into_iter() {
    println!("{x}");  // x は i32（所有権を持つ）
}
// v はもう使えない
```

**`for x in v` は実は `for x in v.into_iter()` の糖衣構文。** `v` の所有権が奪われる点に注意。

### イテレータチェーン

```rust
let nums = vec![1, 2, 3, 4, 5];

// map — 各要素を変換
let doubled: Vec<i32> = nums.iter().map(|x| x * 2).collect();
// [2, 4, 6, 8, 10]

// filter — 条件で絞り込み
let evens: Vec<i32> = nums.iter().filter(|&&x| x % 2 == 0).copied().collect();
// [2, 4]

// sum — 合計
let total: i32 = nums.iter().sum();
// 15

// チェーン
let result: i32 = nums.iter()
    .filter(|&&x| x % 2 == 0)
    .map(|&x| x * x)
    .sum();
// 2*2 + 4*4 = 20
```

### よく使うメソッド一覧

| メソッド | 説明 | Go相当 |
|---|---|---|
| `map(f)` | 各要素に `f` を適用 | for ループ + append |
| `filter(p)` | 述語 `p` が true の要素のみ残す | for ループ + if |
| `collect()` | イテレータを `Vec` などに集める | （終端処理） |
| `sum()` | 合計 | for ループで加算 |
| `count()` | 要素数 | `len()` または for ループ |
| `enumerate()` | `(index, value)` のペアにする | `for i, v := range s` |
| `zip(other)` | 2つのイテレータをペアにする | （Goにはない、自前ループ） |
| `take(n)` | 先頭 n 個を取る | `s[:n]` |
| `skip(n)` | 先頭 n 個を捨てる | `s[n:]` |
| `chain(other)` | 2つのイテレータを連結 | `append(s1, s2...)` |
| `fold(init, f)` | 畳み込み（reduce） | for ループでアキュムレータ |
| `any(p)` | 1つでも true があるか | for ループ + early return |
| `all(p)` | 全て true か | for ループ + early return |
| `find(p)` | 最初に true となる要素 | for ループ + early return |

### 遅延評価

```rust
let v = vec![1, 2, 3, 4, 5];

// この時点では何も実行されない（イテレータが組み立てられるだけ）
let iter = v.iter().map(|x| {
    println!("計算中: {x}");
    x * 2
});

println!("終端メソッド呼び出し前");
let result: Vec<i32> = iter.collect();  // ← ここで初めて実行される
println!("{result:?}");
```

出力:
```
終端メソッド呼び出し前
計算中: 1
計算中: 2
計算中: 3
計算中: 4
計算中: 5
[2, 4, 6, 8, 10]
```

`collect()` を呼ぶまで `map` の中身は実行されない。**Goにはない概念**。

### `&&` と `copied()` の罠

```rust
let nums = vec![1, 2, 3];
let evens: Vec<i32> = nums.iter()
    .filter(|&&x| x % 2 == 0)  // ← なぜ && が必要？
    .copied()                    // ← なぜ copied?
    .collect();
```

- `nums.iter()` は `&i32` を返す
- `filter` のクロージャは `&&i32` を受け取る（イテレータの要素への参照）
- `|&&x|` で2回デリファレンスして `i32` として取り出す
- `filter` 後も要素は `&i32` のまま → `copied()` で `i32` に変換してから `Vec<i32>` に集める

最初は戸惑うが、**所有権と参照の規則を一貫させた結果**。

## 演習

### 演習1: 基礎

`Vec<i32>` を受け取り、奇数だけを抽出して2乗した結果を返す関数を作ろう。

```rust
fn odd_squares(nums: &[i32]) -> Vec<i32> {
    todo!()
}
// odd_squares(&[1, 2, 3, 4, 5]) => vec![1, 9, 25]
```

ヒント: `iter()`, `filter()`, `map()`, `collect()` を使う。

### 演習2: 応用

文字列を受け取り、各文字の出現回数を `HashMap<char, i32>` として返す関数を作ろう。

```rust
use std::collections::HashMap;

fn char_count(s: &str) -> HashMap<char, i32> {
    todo!()
}
// char_count("hello") => {'h': 1, 'e': 1, 'l': 2, 'o': 1}
```

ヒント: `s.chars()` で文字のイテレータが得られる。`entry().or_insert(0)` パターンを使う。

### 演習3: チャレンジ

`Vec<(String, i32)>`（名前と点数のペア）を受け取り、以下を返す関数を作ろう。

1. 平均点以上の人だけ抽出
2. 名前のアルファベット順にソート
3. 名前と点数を `"Alice: 90"` の形式の文字列にして `Vec<String>` で返す

```rust
fn top_students(scores: &[(String, i32)]) -> Vec<String> {
    todo!()
}
```

ヒント:
- 平均値は `iter().map(|(_, s)| *s).sum::<i32>() / scores.len() as i32` で計算
- ソートは `sort_by_key` を使う（または `sort_by`）
- イテレータチェーンの途中で `Vec` に集めて、ソートしてから次のチェーンに繋ぐ

## まとめ

- **`Vec<T>`** は Goの `[]T` に相当。`push`/`pop`/`get` などのメソッドで操作
- **`HashMap<K, V>`** は Goの `map[K]V` に相当。所有権が必要な点と `entry()` API が特徴
- **イテレータ**は Rust の強力な抽象化。`map`/`filter`/`collect` などのチェーンで宣言的に書ける
- **3種類のイテレータ** — `iter()` (&T), `iter_mut()` (&mut T), `into_iter()` (T)
- **遅延評価** — 終端メソッド（`collect`, `sum` など）を呼ぶまで実行されない
- **ゼロコスト抽象化** — イテレータチェーンは手書きループと同等の性能
- Goは命令的に for ループを書くスタイル。Rustは関数型スタイルが標準
