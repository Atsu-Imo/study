// net-02: 簡易HTTPサーバー
// ネットワークの「登場人物」第2弾: Webサーバー（Apache/nginx 的な存在）
//
// HTTPは「TCPの上に乗ったテキストプロトコル」にすぎない。
// このプロジェクトでは TcpListener から手書きで HTTP リクエストをパースし、
// レスポンスを組み立てて返す。

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

// =============================================================================
// HTTP リクエスト / レスポンス の型
// =============================================================================

/// HTTPリクエスト
/// 例:
///   GET /hello HTTP/1.1
///   Host: 127.0.0.1:7878
///   User-Agent: curl/8.0
///   <空行>
#[derive(Debug)]
struct HttpRequest {
    method: String,                 // "GET", "POST" など
    path: String,                   // "/hello", "/echo?msg=hi" など
    version: String,                // "HTTP/1.1"
    headers: Vec<(String, String)>, // ("Host", "127.0.0.1:7878") など
    body: Vec<u8>,                  // ボディはバイト列で保持（例: POSTのフォームデータ）
}

/// HTTPレスポンス
/// 例:
///   HTTP/1.1 200 OK
///   Content-Type: text/plain; charset=utf-8
///   Content-Length: 13
///   <空行>
///   Hello, world!
struct HttpResponse {
    status: u16,                // 200, 404 など
    reason: &'static str,       // "OK", "Not Found"
    content_type: &'static str, // "text/plain; charset=utf-8"
    body: String,
}

impl HttpResponse {
    fn ok(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            reason: "OK",
            content_type: "text/plain; charset=utf-8",
            body: body.into(),
        }
    }

    fn html(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            reason: "OK",
            content_type: "text/html; charset=utf-8",
            body: body.into(),
        }
    }

    fn not_found() -> Self {
        Self {
            status: 404,
            reason: "Not Found",
            content_type: "text/plain; charset=utf-8",
            body: "404 Not Found\n".to_string(),
        }
    }

    /// レスポンスをHTTP/1.1の生バイト列にシリアライズする
    fn to_bytes(&self) -> Vec<u8> {
        // ステータス行 + ヘッダー + 空行 + ボディ という構造を組み立てる
        let body_bytes = self.body.as_bytes();
        let header = format!(
            "HTTP/1.1 {status} {reason}\r\n\
             Content-Type: {ctype}\r\n\
             Content-Length: {len}\r\n\
             Connection: close\r\n\
             \r\n",
            status = self.status,
            reason = self.reason,
            ctype = self.content_type,
            len = body_bytes.len(),
        );
        let mut out = header.into_bytes();
        out.extend_from_slice(body_bytes);
        out
    }
}

// =============================================================================
// リクエストのパース
// =============================================================================

/// TCPストリームから1つのHTTPリクエストを読み取る
///
/// 戻り値:
///   Ok(Some(req))  → リクエストを正常に読み取れた
///   Ok(None)       → クライアントが何も送らずに切断した（空リクエスト）
///   Err(...)       → I/Oエラーまたは不正なリクエスト行
fn parse_request<R: BufRead>(reader: &mut R) -> std::io::Result<Option<HttpRequest>> {
    // 1行目: リクエスト行 (例: "GET /hello HTTP/1.1")
    let mut request_line = String::new();
    let n = reader.read_line(&mut request_line)?;
    if n == 0 {
        // EOF: 接続が即座に閉じられた
        return Ok(None);
    }
    let trimmed = request_line.trim_end_matches(['\r', '\n']);
    let mut parts = trimmed.splitn(3, ' ');
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    let version = parts.next().unwrap_or("").to_string();

    if method.is_empty() || path.is_empty() || version.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("不正なリクエスト行: {trimmed:?}"),
        ));
    }

    // 2行目以降: ヘッダー（空行が来るまで）
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            // ヘッダー途中で接続が切れた
            break;
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            // 空行 = ヘッダー終了
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    let content_length = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Content-Length"))
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);

    let mut body = Vec::new();
    if content_length > 0 {
        // ボディがある場合は、Content-Length分だけ追加で読む
        body = vec![0u8; content_length];
        reader.read_exact(&mut body)?;
        // ここではまだ HttpRequest に body フィールドがないので、読み捨てる形にしている
    }

    Ok(Some(HttpRequest {
        method,
        path,
        version,
        headers,
        body,
    }))
}

// =============================================================================
// ルーティング（リクエスト → レスポンス）
// =============================================================================

