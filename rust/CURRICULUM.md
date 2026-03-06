# Rust学習カリキュラム — Go経験者向け

Go経験者がRustをゼロから学ぶためのカリキュラム。各トピックはGoとの比較を含み、小さく独立したCargoプロジェクトとして管理する。

## トピック一覧

| # | ディレクトリ | 内容 | Go との比較 |
|---|---|---|---|
| 01 | `01-hello-cargo` | Cargo基本、プロジェクト構成、ビルド | `go mod init` / `go run` に相当 |
| 02 | `02-variables-and-types` | let/mut、型推論、シャドウイング、タプル | Goは常にmutable。タプルはGoにない |
| 03 | `03-functions-and-control-flow` | 関数、式ベースのreturn、loop/while/for | Goは`for`のみ。式ベースreturnはGoにない |
| 04 | `04-ownership` | 所有権、ムーブ、Copy、スコープベースの解放 | **Rust固有**。GoはGCで管理 |
| 05 | `05-borrowing` | &T, &mut T、借用ルール | Goポインタに似るが借用チェッカーはRust固有 |
| 06 | `06-structs-and-methods` | 構造体、impl、メソッド、関連関数 | Go構造体+レシーバに近い |
| 07 | `07-enums-and-matching` | enum(代数的データ型)、match、Option、Result、`?` | Goのiota/error/`if err != nil`を置き換え |
| 08 | `08-traits-and-generics` | トレイト、ジェネリクス、derive | Goインターフェースに近いが明示的 |
| 09 | `09-error-handling` | カスタムエラー型、thiserror/anyhow | Go error型に相当だが型システムが豊富 |
| 10 | `10-collections-and-iterators` | Vec, HashMap、イテレータチェーン | Goのslice/mapに相当。イテレータはGoにない |
| 11 | `11-lifetimes` | ライフタイム注釈、省略規則、`'static` | **Rust固有**。GC言語には不要な概念 |
| 12 | `12-closures` | Fn/FnMut/FnOnce、moveクロージャ | Go無名関数に近いが所有権と連動 |
| 13 | `13-concurrency` | thread::spawn、mpsc、Arc<Mutex<T>>、Send/Sync | goroutine/channelに相当。コンパイル時データ競合防止はRust固有 |
| 14 | `14-async-await` | async/await、Tokio、非同期HTTP | Goは言語組込み並行性。Rustはライブラリ (Tokio) |

## 難易度カーブ (Go開発者視点)

- **01-03**: 快適 — 構文の違い程度
- **04-05**: パラダイムシフト — 所有権と借用は完全に新しい概念
- **06-08**: 中程度 — Goに類似概念あるが所有権が絡む
- **09-10**: 中程度 — エラー処理は馴染みあり、イテレータは新しい
- **11**: 難 — GC言語開発者に最も馴染みがない概念
- **12-14**: 中〜難 — クロージャ+所有権、並行性+Send/Sync

## 各トピックの構成

各トピックは独立したCargoプロジェクトとして以下の構成をとる:

```
XX-topic-name/
├── Cargo.toml
├── README.md          ← 概念説明 + Go比較 + 演習
└── src/main.rs        ← サンプルコード + 演習の雛形
```

### README.md の構成

1. **概要** — トピックの簡潔な説明
2. **Goとの比較** — Go経験者が理解しやすいように対比
3. **コード解説** — `src/main.rs` のコードを段階的に解説
4. **演習** — 理解を確認するための課題 (段階的に難易度を上げる)
5. **まとめ** — 学んだポイントの箇条書き

## 進め方

1. 各トピックのREADME.mdを読む
2. `src/main.rs` のコードを読み、`cargo run` で実行する
3. 演習に取り組む
4. 理解できたら次のトピックへ進む
