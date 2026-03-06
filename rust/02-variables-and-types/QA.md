# Q&A: 02-variables-and-types

日付: 2026-03-06

## Q1: シャドウイングは何のためにある？複雑さが増すだけでは？

主な用途は2つ:

**1. 型変換の連鎖で名前を使い回す**
```rust
let input = "42";
let input = input.trim();
let input = input.parse::<i32>().unwrap();
// シャドウイングなしだと input_raw, input_trimmed, input_parsed と命名が必要
```

**2. mut を最小限にする**
```rust
let x = compute_something();
let x = x + 1;  // 新しい不変変数。意図しない変更を防げる
```

Rustは不変がデフォルトなので、「同じ意味の値を段階的に変換するとき、毎回 `let` で新しい不変変数を作る」のが自然な書き方。関係ない値に同じ名前を付けるのはNG。あくまで「同じものの変換」に使うのが慣習。

## Q2: trim().parse() のようにメソッドチェーンにして変数名を付けた方がよいのでは？

その通り。チェーンできるならチェーンの方がRust的にも自然:
```rust
let input: i32 = "42".trim().parse().unwrap();
```

シャドウイングが本当に活きるのは、チェーンできない場面（別の関数に渡して戻り値を受ける場合など）:
```rust
let config = std::fs::read_to_string("config.toml").unwrap();
let config = config.trim();
let config = toml::from_str::<Config>(config).unwrap();  // 別クレートの関数に渡す
```

## Q3: クレートって何？

Rustの**コンパイル単位であり、パッケージの単位**。Goの「モジュール」に近い。

- **バイナリクレート** — `main.rs` を持つ。実行可能ファイルを生成
- **ライブラリクレート** — `lib.rs` を持つ。他のクレートから使われる部品

対応関係:
- `Cargo.toml` = `go.mod`
- crates.io = pkg.go.dev
- `[dependencies]` にクレートを追加 = `go get` に相当

## Q4: タプルは型を指定しなくてもいい？

型推論が効くので省略可能:
```rust
let tup = (500, 6.4, true);  // (i32, f64, bool) に推論
```

推論できない場合（`parse()` の戻り値など）は明示が必要:
```rust
let tup: (i32, f64) = ("42".parse().unwrap(), 1.0);
```

タプル固有のルールではなく、`let` の型推論全般に言えること。

## Q5: タプル同士の比較はできる？

できる。要素の型が同じで各要素が比較可能なら `==`, `!=`, `<`, `>` 等が使える:
```rust
let a = (1, 2, 3);
let b = (1, 2, 3);
let c = (1, 2, 4);
println!("{}", a == b);  // true
println!("{}", a < c);   // true（先頭から辞書順で比較）
```

制限: 要素数は12個まで（標準ライブラリの `PartialEq` 等の実装が12要素まで）。型が異なるタプル同士の比較はコンパイルエラー。

## Q6: タプルの配列/Vecで、違う内部型のタプルを弾くにはどうする？

自分で制約を書く必要はない。**コンパイラが型チェックで弾く**:
```rust
let mut items = vec![(1, "apple"), (2, "banana")]; // Vec<(i32, &str)> に推論
items.push((3, "cherry"));    // OK
// items.push((3, 100));      // コンパイルエラー: (i32, i32) は型が違う
// items.push((3,));           // コンパイルエラー: 要素数が違う
```

Goで構造体のスライスに違う型を入れられないのと同じ。Rustではタプルのレベルで型安全が保証される。`append` に相当するのは `Vec` の `push`（配列は固定長なので追加不可）。