/// パスに応じてレスポンスを返す
fn route(req: &HttpRequest, addr: std::net::SocketAddr) -> HttpResponse {
    // クエリ文字列を取り除いた純粋なパス部分
    let (path, query) = req.path.split_once('?').unwrap_or((req.path.as_str(), ""));

    match (req.method.as_str(), path) {
        ("GET", "/") => HttpResponse::html(
            "<!doctype html>\
             <html><head><meta charset=\"utf-8\"><title>net-02</title></head>\
             <body><h1>Hello from net-02!</h1>\
             <p>これは Rust で手書きした HTTP サーバーです。</p>\
             <ul>\
               <li><a href=\"/hello\">/hello</a></li>\
               <li><a href=\"/headers\">/headers</a> — 受信したヘッダーを表示</li>\
             </ul></body></html>",
        ),
        ("GET", "/hello") => HttpResponse::ok("Hello, world!\n"),
        ("GET", "/headers") => {
            // 受信したヘッダーをそのまま表示するデバッグ用エンドポイント
            let mut body = String::from("受信したヘッダー:\n");
            for (k, v) in &req.headers {
                body.push_str(&format!("  {k}: {v}\n"));
            }
            HttpResponse::ok(body)
        }
        ("GET", "/time") => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("時刻エラー");
            HttpResponse::ok(format!("UNIX time: {}秒\n", now.as_secs()))
        }
        ("GET", "/ip") => {
            // クライアントのIPアドレスを返す
            HttpResponse::ok(format!("クライアントのIPアドレス: {}\n", addr.ip()))
        }
        ("GET", "/echo") => {
            let queries = query
                .split('&')
                .filter_map(|pair| pair.split_once('='))
                .collect::<Vec<_>>();
            if let Some((_, msg)) = queries.iter().find(|(k, _)| *k == "msg") {
                HttpResponse::ok(format!("echo: {msg}\n"))
            } else {
                HttpResponse {
                    status: 400,
                    reason: "Bad Request",
                    content_type: "text/plain; charset=utf-8",
                    body: "400 Bad Request: msg パラメータがありません\n".to_string(),
                }
            }
        }
        ("GET", "/form") => HttpResponse::html(
            "<!doctype html>\
                 <html><head><meta charset=\"utf-8\"><title>Form</title></head>\
                 <body><h1>フォーム</h1>\
                 <form method=\"POST\" action=\"/submit\">\
                   <label>名前: <input type=\"text\" name=\"name\"></label><br>\
                   <label>年齢: <input type=\"number\" name=\"age\"></label><br>\
                   <button type=\"submit\">送信</button>\
                 </form></body></html>",
        ),
        ("POST", "/submit") => {
            let body_str = String::from_utf8_lossy(&req.body);
            let pairs = body_str
                .split('&')
                .filter_map(|pair| pair.split_once('='))
                .collect::<Vec<_>>();
            let mut response_body = String::from("受け取ったフォームデータ:\n");
            for (k, v) in pairs {
                response_body.push_str(&format!("  {k}: {v}\n"));
            }
            HttpResponse::ok(response_body)
        }
        _ => HttpResponse::not_found(),
    }
}

// =============================================================================
// サーバー本体
// =============================================================================

fn handle_client(stream: TcpStream) {
    let addr = stream.peer_addr().expect("接続元アドレスの取得に失敗");
    println!("[サーバー] 接続: {addr}");

    let mut reader = BufReader::new(&stream);
    let mut writer = stream.try_clone().expect("ストリームのクローンに失敗");

    match parse_request(&mut reader) {
        Ok(Some(req)) => {
            println!(
                "[サーバー] {addr} {} {} {}",
                req.method, req.path, req.version
            );
            let resp = route(&req, addr);
            println!("[サーバー] → {} {}", resp.status, resp.reason);
            if let Err(e) = writer.write_all(&resp.to_bytes()) {
                eprintln!("[サーバー] 応答書き込み失敗: {e}");
            }
        }
        Ok(None) => {
            println!("[サーバー] {addr} は何も送らずに切断");
        }
        Err(e) => {
            eprintln!("[サーバー] {addr} リクエスト解析エラー: {e}");
            // 400 Bad Request を返す（best effort、失敗しても無視）
            let resp = HttpResponse {
                status: 400,
                reason: "Bad Request",
                content_type: "text/plain; charset=utf-8",
                body: format!("400 Bad Request: {e}\n"),
            };
            writer.write_all(&resp.to_bytes()).ok();
        }
    }
    // Connection: close を返しているので、ここで stream が drop されて切断する
    println!("[サーバー] 切断: {addr}");
}

fn run_server(addr: &str) {
    let listener = TcpListener::bind(addr).expect("バインドに失敗");
    println!("[サーバー] {addr} でリッスン開始");
    println!("[サーバー] ブラウザで http://{addr}/ にアクセスしてみてください");
    println!("[サーバー] Ctrl+C で停止");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| handle_client(stream));
            }
            Err(e) => eprintln!("[サーバー] 接続受付エラー: {e}"),
        }
    }
}

// =============================================================================
// テスト用クライアント (curl の代わり)
// =============================================================================

/// サーバーに GET リクエストを送ってレスポンスを表示する
fn run_client(addr: &str, path: &str) {
    let mut stream = TcpStream::connect(addr).expect("接続失敗");
    println!("[クライアント] {addr} に接続");

    // HTTPリクエストを組み立てて送信
    // リクエスト行 + Host ヘッダー + 空行 という最小構成
    let request = format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {addr}\r\n\
         User-Agent: net-02-client/0.1\r\n\
         Connection: close\r\n\
         \r\n",
    );
    println!("[クライアント] === 送信リクエスト ===");
    print!("{request}");
    println!("=======================");

    stream.write_all(request.as_bytes()).expect("送信失敗");

    // レスポンスを全部読む
    let mut buf = String::new();
    std::io::Read::read_to_string(&mut stream, &mut buf).expect("受信失敗");
    println!("[クライアント] === 受信レスポンス ===");
    print!("{buf}");
    println!("=======================");
}

