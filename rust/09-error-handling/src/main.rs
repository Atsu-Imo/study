// 09 - エラーハンドリング
// Result、カスタムエラー型、thiserror、anyhow を学ぶ

use std::fs;
use std::num::ParseIntError;

fn main() {
    // === Result の基本（復習） ===
    // Goの (value, error) タプルに相当
    // 07で学んだ Result<T, E> をエラーハンドリングの視点で深掘りする
    let input = "42";
    match parse_number(input) {
        Ok(n) => println!("{input} をパース: {n}"),
        Err(e) => println!("パースエラー: {e}"),
    }

    let bad_input = "abc";
    match parse_number(bad_input) {
        Ok(n) => println!("{bad_input} をパース: {n}"),
        Err(e) => println!("パースエラー: {e}"),
    }

    // === ? 演算子でエラーを伝播 ===
    // Goの if err != nil { return err } に相当
    match double_parse("21") {
        Ok(n) => println!("2倍: {n}"),
        Err(e) => println!("エラー: {e}"),
    }

    // === 複数のエラー型を扱う ===
    // Goでは全て error インターフェースだが、Rustは型が異なる
    match read_first_line_number("Cargo.toml") {
        Ok(n) => println!("最初の行の数値: {n}"),
        Err(e) => println!("エラー（想定通り）: {e}"),
    }

    // === カスタムエラー型（手動実装） ===
    match validate_age("17") {
        Ok(age) => println!("年齢OK: {age}"),
        Err(e) => println!("バリデーションエラー: {e}"),
    }
    match validate_age("abc") {
        Ok(age) => println!("年齢OK: {age}"),
        Err(e) => println!("バリデーションエラー: {e}"),
    }
    match validate_age("200") {
        Ok(age) => println!("年齢OK: {age}"),
        Err(e) => println!("バリデーションエラー: {e}"),
    }

    // === thiserror を使ったカスタムエラー型 ===
    // 手動実装のボイラープレートを derive マクロで解消
    match create_user("", "25") {
        Ok(user) => println!("ユーザー作成: {} ({}歳)", user.name, user.age),
        Err(e) => println!("ユーザー作成エラー: {e}"),
    }
    match create_user("Taro", "abc") {
        Ok(user) => println!("ユーザー作成: {} ({}歳)", user.name, user.age),
        Err(e) => println!("ユーザー作成エラー: {e}"),
    }
    match create_user("Taro", "25") {
        Ok(user) => println!("ユーザー作成: {} ({}歳)", user.name, user.age),
        Err(e) => println!("ユーザー作成エラー: {e}"),
    }

    // === anyhow を使った簡易エラーハンドリング ===
    // アプリケーションコード向け。ライブラリでは thiserror を使う
    match run_app() {
        Ok(()) => println!("アプリ正常終了"),
        Err(e) => println!("アプリエラー: {e}"),
    }

    // === エラーのチェーン（原因の連鎖） ===
    match read_config("nonexistent.toml") {
        Ok(content) => println!("設定: {content}"),
        Err(e) => {
            println!("エラー: {e}");
            // source() で原因を辿れる
            // Goの errors.Unwrap() に相当
            if let Some(source) = std::error::Error::source(&*e) {
                println!("  原因: {source}");
            }
        }
    }

    // === 演習 ===
    exercises();
}

// === Result を返す関数 ===
// Go: func parseNumber(s string) (int, error)
fn parse_number(s: &str) -> Result<i32, ParseIntError> {
    s.parse::<i32>()
}

// === ? 演算子 ===
// ? はエラーの場合に早期リターンする
// Go:
//   n, err := parseNumber(s)
//   if err != nil { return 0, err }
//   return n * 2, nil
fn double_parse(s: &str) -> Result<i32, ParseIntError> {
    let n = s.parse::<i32>()?; // エラーなら即 return Err(...)
    Ok(n * 2)
}

// === 複数のエラー型を扱う ===
// ファイル読み込み → std::io::Error
// 数値パース → ParseIntError
// これらを1つの関数で扱うには、共通のエラー型が必要
//
// Go: 全て error インターフェースなので問題にならない
// Rust: 型が違うのでそのまま ? を使えない → Box<dyn Error> で統一
fn read_first_line_number(path: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?; // io::Error → Box<dyn Error> に自動変換
    let first_line = content.lines().next().ok_or("ファイルが空です")?;
    let number = first_line.parse::<i32>()?; // ParseIntError → Box<dyn Error> に自動変換
    Ok(number)
}

// === カスタムエラー型（手動実装） ===
// Go:
//   type ValidationError struct {
//       Field   string
//       Message string
//   }
//   func (e *ValidationError) Error() string { ... }
//
// Rust: enum でバリアントごとに異なるエラーを表現
#[derive(Debug)]
enum ValidationError {
    ParseError(ParseIntError),
    OutOfRange { value: i32, min: i32, max: i32 },
}

// Display トレイトを実装 → エラーメッセージを定義
impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::ParseError(e) => write!(f, "数値の解析に失敗: {e}"),
            ValidationError::OutOfRange { value, min, max } => {
                write!(f, "{value} は範囲外です ({min}〜{max})")
            }
        }
    }
}

