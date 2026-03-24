# Q&A: 06-structs-and-methods

日付: 2026-03-06

## Q1: `String::from()` と `.to_string()` はどっちがいい？

どちらも同じ結果で、内部的にも同じ処理。好みで選んでOK。

- `String::from("hello")` — 「Stringを作る」意図が明確。公式ドキュメントで多い
- `"hello".to_string()` — メソッドチェーンの流れで自然。数値→文字列変換（`42.to_string()`）はこちらが自然

実際のプロジェクトではどちらかに統一されていることが多い。

## Q2: structに対するimplは必ず同名で、必ず1つ？

同名でなければならないのはその通り。ただし**implブロックは複数書ける**:
```rust
impl Rectangle {
    fn area(&self) -> f64 { ... }
}
impl Rectangle {
    fn perimeter(&self) -> f64 { ... }
}
```

複数に分ける主な理由はトレイト実装を分けるとき（08で学ぶ）。実用上は1つにまとめることが多い。

## Q3: 構造体更新構文で所有権のムーブとコピーが同時に発生するのが怖い。慣習的なやり方は？

部分ムーブ状態で元の変数を使い続けるケースはあまりない。実際のパターン:

1. **`Clone` を derive して `.clone()`（最も一般的）**
   ```rust
   let user2 = User { name: String::from("佐藤"), ..user.clone() };
   // user はまだ完全に使える
   ```

2. **元の変数をもう使わないならムーブで問題ない**
   ```rust
   let user2 = User { name: String::from("佐藤"), ..user };
   // user は以降使わない
   ```

3. 全フィールド明示的に書いて、必要なものだけ `.clone()`

## Q4: `&self` の省略しない形は？

`&self` は `self: &Self` の省略形:
```rust
fn area(&self) -> f64 { ... }           // 省略形
fn area(self: &Rectangle) -> f64 { ... } // 展開形
fn area(self: &Self) -> f64 { ... }      // Self はimpl対象の型のエイリアス
```

3パターン:
- `&self` = `self: &Self` — 不変借用
- `&mut self` = `self: &mut Self` — 可変借用
- `self` = `self: Self` — 所有権を受け取る（消費）

第一引数が特別扱いされる点はGoのレシーバと同じ発想。

## Q5: `into_parts(self)` を呼ぶとインスタンスが使えなくなる？

その通り。`self`（参照なし）で受け取ると所有権がムーブされ、呼び出し後はインスタンスが使えない:
```rust
let rect = Rectangle { width: 30.0, height: 50.0 };
let (w, h) = rect.into_parts();  // rect の所有権がムーブ
// println!("{:?}", rect);         // コンパイルエラー
```

`into_` プレフィックスは「所有権を消費する」メソッドのRust命名慣習。
