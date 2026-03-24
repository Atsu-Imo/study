# 09 - エラーハンドリング

## 概要

Rustのエラーハンドリングを深掘りする。`Result<T, E>` の基本は07で学んだが、ここではカスタムエラー型の定義、`thiserror`/`anyhow` クレートによるエラー処理の実践パターンを学ぶ。

## Goとの比較

| 概念 | Go | Rust |
|---|---|---|
| エラー型 | `error` インターフェース（1種類） | `Result<T, E>` の `E` は任意の型 |
| エラー返却 | `return 0, err` | `return Err(e)` / `?` 演算子 |
| エラー伝播 | `if err != nil { return err }` | `?` 演算子 |
| エラーラップ | `fmt.Errorf("%w", err)` | `thiserror` の `#[from]` / `anyhow` の `context()` |
| エラー判別 | `errors.Is()` / `errors.As()` | `match` で enum バリアントを判別 |
| カスタムエラー | `type MyError struct{}` + `Error() string` | `enum` + `Display` + `Error` トレイト |
| 汎用エラー | `error` インターフェース | `Box<dyn Error>` / `anyhow::Error` |

### Goとの主な違い

```go
// Go — error インターフェースで統一
func readConfig(path string) (string, error) {
    data, err := os.ReadFile(path)
    if err != nil {
        return "", fmt.Errorf("config読み込み失敗: %w", err)
    }
    return string(data), nil
}
```

```rust
// Rust — 型でエラーを区別し、? で伝播
fn read_config(path: &str) -> anyhow::Result<String> {
    let content = fs::read_to_string(path)
        .context(format!("{path} の読み込みに失敗"))?;
    Ok(content)
}
```

主な違い:
- **Goは1種類の `error`** — 全てのエラーが同じインターフェース。シンプルだが型情報を失いやすい
- **Rustは型でエラーを区別** — `enum` のバリアントで `match` できる。コンパイラが網羅性を保証
- **`?` 演算子** — `if err != nil { return err }` の3行が `?` 1文字で済む
- **ライブラリ vs アプリ** — ライブラリは `thiserror`（呼び出し側が `match` できるように）、アプリは `anyhow`（表示できればいい）を使い分ける

## コード解説

### Result を返す関数

```rust
fn parse_number(s: &str) -> Result<i32, ParseIntError> {
    s.parse::<i32>()
}
```

```go
func parseNumber(s string) (int, error) {
    return strconv.Atoi(s)
}
```

### ? 演算子によるエラー伝播

```rust
fn double_parse(s: &str) -> Result<i32, ParseIntError> {
    let n = s.parse::<i32>()?;  // エラーなら即 return Err(...)
    Ok(n * 2)
}
```

```go
func doubleParse(s string) (int, error) {
    n, err := strconv.Atoi(s)
    if err != nil {
        return 0, err
    }
    return n * 2, nil
}
```

`?` は以下の省略形:
1. `Ok(v)` なら `v` を取り出して続行
2. `Err(e)` なら `e` を変換して（`From` トレイト）即座に `return Err(e)`

### 複数のエラー型を扱う

```rust
// io::Error と ParseIntError を両方扱う
fn read_first_line_number(path: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;  // io::Error
    let first_line = content.lines().next().ok_or("ファイルが空です")?;
    let number = first_line.parse::<i32>()?;   // ParseIntError
    Ok(number)
}
```

`Box<dyn std::error::Error>` は「何でもいいから Error トレイトを実装した型」。Goの `error` インターフェースに最も近い。ただし `match` でバリアントを判別できない欠点がある。

### カスタムエラー型（手動実装）

```rust
#[derive(Debug)]
enum ValidationError {
    ParseError(ParseIntError),
    OutOfRange { value: i32, min: i32, max: i32 },
}

impl std::fmt::Display for ValidationError { ... }  // エラーメッセージ
impl std::error::Error for ValidationError { ... }   // source() で原因を辿る
impl From<ParseIntError> for ValidationError { ... }  // ? で自動変換
```

Goのカスタムエラー:
```go
type ValidationError struct {
    Field   string
    Message string
}

func (e *ValidationError) Error() string {
    return fmt.Sprintf("%s: %s", e.Field, e.Message)
}

// 判別には errors.As() を使う
var ve *ValidationError
if errors.As(err, &ve) { ... }
```

