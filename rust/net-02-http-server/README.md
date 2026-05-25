# net-02: 簡易HTTPサーバー

## 概要

ネットワークの「登場人物」第2弾は **Webサーバー**（Apache や nginx の役割）。

`net-01` で TCP の上で「行ベースのオレオレプロトコル」を作ったが、世の中の Web は **HTTP** という共通プロトコルで会話している。このプロジェクトでは `TcpListener` から手書きでHTTPリクエストをパースし、レスポンスを組み立てて返すサーバーを作る。

**最大のポイント:**
> HTTPは「TCPの上に乗ったテキストプロトコル」にすぎない。

魔法ではない。改行で区切られた文字列を読んで、改行で区切られた文字列を返すだけ。

## HTTPプロトコルの基本構造

### リクエスト

```
GET /hello HTTP/1.1\r\n          ← リクエスト行 (method path version)
Host: 127.0.0.1:7878\r\n         ← ヘッダー
User-Agent: curl/8.0\r\n         ← ヘッダー
Accept: */*\r\n                  ← ヘッダー
\r\n                             ← 空行 = ヘッダー終了
(任意のボディ)                    ← POST/PUT のときだけ
```

3パートある:

1. **リクエスト行**: `GET /hello HTTP/1.1` のように `メソッド パス バージョン` の3要素
2. **ヘッダー**: `Key: Value` 形式が改行区切りで続く。空行で終端
3. **ボディ**: GETなら通常なし。POST/PUTでフォーム値やJSONが入る

行の区切りは `\r\n` (CRLF) であることに注意。Unix流の `\n` だけだと厳密には違反。

### レスポンス

```
HTTP/1.1 200 OK\r\n              ← ステータス行
Content-Type: text/plain\r\n     ← ヘッダー
Content-Length: 13\r\n           ← ヘッダー
Connection: close\r\n            ← ヘッダー
\r\n                             ← 空行
Hello, world!                    ← ボディ
```

同じく3パート:

1. **ステータス行**: `HTTP/1.1 200 OK` のように `バージョン ステータスコード 理由フレーズ`
2. **ヘッダー**: リクエストと同じ形式
3. **ボディ**: 実際のコンテンツ

### よく使うステータスコード

| 範囲 | カテゴリ | 例 |
|---|---|---|
| 1xx | 情報 | 100 Continue, 101 Switching Protocols |
| 2xx | 成功 | **200 OK**, 201 Created, 204 No Content |
| 3xx | リダイレクト | 301 Moved Permanently, 302 Found, 304 Not Modified |
| 4xx | クライアントエラー | 400 Bad Request, **404 Not Found**, 403 Forbidden |
| 5xx | サーバーエラー | 500 Internal Server Error, 503 Service Unavailable |

### よく使うヘッダー

| ヘッダー | 説明 |
|---|---|
| `Host` | リクエスト先のホスト名 (HTTP/1.1で必須) |
| `Content-Type` | ボディのMIMEタイプ (`text/html`, `application/json` など) |
| `Content-Length` | ボディのバイト数 |
| `User-Agent` | クライアントの種類 (ブラウザ名など) |
| `Connection` | `close` か `keep-alive`。接続を再利用するか |

## Goとの比較

Goには `net/http` パッケージがあって、ふつうはそれを使う:

```go
// Go: 標準ライブラリ net/http
http.HandleFunc("/hello", func(w http.ResponseWriter, r *http.Request) {
    fmt.Fprintln(w, "Hello, world!")
})
http.ListenAndServe("127.0.0.1:7878", nil)
```

リクエスト行のパース、ヘッダーパース、レスポンスの組み立てなど **HTTPプロトコルの面倒な部分は全部 net/http がやってくれる**。

Rustも `hyper` / `axum` / `warp` などのクレートを使えば同じレベルの抽象化がある。だが今回は **HTTPの中身を理解するため**にあえて `std::net::TcpListener` から手書きする。Goでいうと `net.Listen("tcp", ...)` で受けて自分でリクエスト行を `bufio.Scanner` で読むのに相当。

| 概念 | Go (net/http) | Rust (このプロジェクト) |
|---|---|---|
| サーバー起動 | `http.ListenAndServe` | `TcpListener::bind` + `incoming()` |
| ルーティング | `http.HandleFunc` | `match (method, path) { ... }` |
| リクエスト | `*http.Request` (パース済) | `BufReader::read_line` で自分でパース |
| レスポンス | `http.ResponseWriter` | `format!()` で生バイト列を組み立て |

## コード解説

### 1. リクエストのパース

```rust
fn parse_request<R: BufRead>(reader: &mut R) -> std::io::Result<Option<HttpRequest>> {
    // 1行目: リクエスト行
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut parts = request_line.trim_end().splitn(3, ' ');
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    let version = parts.next().unwrap_or("").to_string();

    // ヘッダー（空行が来るまで）
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() { break; }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    // ...
}
```

ポイント:

- `read_line` は **`\n` まで読む** (`\r` 込みで返ってくる)。なので `trim_end_matches(['\r', '\n'])` で両方落とす
- ヘッダーの区切り文字は `:`。値の先頭にスペースが入る (`Host: 127.0.0.1`) ので `trim` する
- 空行が来たらヘッダー終了

### 2. レスポンスの組み立て

