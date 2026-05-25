# net-02: 簡易HTTPサーバー Q&A

日付: 2026-05-22

## Q1: `let mut parts = request_line.trim_end().splitn(3, ' ');` の3行はこれって安全なの？

レイヤー別に分けて考える必要がある。**「コードがパニックするか」という意味では安全、「攻撃に強いか」という意味では脆弱**。

```rust
let mut parts = request_line.trim_end().splitn(3, ' ');
let method = parts.next().unwrap_or("").to_string();
let path = parts.next().unwrap_or("").to_string();
let version = parts.next().unwrap_or("").to_string();
```

### ① パニック安全性 — OK

- `unwrap_or("")` を使っているので `None` でもパニックしない
- `splitn(3, ' ')` は文字列がどんな形でも必ず1個以上の要素を返すイテレータ（空でも `""` を1個返す）
- `&str` のインデックスではなく `splitn` を経由しているので、`text[5..]` のようなバイト境界エラーも起きない

どんな入力でもこの3行はパニックしない。

### ② 値の正しさ — `is_empty()` チェックでカバー

直後の以下のチェックがあるおかげで、`""` フォールバックが下流に流れ込んでも `Err` で弾かれる:

```rust
if method.is_empty() || path.is_empty() || version.is_empty() {
    return Err(...InvalidData...);
}
```

代表的な不正入力の挙動:

| 入力 | parts の中身 | 結果 |
|---|---|---|
| `"GET / HTTP/1.1"` | `["GET", "/", "HTTP/1.1"]` | OK |
| `""` (空行のみ) | `[""]` | path/version が空 → 400 |
| `"GET"` | `["GET"]` | path/version が空 → 400 |
| `"GET /"` | `["GET", "/"]` | version が空 → 400 |
| `"GET / HTTP/1.1 extra"` | `["GET", "/", "HTTP/1.1 extra"]` | **通ってしまう** (version に余計な文字) |

最後のケースだけ要注意。本来リクエスト行は厳密に3要素なので、4要素目があれば 400 にすべき:

```rust
let parts: Vec<&str> = trimmed.split(' ').collect();
if parts.len() != 3 { return Err(...); }
```

または `version` のフォーマット (`HTTP/1.1` か `HTTP/1.0`) を検証する。

### ③ DoS耐性 — **ここが弱い**

本番だと致命的な穴がいくつかある。

#### 穴1: リクエスト行に長さ制限がない

```rust
reader.read_line(&mut request_line)?;
```

`read_line` には **最大バイト数の引数がない**。攻撃者が改行なしで巨大なバイト列を送り続けると、サーバーがメモリを食い潰して落ちる。

本番では `BufRead::take(N)` で上限をかけるべき:

```rust
let mut limited = reader.take(8192);  // 8KB 上限
limited.read_line(&mut request_line)?;
```

nginx などは「リクエスト行 8KB」「ヘッダー全体 8KB」のような上限を設けている。

#### 穴2: ヘッダー本数/総サイズに上限がない

ヘッダー読み取りループも同じ問題。`X-A: 1\r\n` を100万行送られるとメモリが尽きる。

#### 穴3: スロー攻撃 (Slowloris)

`read_line` は **改行が来るまで永遠に待つ**。1秒に1バイトずつ送るだけで接続を占有し続けられる。対策は read のタイムアウト:

```rust
stream.set_read_timeout(Some(Duration::from_secs(10)))?;
```

#### 穴4: 演習3で実装するボディ読み取り

`Content-Length: 999999999999` を送られた瞬間に `vec![0u8; content_length]` で OOM。上限チェックが必要。

### ④ Go との比較

Goの `net/http` は**これら全部を内部で処理している**:

- `http.Server.MaxHeaderBytes` (デフォルト 1MB)
- `http.Server.ReadTimeout`, `WriteTimeout`, `IdleTimeout`, `ReadHeaderTimeout`
- 不正な Request-Line を自動で 400