Rustの `match` はGoの `errors.As()` よりも安全で網羅的。

### thiserror — ボイラープレートを削減

```rust
#[derive(Debug, thiserror::Error)]
enum UserError {
    #[error("名前が空です")]
    EmptyName,

    #[error("年齢の解析に失敗: {0}")]
    InvalidAge(#[from] ParseIntError),  // From を自動実装

    #[error("{value} は有効な年齢ではありません ({min}〜{max})")]
    AgeOutOfRange { value: i32, min: i32, max: i32 },
}
```

- `#[error("...")]` — `Display` を自動実装
- `#[from]` — `From` トレイトを自動実装（`?` で自動変換できる）
- `#[source]` — `Error::source()` を自動実装

手動で `Display`, `Error`, `From` を書くのと同じことを数行で実現。

### anyhow — アプリケーション向け簡易エラー

```rust
fn run_app() -> anyhow::Result<()> {
    let content = fs::read_to_string("config.txt")
        .context("設定ファイルの読み込みに失敗")?;
    let number: i32 = content.trim().parse()?;
    Ok(())
}
```

- `anyhow::Result<T>` = `Result<T, anyhow::Error>`
- どんなエラー型でも `?` で変換できる
- `context()` でエラーに文脈情報を追加（Goの `fmt.Errorf("%w")` に相当）
- `anyhow::bail!("...")` でエラーを即座に返す

### thiserror vs anyhow の使い分け

| | thiserror | anyhow |
|---|---|---|
| 用途 | ライブラリ | アプリケーション |
| エラー型 | 具体的な `enum` | `anyhow::Error`（型消去） |
| 呼び出し側 | `match` で判別できる | メッセージを表示するだけ |
| Go相当 | カスタム `error` 型 | `fmt.Errorf()` で十分な場合 |

## 演習

### 演習1: 基礎

カスタムエラー型を手動で定義し、文字列を正の整数としてパースする関数を作ろう。

```rust
#[derive(Debug)]
enum PositiveIntError {
    ParseFailed(ParseIntError),
    NotPositive(i32),
}

// Display, Error, From を手動実装し、以下の関数を完成させる
fn parse_positive(input: &str) -> Result<i32, PositiveIntError> {
    todo!()
}
// parse_positive("42")  => Ok(42)
// parse_positive("-1")  => Err(NotPositive(-1))
// parse_positive("abc") => Err(ParseFailed(...))
```

### 演習2: 応用

`thiserror` を使って演習1と同等のエラー型を定義しよう。手動実装との違い（コード量）を体感する。

```rust
#[derive(Debug, thiserror::Error)]
enum PositiveIntError2 {
    #[error("数値の解析に失敗: {0}")]
    ParseFailed(#[from] ParseIntError),

    #[error("{0} は正の整数ではありません")]
    NotPositive(i32),
}

fn parse_positive2(input: &str) -> Result<i32, PositiveIntError2> {
    todo!()
}
```

### 演習3: チャレンジ

`anyhow` を使って、ファイルからCSV風データ（各行が `name,age`）を読み込む関数を作ろう。`context()` でエラーに行番号情報を追加する。

```rust
fn parse_csv(path: &str) -> anyhow::Result<Vec<(String, i32)>> {
    // 1. ファイルを読み込む（context でファイル名を追加）
    // 2. 各行を "," で分割
    // 3. 名前と年齢を取り出す
    // 4. フォーマットエラーには bail! で行番号を含める
    todo!()
}
```

## まとめ

- **`?` 演算子** は `if err != nil { return err }` を1文字で書ける
- **カスタムエラー型** は `enum` で定義し、`Display` + `Error` + `From` を実装する
- **`thiserror`** は `Display`, `Error`, `From` の手動実装を derive マクロで自動化する
- **`anyhow`** はアプリケーション向け。どんなエラーも受け入れ、`context()` で文脈を追加できる
- ライブラリには `thiserror`、アプリには `anyhow` を使い分けるのが定石
- Goは `error` インターフェース1つで統一。Rustは型でエラーを区別し、`match` で網羅的に処理できる