// =============================================================================
// メイン
// =============================================================================

fn main() {
    let addr = "127.0.0.1:7878";

    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("demo");

    match mode {
        "server" => {
            // サーバー専用モード: cargo run -- server
            run_server(addr);
        }
        "client" => {
            // クライアントモード: cargo run -- client [path]
            let path = args.get(2).map(String::as_str).unwrap_or("/");
            run_client(addr, path);
        }
        "demo" => {
            // デモモード: サーバーを別スレッドで起動し、複数のパスをクライアントで叩く
            println!("=== net-02 簡易HTTPサーバー デモ ===\n");
            println!("仕組み:");
            println!("  1. {addr} でHTTPサーバーを起動");
            println!("  2. 自前クライアントで GET / と GET /hello を叩く");
            println!("  3. ブラウザでも http://{addr}/ にアクセス可能\n");

            let server_addr = addr.to_string();
            thread::spawn(move || run_server(&server_addr));
            thread::sleep(std::time::Duration::from_millis(100));

            for path in ["/", "/hello", "/headers", "/nonexistent"] {
                println!("\n--- GET {path} ---");
                run_client(addr, path);
                thread::sleep(std::time::Duration::from_millis(50));
            }

            println!("\n=== デモ終了 ===");
            println!("別ターミナルで試すには:");
            println!("  cargo run -- server      # サーバー起動");
            println!("  cargo run -- client /    # 自前クライアントで叩く");
            println!("  curl http://{addr}/      # curl でも当然OK");
        }
        other => {
            eprintln!("不明なモード: {other}");
            eprintln!("使い方: cargo run -- [server|client [path]|demo]");
        }
    }
}

// =============================================================================
// 演習
// =============================================================================

// --- 演習1: 基礎 ---
// ルーティングに新しいパスを追加してみよう。
//   GET /time → 現在時刻を返す (例: "2026-05-22 12:34:56")
//   GET /ip   → クライアントのIPアドレスを返す (peer_addr() を使う)
//
// ヒント:
//   - 時刻フォーマット用クレートは使わず、std::time::SystemTime と UNIX_EPOCH で
//     秒数を出すだけでもOK
//   - /ip は handle_client で peer_addr を取れるので、route 関数に渡す形に拡張する
//
// route 関数に追加するだけで動く:
//
// ("GET", "/time") => {
//     let now = std::time::SystemTime::now()
//         .duration_since(std::time::UNIX_EPOCH)
//         .expect("時刻エラー");
//     HttpResponse::ok(format!("UNIX time: {}秒\n", now.as_secs()))
// }

// --- 演習2: 応用 ---
// クエリパラメータをパースする /echo エンドポイントを作ろう。
//   GET /echo?msg=hello → "hello\n" を返す
//   GET /echo?msg=こんにちは → "こんにちは\n" を返す（URLエンコードはここでは無視してOK）
//   GET /echo (msg なし) → "msg パラメータがありません\n" を 400 で返す
//
// ヒント:
//   - req.path には "/echo?msg=hello" のように ? 以降も含まれている
//   - split_once('?') でクエリ部分を取り出せる
//   - クエリは "key1=value1&key2=value2" 形式。簡単のため key=value 1つだけサポートでOK
//
// fn parse_query(query: &str) -> Vec<(String, String)> {
//     query.split('&')
//         .filter_map(|pair| pair.split_once('='))
//         .map(|(k, v)| (k.to_string(), v.to_string()))
//         .collect()
// }

// --- 演習3: チャレンジ ---
// POSTリクエストを受け付けて、フォーム送信内容を表示するエンドポイントを作ろう。
//   GET  /form → HTMLフォーム (<form method="POST" action="/submit">) を返す
//   POST /submit → リクエストボディ (例: "name=alice&age=30") をパースして表示
//
// 難しいポイント:
//   - POSTにはボディがある → Content-Length ヘッダー分のバイトを追加で読む必要がある
//   - parse_request はヘッダーまでしか読まないので、ボディ読み取りロジックを足す
//
// 拡張のヒント:
//
// struct HttpRequest {
//     // ...
//     body: Vec<u8>,
// }
//
// fn parse_request<R: BufRead>(reader: &mut R) -> std::io::Result<Option<HttpRequest>> {
//     // ヘッダーまで読んだ後...
//     let content_length: usize = headers.iter()
//         .find(|(k, _)| k.eq_ignore_ascii_case("Content-Length"))
//         .and_then(|(_, v)| v.parse().ok())
//         .unwrap_or(0);
//     let mut body = vec![0u8; content_length];
//     reader.read_exact(&mut body)?;
//     // ...
// }