Goで `http.HandleFunc` でルートを書いただけで Slowloris攻撃に強いのは、`net/http` 内部がちゃんと上限とタイムアウトを管理してくれているおかげ。Rust の `hyper`/`axum` も同じ。

### ⑤ 結論

| 文脈 | 評価 |
|---|---|
| ローカル 127.0.0.1 で curl やブラウザから叩く学習用 | **安全** |
| 0.0.0.0 でインターネットに公開する本番用 | **危険** (DoS の的) |

学習目的では今のままで問題ない。気になるなら「リクエスト行を 8KB に制限」「`set_read_timeout` を入れる」あたりを試すと、本番Webサーバーが内部で何をしているかの体感になる。

## Q2: レスポンスで Content-Length がわからないケースとかないの？

ある。Content-Length が使えない/不要なケースは沢山あり、HTTP/1.1 にはボディ終端を伝える方法が3種類ある。

### HTTP/1.1 のボディ終端方式

| 方式 | 終端の伝え方 | 用途 |
|---|---|---|
| Content-Length | バイト数を最初に伝える | 固定長で全体サイズが分かるとき |
| Transfer-Encoding: chunked | 各チャンクの先頭にサイズを書く | 全体サイズが事前に分からないとき |
| Connection: close | 接続を閉じることでボディ終端を示す | HTTP/1.0 互換、最後の砦 |

優先順位は **Transfer-Encoding > Content-Length > Connection: close**。両方付与すると HTTP Smuggling のリスクがあり RFC で禁止されている。

### Content-Length が使えないユースケース

#### ① 動的生成・ストリーミング

応答を作りながら送るケース。終わるまで全体サイズが分からない:

- DB のクエリ結果を1行ずつ流す
- ログのテール (`tail -f` 相当)
- LLM の応答 (トークンを生成しながら送る、ChatGPT 風)
- 大きなファイルを upstream から受けて proxy するときに、upstream が Content-Length を返してこない

→ `Transfer-Encoding: chunked` を使う。

#### ② Server-Sent Events (SSE)

長時間維持する単方向ストリーム:

```
HTTP/1.1 200 OK
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive

data: イベント1\n\n
data: イベント2\n\n
(無限に続く)
```

「いつ終わるか不明」なので Content-Length は付けようがない。

#### ③ HTTP/1.0 (Content-Length なし)

HTTP/1.0 では「接続が閉じたらボディ終了」が暗黙のルールだった。今でも古い CGI 等はこの挙動:

```
HTTP/1.0 200 OK
Content-Type: text/html

<html>...</html>
(EOF)
```

クライアントは TCP FIN を見るまで読み続ける。

#### ④ ボディがそもそも無いケース

| ケース | Content-Length |
|---|---|
| HEAD リクエストのレスポンス | informational に付けるがボディは空 |
| 204 No Content / 205 Reset Content | ボディ禁止、Content-Length 不要 |
| 304 Not Modified | ボディ禁止 |
| 1xx Informational | ボディなし |
| CONNECT トンネル成立後の 200 Connected | ボディ概念がなくなる |

### Transfer-Encoding: chunked の仕組み

サイズが事前に分からないとき、こうやって送る:

```
HTTP/1.1 200 OK
Content-Type: text/plain
Transfer-Encoding: chunked

5\r\n              ← 次のチャンクのバイト数（16進）
Hello\r\n          ← 5バイトのチャンク本体
6\r\n              ← 6バイト
 World\r\n
0\r\n              ← サイズ 0 = 終端マーカー
\r\n               ← 最終 CRLF
```

各チャンクは「サイズ(hex) + CRLF + データ + CRLF」の繰り返し。最後に `0\r\n\r\n` で終端を示す。

Go の `net/http` も Rust の `hyper` も、`Content-Length` をセットしないで書き出すと**自動で chunked encoding に切り替わる**。

### HTTP/2 と HTTP/3 では

そもそも前提が違う。

- HTTP/2 はテキストではなくバイナリフレームで通信する
- レスポンスは HEADERS フレーム → DATA フレーム... → END_STREAM フラグ付き DATA フレーム という構造
- **フレーム自体が長さ情報を持っている**ので、Content-Length は informational (あってもなくても通信成立)
- chunked encoding は HTTP/2 では禁止 (フレーミングが既にあるので不要)

