# drill-03: HTTP ミドルウェア

| 項目 | 内容 |
|---|---|
| タイプ | 実装 |
| 想定時間 | 90〜120分 |
| 使ってよいもの | 標準ライブラリのみ |

## 状況

注文管理サービスの HTTP 層。ハンドラは3本ある。

```
                    RequestID
                   ┌──────────────────────────────────┐
                   │        AccessLog                 │
                   │  ┌────────────────────────────┐  │
   リクエスト ──────▶ │  │      Recover             │  │
                   │  │  ┌──────────────────────┐  │  │
                   │  │  │  ServeMux → ハンドラ   │  │  │
                   │  │  └──────────────────────┘  │  │
                   │  └────────────────────────────┘  │
                   └──────────────────────────────────┘
```

ハンドラ (`handlers.go`) には**業務のことしか書かれていない**。ログを出す行も、
`recover()` も、リクエストIDを発行する行もない。それらは全リクエストに共通する処理なので、
ハンドラ側に散らすと3本が3本とも同じ定型文を持つことになり、
1本書き忘れるとそこだけログが消える。

そこで横断的な処理はミドルウェアとして1箇所に置く。`server.go` の `Handler()` が
その組み立てで、**すでに書かれている**。`middleware.go` の中身を実装する。

| ファイル | 状態 |
|---|---|
| `server.go` | 足場。ルーティングとミドルウェアの合成順 |
| `handlers.go` | 足場。業務ハンドラとストア |
| `reqid.go` | 足場。リクエストIDの ctx への出し入れ |
| `observer.go` | 足場。`ResponseObserver` インターフェースの定義 |
| `middleware.go` | **演習対象**。5つとも `panic("not implemented")` |

## 要件

### Chain

1. `Chain(h, a, b, c)` は `a` が最も外側、`c` が最も内側になる
2. ミドルウェアが1つもなければ `h` をそのまま返す

### RequestID

3. `X-Request-Id` ヘッダが付いていれば、その値を引き継ぐ
4. 付いていなければ `NewRequestID()` で発行する
5. ハンドラから `RequestIDFrom(r.Context())` で取れること
6. レスポンスヘッダ `X-Request-Id` にも同じ値を入れる

### ResponseObserver (`NewResponseObserver`)

7. `Status()` は書き込まれたステータス。まだ何も書かれていなければ 0
8. `WriteHeader` を呼ばずに `Write` された場合、`Status()` は 200
9. `WriteHeader` が2回以上呼ばれたら、2回目以降は**下位の ResponseWriter にも渡さない**
10. `BytesWritten()` は本文として書かれたバイト数の累計
11. `Written()` はヘッダか本文が書かれたかどうか
12. 包んだあとも `http.ResponseController` 経由の `Flush()` が下位に届くこと

### AccessLog

13. リクエスト1本につき1行。レベルは INFO、メッセージは `"request"`
14. 属性は `method` / `path` / `status` / `bytes` / `request_id` / `duration_ms`
15. ハンドラが `WriteHeader` を呼ばなくても `status` は 200

### Recover

16. panic を受け止め、500 と `{"code":"internal_error"}` を返す。
    panic の内容をクライアントに漏らさないこと
17. レベル ERROR、メッセージ `"panic"`、属性は `panic` / `stack` / `request_id`
18. **すでに本文が書かれていたら、レスポンスに手を加えない**（記録だけする）
19. `http.ErrAbortHandler` の panic は握り潰さず、そのまま投げ直す

### 配置

20. `server.go` の並び順のまま、panic したリクエストもアクセスログに `status=500` で残り、
    アクセスログと panic ログに**同じ `request_id`** が入ること

## 実行

```bash
mise x -- go test ./03-http-middleware/            # drill/ 直下から
mise x -- go test -race ./03-http-middleware/
mise x -- go test -run TestChain -v ./03-http-middleware/
```

テストは17本。`Chain` → `RequestID` → `ResponseObserver` → `AccessLog` → `Recover` の順に
並んでいるので、上から潰していくのが進めやすい。`ResponseObserver` が他の2つの土台になる。

## 進め方

- **`server.go` と `handlers.go` を先に読む**。ハンドラが何を書き、何を書いていないかが要件の背景
- 詰まったら、まず「どのテストの何が満たせていないか」を言葉にしてみる
- Claude に聞くときは、いきなり答えを求めず「概念の名前だけ教えて」から始める
- 終わったら `NOTES.md` に詰まった箇所と、後から気づいたことを書く

## この課題で踏む罠

実装が終わったあとに答え合わせをするためのリスト。先に読まないこと。

<details>
<summary>ネタバレ（実装後に開く）</summary>

- `Chain` の適用順を逆にして、最後の引数が最外側になる
- `RequestID` を ctx に載せたが、`r` を差し替え忘れて下流に届かない
- レスポンスヘッダへの書き込みを、ハンドラが本文を書いた**後**にやろうとして効かない
- `ResponseWriter` を包んだせいで `http.Flusher` が隠れ、SSE が流れなくなる
- `WriteHeader` の2回目を下位に渡してしまい、`superfluous WriteHeader call` が出る
- ハンドラが `WriteHeader` を呼ばないケースを忘れ、ステータスが 0 のままログに出る
- `Write` の戻り値のバイト数ではなく、引数の長さを足してしまう
- panic の値をそのままレスポンス本文に書いて、内部情報を外に出す
- 本文を書き終えた後の panic で 500 を書きに行き、壊れた本文が返る
- `http.ErrAbortHandler` を握り潰して、切断済みの接続にログの山を作る
- panic の処理をアクセスログより外側に置いてしまい、落ちたリクエストがログに残らない
- ミドルウェアの中でハンドラ実行前にログを出し、ステータスが決まる前に書いてしまう

</details>

## 補足: 実務では

**タイムアウト**もミドルウェアの定番だが、この課題には入れていない。
標準に `http.TimeoutHandler` があり、自作すると「ハンドラが書いている最中に
タイムアウトが来たらどうするか」を正しく扱うのが難しい（標準実装は本文をいったん
バッファに溜めて解決している）。実務でも自作せず標準を使う。

**ミドルウェアの合成**は `Chain` のような自前ヘルパでも書けるが、
`chi` などのルータは同じものを持っている。この課題の `ResponseObserver` に相当するものも
`chi/middleware.WrapResponseWriter` として用意されている。自分で書いておくと、
それらが何を肩代わりしているかが分かる。

**`http.ResponseController`** (Go 1.20 以降) は、`ResponseWriter` を何段包んでも
`Flush` や `SetWriteDeadline` を下まで届けるための仕組み。それ以前は
`w.(http.Flusher)` の型アサーションが包むたびに壊れるのが定番の事故だった。

**ログの出力先**は本番では標準出力で、収集基盤 (CloudWatch Logs、Loki など) が拾う。
`slog` の JSON ハンドラを使うと構造化されたまま検索できる。この課題でログを
「1行の文字列」ではなく属性の集まりとして出しているのはそのため。
`request_id` はサービスをまたいで引き継がれ、分散トレーシングの相関IDになる。
