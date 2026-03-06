# 01 - Hello, Cargo!

## 概要

Rustのビルドシステム兼パッケージマネージャである **Cargo** の基本的な使い方と、最初のRustプログラムを学ぶ。

## Goとの比較

| 概念 | Go | Rust |
|---|---|---|
| プロジェクト初期化 | `go mod init` | `cargo new` / `cargo init` |
| ビルド | `go build` | `cargo build` |
| 実行 | `go run main.go` | `cargo run` |
| 依存管理ファイル | `go.mod` | `Cargo.toml` |
| ロックファイル | `go.sum` | `Cargo.lock` |
| テスト | `go test` | `cargo test` |
| フォーマッタ | `gofmt` | `rustfmt` (`cargo fmt`) |
| リンタ | `golangci-lint` | `clippy` (`cargo clippy`) |

### Goとの主な違い

- Goは `main` パッケージの `main()` 関数がエントリポイント。Rustも `fn main()` がエントリポイントだが、パッケージの概念は異なる
- Goの `fmt.Println` に相当するのが `println!` マクロ。`!` がついているのはマクロである証拠
- Goは `go.mod` で1つのモジュール。Rustは `Cargo.toml` で1つのクレート（パッケージ）

## プロジェクト構成

```
01-hello-cargo/
├── Cargo.toml          ← プロジェクトのメタデータと依存関係
├── README.md           ← このファイル
└── src/
    └── main.rs         ← エントリポイント
```

### Cargo.toml

```toml
[package]
name = "hello-cargo"    # クレート名（Goのmodule名に相当）
version = "0.1.0"       # セマンティックバージョニング
edition = "2021"        # Rustエディション（言語バージョンのようなもの）
```

Goの `go.mod` に相当するが、より多くのメタデータを持つ。

## コード解説

```rust
fn main() {
    println!("Hello, Cargo!");
}
```

- `fn` — 関数定義キーワード（Goの `func` に相当）
- `main()` — エントリポイント。Goと同じく引数なし
- `println!` — 標準出力に改行付きで出力するマクロ
  - `!` はマクロ呼び出しを意味する。通常の関数呼び出しではない
  - Goの `fmt.Println` に相当

### フォーマット文字列

```rust
let language = "Rust";
let version = 2021;
println!("{language} edition {version} へようこそ！");  // 変数を直接埋め込み
println!("1 + 2 = {}", 1 + 2);                        // {} に引数を順番に埋め込み
println!("デバッグ表示: {:?}", (1, "hello", true));     // Debug表示
```

| Go | Rust | 説明 |
|---|---|---|
| `fmt.Println("hello")` | `println!("hello")` | 改行付き出力 |
| `fmt.Printf("%s %d", s, n)` | `println!("{} {}", s, n)` | フォーマット出力 |
| `fmt.Printf("%v", x)` | `println!("{:?}", x)` | デバッグ出力 |
| `fmt.Printf("%+v", x)` | `println!("{:#?}", x)` | 詳細デバッグ出力 |

## Cargoコマンド

```bash
# ビルド（デバッグモード）
cargo build

# ビルドして実行
cargo run

# コードチェック（ビルドより高速）
cargo check

# リリースビルド（最適化あり）
cargo build --release

# フォーマット
cargo fmt

# リント
cargo clippy

# テスト
cargo test
```

## 演習

### 演習1: 基礎

`src/main.rs` を編集して、自分の名前と今日の日付を表示するプログラムに変更してみよう。

```rust
fn main() {
    let name = "あなたの名前";
    let date = "2024-01-01";
    println!("こんにちは、{name}さん！今日は{date}です。");
}
```

### 演習2: 応用

複数のフォーマット指定子を試してみよう。

- `{:b}` — 2進数表示
- `{:x}` — 16進数表示
- `{:>10}` — 右寄せ（幅10）
- `{:.2}` — 小数点以下2桁

```rust
fn main() {
    let n = 42;
    println!("10進: {n}");
    println!("2進: {n:b}");
    println!("16進: {n:x}");
    println!("右寄せ: {:>10}", "hello");
    println!("小数: {:.2}", 3.14159);
}
```

### 演習3: チャレンジ

`eprintln!` マクロを使って標準エラー出力にメッセージを出力してみよう。Goでは `fmt.Fprintln(os.Stderr, ...)` に相当する。`println!` との違いを確認しよう。

```bash
# 標準出力だけリダイレクトすると、eprintln! の出力は画面に残る
cargo run > /dev/null
```

## まとめ

- **Cargo** はRustのビルドシステム兼パッケージマネージャ（Goの `go` コマンドに相当）
- **Cargo.toml** がプロジェクト定義ファイル（Goの `go.mod` に相当）
- **`fn main()`** がエントリポイント（Goと同じ）
- **`println!`** は関数ではなくマクロ（`!` で区別）
- **フォーマット文字列** は `{}` を使う（Goの `%v`, `%s` の代わり）
- `cargo run`, `cargo build`, `cargo check`, `cargo clippy`, `cargo fmt` を覚えておこう