HTTP/3 (QUIC) も同様にフレームベース。

### まとめ: いつ何を使うか

| 状況 | 推奨 |
|---|---|
| 全体サイズが事前に分かる静的ファイル | Content-Length |
| 動的生成、ストリーミング | Transfer-Encoding: chunked |
| SSE | chunked + text/event-stream |
| HTTP/1.0 互換 / シンプル化 | Connection: close (net-02 はこれ) |
| HEAD、204、304 | ボディなしなので Content-Length 不要 |
| HTTP/2 以降 | フレーミングが面倒見るので考えなくてOK |

### net-02 の文脈

現状のサーバーは「Connection: close + Content-Length 両方付与」。Content-Length を消して Connection: close だけにしても動く (クライアントは接続が閉じるまで読む)。

ただ Content-Length を付けるメリット:
- クライアントが事前にプログレスバーを表示できる
- keep-alive にしたいときに必須になる (接続を閉じない以上、終端マーカーが必要)

「Content-Length を付けられるなら付ける」が無難な方針。

## Q3: HTTP/2, HTTP/3 ってなに？普通のHTTPでの通信はどれにあたるの？

「普通のHTTP通信」は誰と誰の通信かで変わる。HTTP/2 以降は HTTP/1.1 と全くの別物 (テキストプロトコルではなくなる)。

### HTTP のバージョン履歴

| バージョン | 年 | 形式 | 下のレイヤー | 主な特徴 |
|---|---|---|---|---|
| HTTP/0.9 | 1991 | テキスト | TCP | `GET /` だけ。ヘッダーもステータスコードも無い |
| HTTP/1.0 | 1996 | テキスト | TCP | ヘッダー、ステータスコード導入。1リクエスト=1接続 |
| HTTP/1.1 | 1997 | テキスト | TCP | keep-alive、Host ヘッダー、chunked。**net-02 で書いているもの** |
| HTTP/2 | 2015 | バイナリ | TCP + TLS | 多重化、ヘッダー圧縮、サーバープッシュ |
| HTTP/3 | 2022 | バイナリ | **UDP** + QUIC | 接続確立が高速、TCPヘッドオブラインブロッキング解消 |

### HTTP/1.1 の限界

#### 問題1: 1接続で同時に1リクエスト

パイプライニングという機能もあったがサーバー実装が壊れがちで実質失敗。ブラウザは **1ドメインに 6本のTCP接続を張る** workaround を採用していた。

#### 問題2: ヘッダーが毎回フル送信

Cookie や User-Agent を100リクエストすれば100回送る。

### HTTP/2 (2015) が解決したこと

#### バイナリフレーム化

テキストではなくバイナリのフレームで通信:

```
[Frame Header: 9バイト][Payload: 可変長]
  - Length, Type, Flags, Stream ID
```

フレーム種別: HEADERS, DATA, SETTINGS, PING, RST_STREAM, etc.

#### 多重化 (Multiplexing)

1つのTCP接続上に複数のストリームを同時に走らせる:

```
TCP接続1本
  ├ ストリーム1: GET /a.html  (HEADERS → DATA → END_STREAM)
  ├ ストリーム3: GET /b.png   (HEADERS → DATA → DATA → END_STREAM)
  ├ ストリーム5: GET /c.css   (HEADERS → END_STREAM)
  └ ストリーム7: GET /d.js    (進行中)
```

フレーム単位でインターリーブできる。1ドメイン6接続も不要に。

#### HPACK によるヘッダー圧縮

送信側と受信側でヘッダーテーブルを共有し、2回目以降はインデックス番号だけ送る。

#### TLS 必須 (実質)

仕様上は平文でも動くが、ブラウザは HTTPS のみ実装。

### HTTP/3 (2022) が解決したこと

HTTP/2 の残った問題: **TCP のヘッドオブラインブロッキング**。

