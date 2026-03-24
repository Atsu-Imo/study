# net-01: TCPエコーサーバー

## 概要

ネットワークプログラミングの第一歩として、**TCPエコーサーバー**を作る。クライアントが送ったデータをそのまま返すだけのシンプルなサーバーだが、この中にネットワーク通信の基本概念がすべて詰まっている。

## そもそもネットワーク通信とは？

2つのプログラムがデータをやり取りすること。同じPC上でも、地球の裏側のサーバーとでも、仕組みは同じ。

### 通信するために必要な3つの情報

```
┌─────────────────────────────────────────────────┐
│  「誰に」 「どこに」 「どうやって」                     │
│                                                 │
│   IPアドレス  ポート番号   プロトコル                  │
│   127.0.0.1   :7878      TCP                    │
└─────────────────────────────────────────────────┘
```

| 概念 | 例え | 説明 |
|---|---|---|
| **IPアドレス** | 建物の住所 | ネットワーク上のコンピュータを特定する番号。`127.0.0.1` は「自分自身」を指す特別なアドレス（ループバック） |
| **ポート番号** | 部屋番号 | 1台のコンピュータ上で複数のサービスを区別する番号（0〜65535）。Webは80/443、SSHは22 など |
| **プロトコル** | 会話のルール | データをどうやり取りするかの約束事。TCPとUDPが代表的 |

### TCP（Transmission Control Protocol）とは

**「信頼性のある通信」を提供するプロトコル**。以下を保証する：

- **データが届く** — 届かなかったら自動で再送する
- **順番が保たれる** — 送った順にデータが届く
- **重複しない** — 同じデータが2回届くことはない

#### TCP接続の流れ（3ウェイハンドシェイク）

通信を始める前に、まず「接続」を確立する。これが3ウェイハンドシェイク：

```
クライアント                     サーバー
    │                              │
    │  1. SYN（接続したいです）      │
    │─────────────────────────────>│
    │                              │
    │  2. SYN-ACK（OKです、こちらも）│
    │<─────────────────────────────│
    │                              │
    │  3. ACK（了解、接続開始！）    │
    │─────────────────────────────>│
    │                              │
    │     ← データの送受信 →        │
    │                              │
```

現実の例え：電話をかけるのに似ている。
1. 「もしもし」（SYN）
2. 「はい、もしもし」（SYN-ACK）
3. 「つながりましたね」（ACK）
4. 会話開始

### ソケットとは

**ソケット = プログラムがネットワーク通信するための窓口**

OSが提供するAPI。プログラムはソケットを通じてネットワークにアクセスする。ファイルの読み書きに似ている：

```
ファイル:     open → read/write → close
ソケット:     bind/connect → read/write → close
```

## サーバー・クライアントモデル

```
┌────────────┐                    ┌────────────┐
│  サーバー    │                    │ クライアント │
│            │                    │            │
│ 1. bind    │  ← ポートを確保    │            │
│ 2. listen  │  ← 接続を待つ      │            │
│ 3. accept  │ <─── connect ──── │ 1. connect │
│ 4. read    │ <─── write ────── │ 2. write   │
│ 5. write   │ ──── read ─────> │ 3. read    │
│ 6. close   │                    │ 4. close   │
└────────────┘                    └────────────┘
```

| ステップ | 説明 | Rustのコード |
|---|---|---|
| `bind` | 「このアドレス:ポートを使います」とOSに宣言 | `TcpListener::bind("127.0.0.1:7878")` |
| `listen` | 接続要求の待ち受けを開始（bindに含まれる） | 同上 |
| `accept` | クライアントからの接続を1つ受け入れる | `listener.incoming()` |
| `connect` | サーバーに接続を要求 | `TcpStream::connect("127.0.0.1:7878")` |
| `read/write` | データの送受信 | `stream.read()` / `stream.write_all()` |

## Goとの比較

### サーバー側

```go
// Go
ln, _ := net.Listen("tcp", "127.0.0.1:7878")
for {
    conn, _ := ln.Accept()
    go handleConn(conn)  // goroutineで並行処理
}
```

```rust
// Rust
let listener = TcpListener::bind("127.0.0.1:7878")?;
for stream in listener.incoming() {
    let stream = stream?;
    thread::spawn(|| handle_client(stream));  // スレッドで並行処理
}
```

### クライアント側

```go
// Go
conn, _ := net.Dial("tcp", "127.0.0.1:7878")
fmt.Fprintln(conn, "Hello")
```

```rust
// Rust
let mut stream = TcpStream::connect("127.0.0.1:7878")?;
stream.write_all(b"Hello\n")?;
```

### 主な違い