```rust
fn to_bytes(&self) -> Vec<u8> {
    let body_bytes = self.body.as_bytes();
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {ctype}\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\
         \r\n",
        // ...
    );
    let mut out = header.into_bytes();
    out.extend_from_slice(body_bytes);
    out
}
```

ポイント:

- **`Content-Length` は必須**。これがないとクライアントは「ボディがどこまで続くか」分からない
- `Connection: close` を付けると、レスポンスを送り終わったら接続を閉じる宣言になる。HTTP/1.1のデフォルトは keep-alive (接続再利用) だが、今回はシンプルさのために常に閉じる
- 末尾の `\r\n` でヘッダーとボディを区切るのを忘れずに

### 3. ルーティング

```rust
match (req.method.as_str(), path_only) {
    ("GET", "/")        => HttpResponse::html("..."),
    ("GET", "/hello")   => HttpResponse::ok("Hello, world!\n"),
    ("GET", "/headers") => { ... }
    _                   => HttpResponse::not_found(),
}
```

`(method, path)` のタプルで match するのは Rust らしい書き方。`POST /submit` を追加するときも同じ枠で書ける。

## 動作確認

### デモモード（推奨）

```bash
cd net-02-http-server
cargo run
```

自前クライアントで `/`, `/hello`, `/headers`, `/nonexistent` を順に叩く。

### 個別実行

```bash
# サーバーだけ起動
cargo run -- server

# 別ターミナルで:
cargo run -- client /          # 自前クライアント
cargo run -- client /hello

# curl でも当然動く
curl -v http://127.0.0.1:7878/
curl http://127.0.0.1:7878/hello

# ブラウザで http://127.0.0.1:7878/ を開く
```

### 通信内容を観察する

`curl -v` でリクエスト・レスポンスの生テキストが見られる:

```
> GET /hello HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/8.0
> Accept: */*
>
< HTTP/1.1 200 OK
< Content-Type: text/plain; charset=utf-8
< Content-Length: 14
< Connection: close
<
Hello, world!
```

これがまさにこのサーバーが受け取り、組み立てている文字列。

## 実装上の注意

### 改行コードは `\r\n`

HTTP/1.1 は **CRLF** が正式。`\n` だけだとブラウザによっては動くが厳密には違反。

### `Content-Length` の重要性

ボディのバイト数を正確に書かないと:
- 多く書きすぎる → クライアントが追加バイトを待ってタイムアウト
- 少なく書きすぎる → ボディの一部が切り捨てられる

UTF-8文字列の場合、**バイト数 ≠ 文字数** なので `s.len()`（バイト数）を使う。`s.chars().count()`（文字数）ではない。

### `Connection: close` を返す理由

このサーバーは **1リクエスト処理したら接続を閉じる**設計にしている。HTTP/1.1の本来の挙動 (keep-alive で複数リクエストを処理) を実装すると、

- 次のリクエストを待つループ
- 適切なタイムアウト
- パイプライニングの考慮

など複雑になるため、学習目的では `close` で割り切る。

## 演習

### 演習1: 基礎 — 新しいエンドポイントを追加

`route` 関数に2つのエンドポイントを追加しよう:

| パス | 応答 |
|---|---|
| `GET /time` | 現在のUNIX時刻 (例: `UNIX time: 1716345678秒`) |
| `GET /ip` | クライアントのIPアドレス (例: `127.0.0.1:55432`) |

ヒント:
- 時刻取得は `std::time::SystemTime::now().duration_since(UNIX_EPOCH)`
- `/ip` を実装するには `handle_client` から `peer_addr` を `route` に渡すように拡張が必要

### 演習2: 応用 — クエリパラメータ

`GET /echo?msg=hello` のように `?` 以降のクエリパラメータを処理しよう。

| リクエスト | 応答 |
|---|---|
| `GET /echo?msg=hello` | `hello\n` |
| `GET /echo?msg=world` | `world\n` |
| `GET /echo` (msgなし) | 400 Bad Request |

ヒント:
- `req.path` は `"/echo?msg=hello"` のように `?` 以降も含む文字列
- `split_once('?')` でパス部分とクエリ部分に分ける
- クエリは `key1=val1&key2=val2` 形式。`split('&')` → `split_once('=')` で分解

### 演習3: チャレンジ — POSTフォーム処理

ブラウザからフォームを送れるようにする:

| パス | 応答 |
|---|---|
| `GET /form` | HTMLフォーム (`<form method="POST" action="/submit">`) |
| `POST /submit` | フォーム内容を表示 (`name=alice&age=30` を整形して返す) |

難所:
- POSTにはボディがある → `Content-Length` ヘッダーを見て、その分のバイトを `read_exact` で読む必要がある
- 現在の `parse_request` はヘッダーまでしか読まないので、ボディ読み取りを追加する

ブラウザで `http://127.0.0.1:7878/form` を開き、フォームを送信して結果が表示されれば成功。

## まとめ

- **HTTPはTCP上のテキストプロトコル**。改行で区切られた文字列の往復にすぎない
- リクエスト = リクエスト行 + ヘッダー + 空行 + (ボディ)
- レスポンス = ステータス行 + ヘッダー + 空行 + ボディ
- 改行コードは `\r\n` (CRLF)
- `Content-Length` は必須。ボディの長さを正確に伝える
- ステータスコードでクライアントに結果のカテゴリを伝える (2xx成功、4xxクライアント側ミス、5xxサーバー側ミス)
- Goの `net/http` や Rustの `hyper`/`axum` は、この面倒な部分をラップしてくれる存在