```
TCP接続1本
  ├ ストリーム1: パケット A
  ├ ストリーム3: パケット B  ← Bが途中で紛失
  ├ ストリーム5: パケット C  ← Cは届いてるのに...
  └ ストリーム7: パケット D  ← Bの再送を待たされる
```

TCP はバイトストリームを順序保証するので、1パケット落ちると後続全部が待たされる。HTTP/2 はアプリ層で多重化したが、TCP 層でブロックされる構造的問題。

#### 解決策: UDP の上に QUIC を載せる

```
HTTP/3
  └─ QUIC (信頼性、暗号化、多重化を全部担当)
       └─ UDP
            └─ IP
```

QUIC はストリームごとに独立して順序管理するので、Bが落ちても C, D は処理できる。

#### 0-RTT 接続再開

TLS 1.3 のセッション再開機能を使い、過去にやりとりしたサーバーへは初回データを最初のパケットに乗せられる。レイテンシが激減 (特にモバイル)。

### 「普通のHTTP通信」は何にあたるか

状況別:

| 通信元 ↔ 通信先 | 使われるバージョン |
|---|---|
| ブラウザ ↔ Google/YouTube/Cloudflare系 | **HTTP/3** 優先 → 失敗で HTTP/2 → HTTP/1.1 |
| ブラウザ ↔ 普通の HTTPS サイト | **HTTP/2** が標準 (TLS の ALPN で negotiate) |
| `curl http://example.com` (平文) | **HTTP/1.1** が確定 (TLS なしでは HTTP/2/3 は実質使われない) |
| 社内マイクロサービス間 | HTTP/1.1 がまだ多数派、gRPC は HTTP/2 |
| net-02 (今書いているサーバー) | **HTTP/1.1** (text + TCP + 平文) |

ブラウザが TLS で接続するとき、ハンドシェイク中の **ALPN (Application-Layer Protocol Negotiation)** で「h2 が話せるか」を確認し、できれば HTTP/2、無理なら HTTP/1.1 にフォールバックする。

### 確認方法

Chrome / Edge:
- DevTools → Network タブ → 列に "Protocol" を追加
- `http/1.1`, `h2`, `h3` のいずれかが表示される

curl:
```bash
curl -I --http2 https://www.google.com
curl -sI -o /dev/null -w "%{http_version}\n" https://www.google.com
# → 3 (= HTTP/3) など
```

### 実装の現実

| 言語/ライブラリ | HTTP/1.1 | HTTP/2 | HTTP/3 |
|---|---|---|---|
| Rust `std::net` | 自分で書ける | 自分で書くのは困難 | 不可能 |
| Rust `hyper` | ✅ | ✅ | `h3` クレートで対応 |
| Rust `reqwest` | ✅ | ✅ (自動) | 機能フラグで対応 |
| Go `net/http` | ✅ | ✅ (自動) | 標準では未対応、`quic-go` 等 |
| Node.js | ✅ | ✅ | 実験的 |
| nginx | ✅ | ✅ | 1.25+ で対応 |

**HTTP/2 以降は手書きで実装するのは現実的でない**。バイナリフレーミング、HPACK、QUIC など複雑すぎる。net-02 で HTTP/1.1 を手書きするのは「テキストプロトコルだから可能」という特別な事情。

### まとめ

| 用語 | 現実的な意味 |
|---|---|
| 「普通のHTTP」 | 文脈次第。ブラウザなら今は HTTP/2 か HTTP/3、curl のデフォルトや学習用は HTTP/1.1 |
| HTTP/1.1 | テキスト+TCP。**手で書ける唯一のバージョン**。net-02 がこれ |
| HTTP/2 | バイナリ+TCP+TLS。多重化とヘッダー圧縮。ライブラリ必須 |
| HTTP/3 | バイナリ+UDP+QUIC。レイテンシ最小化。ライブラリ必須 |

カリキュラム的には、net-02 で HTTP/1.1 のテキストを手書きで触る → net-04 で TLS を理解する → そこまで来れば HTTP/2/3 の話が地続きで理解できる構造。