| 概念 | Go | Rust |
|---|---|---|
| TCP リスナー | `net.Listen("tcp", addr)` | `TcpListener::bind(addr)` |
| 接続 | `net.Dial("tcp", addr)` | `TcpStream::connect(addr)` |
| 並行処理 | `go func(){}()` | `thread::spawn(\|\| {})` |
| 読み取り | `bufio.NewReader(conn)` | `BufReader::new(&stream)` |
| エラー処理 | `if err != nil` | `match` / `?` |
| 接続の型 | `net.Conn`（インターフェース） | `TcpStream`（具体型） |

## コード解説

### サーバーの動作

```rust
// 1. バインド: ポート7878で待ち受け
let listener = TcpListener::bind("127.0.0.1:7878")?;

// 2. 接続受付ループ
for stream in listener.incoming() {
    let stream = stream?;
    // 3. 各クライアントを別スレッドで処理
    thread::spawn(|| handle_client(stream));
}
```

`listener.incoming()` は**ブロッキング**イテレータ。新しい接続が来るまで待ち続ける。

### クライアント処理

```rust
fn handle_client(stream: TcpStream) {
    let reader = BufReader::new(&stream);
    let mut writer = stream.try_clone()?;

    for line in reader.lines() {
        let text = line?;
        // 受け取った文字列をそのまま返す
        writer.write_all(format!("{text}\n").as_bytes())?;
    }
}
```

`BufReader` は内部バッファを持ち、`lines()` で行単位の読み取りを提供する。
`try_clone()` でストリームを複製しているのは、`BufReader` が `&stream` を借用しているため、書き込み用に別のハンドルが必要だから。

### デモモード

```bash
cargo run           # サーバー起動→クライアント自動実行
cargo run -- server # サーバーのみ起動
cargo run -- client # クライアントのみ実行
```

## 重要な概念のまとめ

### 127.0.0.1（ループバックアドレス）

自分自身を指す特別なIPアドレス。外部ネットワークを通らないので、ネットワークがなくても通信できる。開発・テストに便利。`localhost` とも呼ばれる。

### ポート番号

- **0〜1023**: Well-known ポート（HTTP=80, HTTPS=443, SSH=22）
- **1024〜49151**: 登録済みポート（MySQL=3306, PostgreSQL=5432）
- **49152〜65535**: 動的/プライベートポート

サーバーは固定ポートで待ち受け、クライアントはOSが自動で割り当てた動的ポートを使う。

### ストリーム指向通信

TCPは**バイトストリーム**。「メッセージ」の境界はない。送った順にバイトが流れてくる。
だから行単位で通信したいときは `\n` で区切る必要がある（`BufReader::lines()` がやっている）。

```
送信側が "Hello\nWorld\n" を送ると...
受信側では "Hello\n" と "World\n" が来るとは限らない。
"Hell" + "o\nWor" + "ld\n" のように分かれることもある。
→ だから BufReader で \n まで貯めてから処理する
```

## 実行方法

```bash
# デモモード（サーバーとクライアントが自動で動く）
cd net-01-tcp-echo
cargo run

# 2つのターミナルで別々に動かす場合:
# ターミナル1
cargo run -- server

# ターミナル2
cargo run -- client
```

## 演習

### 演習1: 基礎 — SHOUTサーバー

エコーサーバーを改造して、受け取った文字列を**大文字にして返す**「SHOUTサーバー」を作ろう。

```
クライアント送信: hello world
サーバー応答:     HELLO WORLD
```

ヒント: `String::to_uppercase()` を使う

### 演習2: 応用 — 対話型クライアント

標準入力からユーザーの入力を読み取り、サーバーに送って応答を表示する**対話型クライアント**を作ろう。

```
> hello        ← ユーザー入力
< hello        ← サーバーからの応答
> rust rocks
< rust rocks
>              ← 空行で終了
```

ヒント: `std::io::stdin().lock().lines()` で標準入力を1行ずつ読める

### 演習3: チャレンジ — コマンドサーバー

サーバーにコマンド機能を追加しよう:

| コマンド | 応答 |
|---|---|
| `TIME` | 現在時刻（例: `2024-01-15 12:34:56`） |
| `ECHO hello` | `hello`（ECHO 以降をそのまま返す） |
| `QUIT` | 接続を切断 |
| その他 | `UNKNOWN COMMAND` |

ヒント: `text.starts_with("ECHO ")`, `text.as_str()` でマッチング

## まとめ

- **IPアドレス**はコンピュータの住所、**ポート番号**は部屋番号
- **TCP**は信頼性のある通信（順序保証、再送あり）
- **3ウェイハンドシェイク**で接続を確立してからデータを送る
- **ソケット**はプログラムがネットワーク通信するためのOS API
- サーバーは `bind → listen → accept` 、クライアントは `connect`
- TCPは**バイトストリーム**なので、メッセージ境界は自分で決める（改行など）
- `127.0.0.1` は自分自身を指すアドレス（開発に便利）