// Error トレイトを実装（source() で原因を辿れるようにする）
impl std::error::Error for ValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ValidationError::ParseError(e) => Some(e),
            ValidationError::OutOfRange { .. } => None,
        }
    }
}

// From トレイトを実装 → ? 演算子で ParseIntError を自動変換
impl From<ParseIntError> for ValidationError {
    fn from(e: ParseIntError) -> Self {
        ValidationError::ParseError(e)
    }
}

fn validate_age(input: &str) -> Result<i32, ValidationError> {
    let age: i32 = input.parse()?; // ParseIntError → ValidationError に自動変換
    if !(0..=150).contains(&age) {
        return Err(ValidationError::OutOfRange {
            value: age,
            min: 0,
            max: 150,
        });
    }
    Ok(age)
}

// === thiserror を使ったカスタムエラー型 ===
// 上の手動実装を derive マクロで簡潔に書ける
// Display, Error, From を自動生成してくれる

#[derive(Debug, thiserror::Error)]
enum UserError {
    #[error("名前が空です")]
    EmptyName,

    #[error("年齢の解析に失敗: {0}")]
    InvalidAge(#[from] ParseIntError),

    #[error("{value} は有効な年齢ではありません ({min}〜{max})")]
    AgeOutOfRange { value: i32, min: i32, max: i32 },
}

struct User {
    name: String,
    age: i32,
}

fn create_user(name: &str, age_str: &str) -> Result<User, UserError> {
    if name.is_empty() {
        return Err(UserError::EmptyName);
    }
    let age: i32 = age_str.parse()?; // #[from] のおかげで自動変換
    if !(0..=150).contains(&age) {
        return Err(UserError::AgeOutOfRange {
            value: age,
            min: 0,
            max: 150,
        });
    }
    Ok(User {
        name: name.to_string(),
        age,
    })
}

// === anyhow を使った簡易エラーハンドリング ===
// anyhow::Result は Box<dyn Error> の上位互換
// アプリケーションコードで「エラーの型は気にしない、メッセージが出ればいい」時に便利
//
// Go: ほぼ全てのコードが error インターフェースだけで済む → anyhow に近い感覚
// ライブラリ: thiserror（呼び出し側がエラーを match したいから）
// アプリ: anyhow（エラーを表示するだけでいいから）
fn run_app() -> anyhow::Result<()> {
    let content = fs::read_to_string("nonexistent.txt")
        .map_err(|e| anyhow::anyhow!("設定ファイルの読み込みに失敗: {e}"))?;
    let _number: i32 = content.trim().parse()?;
    Ok(())
}

// === エラーのチェーン ===
// anyhow の context() でエラーに文脈情報を追加
// Go: fmt.Errorf("config読み込み失敗: %w", err) に相当
use anyhow::Context;

fn read_config(path: &str) -> anyhow::Result<String> {
    let content = fs::read_to_string(path)
        .context(format!("{path} の読み込みに失敗"))?;
    Ok(content)
}

// ============================================================
// 演習
// ============================================================

// --- 演習1: 基礎 ---
// 文字列を受け取り、正の整数かどうか検証する関数を作ろう
// - パースに失敗したらエラー
// - 0以下ならエラー
// カスタムエラー型を手動で定義してみよう
//
// #[derive(Debug)]
// enum PositiveIntError {
//     ParseFailed(ParseIntError),
//     NotPositive(i32),
// }
//
// fn parse_positive(input: &str) -> Result<i32, PositiveIntError> {
//     todo!()
// }

// --- 演習2: 応用 ---
// thiserror を使って、演習1と同等のエラー型を定義しよう
// 手動実装との違い（コード量）を体感する
//
// #[derive(Debug, thiserror::Error)]
// enum PositiveIntError2 {
//     #[error("数値の解析に失敗: {0}")]
//     ParseFailed(#[from] ParseIntError),
//
//     #[error("{0} は正の整数ではありません")]
//     NotPositive(i32),
// }
//
// fn parse_positive2(input: &str) -> Result<i32, PositiveIntError2> {
//     todo!()
// }

// --- 演習3: チャレンジ ---
// anyhow を使って、ファイルからCSV風データを読み込み処理する関数を作ろう
// ファイルの各行が "name,age" の形式。パースして Vec<(String, i32)> を返す
// context() でエラーに行番号情報を追加する
//
// fn parse_csv(path: &str) -> anyhow::Result<Vec<(String, i32)>> {
//     let content = fs::read_to_string(path)
//         .context(format!("{path} の読み込みに失敗"))?;
//     let mut results = Vec::new();
//     for (i, line) in content.lines().enumerate() {
//         let parts: Vec<&str> = line.split(',').collect();
//         if parts.len() != 2 {
//             anyhow::bail!("{}行目: フォーマットが不正です: {}", i + 1, line);
//         }
//         let name = parts[0].to_string();
//         let age: i32 = parts[1].parse()
//             .context(format!("{}行目: 年齢のパースに失敗", i + 1))?;
//         results.push((name, age));
//     }
//     Ok(results)
// }

fn exercises() {
    // 演習の動作確認をここに書く
    // 例:
    // println!("{:?}", parse_positive("42"));
    // println!("{:?}", parse_positive("-1"));
    // println!("{:?}", parse_positive("abc"));
}
