// net-04: TLS ハンドシェイク
// ネットワークの「登場人物」第4弾: TLS (HTTPS の "S")
//
// net-02 で作った HTTP は「平文のテキスト」だった。誰でも盗み見・改ざんできる。
// TLS は HTTP の下に潜り込んで通信を暗号化する層。ブラウザの「鍵マーク」の正体。
//
// このプロジェクトでは本物の暗号ライブラリ (rustls 等) を使わず、
// **ハンドシェイクの「流れ」と「役者の受け渡し」を std だけで自作**して、
//   ClientHello → ServerHello → 証明書 → 鍵交換 → 暗号化通信
// の一連を TCP 上で実際に動かす。数学は小さな toy 実装で雰囲気を掴む。
//
// ⚠️ 注意: ここで使う鍵長・乱数・暗号はすべて教育用の「おもちゃ」。
//          本番では絶対に使わないこと (本番は鍵交換は数千ビット、暗号は AES-GCM 等)。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// =============================================================================
// 定数
// =============================================================================

const PORT: u16 = 8443; // HTTPS の 443 にちなんだ非特権ポート

// --- Diffie-Hellman 鍵交換のパラメータ (公開情報) ---
// p は素数 (2^61 - 1 メルセンヌ素数)、g は生成元。本物は 2048bit 以上。
const DH_P: u64 = 2_305_843_009_213_693_951; // 2^61 - 1
const DH_G: u64 = 5;

// サーバーの「長期」DH 秘密鍵 (本来サーバーだけが知る)。
// 本物はセッションごとに使い捨て (ephemeral) だが、ここでは CA が事前に署名できるよう固定。
const SERVER_DH_PRIVATE: u64 = 9_876_543_210_987;

// --- CA (認証局) の toy RSA 鍵 ---
// 教科書サイズ: p=61, q=53, n=3233, φ=3120, e=17, d=2753
// 公開鍵 (e, n) は世界中が知っている (ブラウザに同梱)。秘密鍵 d は CA だけが持つ。
const RSA_N: u64 = 3233;
const RSA_E: u64 = 17; // 公開指数 (検証に使う)
const RSA_D: u64 = 2753; // 秘密指数 (署名に使う) — CA だけが知る想定

const SERVER_NAME: &str = "toytls.local";

// --- ハンドシェイクのメッセージ種別 (TLS の HandshakeType を模したもの) ---
const MSG_CLIENT_HELLO: u8 = 1;
const MSG_SERVER_HELLO: u8 = 2;
const MSG_CERTIFICATE: u8 = 3;
const MSG_SERVER_HELLO_DONE: u8 = 4;
const MSG_CLIENT_KEY_EXCHANGE: u8 = 5;
const MSG_FINISHED: u8 = 6;
const MSG_APP_DATA: u8 = 7;

// =============================================================================
// 数学プリミティブ
// =============================================================================

/// 冪剰余: base^exp mod modulus を高速に計算する (バイナリ法)。
/// 公開鍵暗号・DH 鍵交換の心臓部。中間計算は u128 で溢れを防ぐ。
fn mod_pow(base: u64, exp: u64, modulus: u64) -> u64 {
    if modulus == 1 {
        return 0;
    }
    let m = modulus as u128;
    let mut result: u128 = 1;
    let mut b = (base as u128) % m;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = result * b % m;
        }
        e >>= 1;
        b = b * b % m;
    }
    result as u64
}

/// toy ハッシュ関数 (FNV 風)。結果を modulus 未満に収める。
/// 本物は SHA-256 等。ここでは「データを固定長の数値に潰す」役割だけ示す。
fn toy_hash(data: &[u8], modulus: u64) -> u64 {
    let mut h: u64 = 0;
    for &byte in data {
        h = (h.wrapping_mul(31).wrapping_add(byte as u64)) % modulus;
    }
    h
}

// =============================================================================
// 乱数 (toy)
// =============================================================================

/// SystemTime をタネにした簡易乱数の状態を作る。
/// ⚠️ 暗号用途には絶対不適 (予測可能)。本物は OS の CSPRNG を使う。
fn rng_seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x1234_5678_9abc_def0);
    nanos | 1 // 0 を避ける
}

/// 線形合同法で次の乱数を返す。
fn next_rand(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

// =============================================================================
// 証明書 (toy): CA がサーバーの「名前 + DH 公開鍵」に署名したもの
// =============================================================================

/// 証明書の中身 (名前 + サーバー DH 公開鍵) を1つのハッシュ値に潰す。
/// CA はこのハッシュに署名し、クライアントは同じハッシュを再計算して検証する。
fn cert_digest(name: &str, dh_public: u64) -> u64 {
    let mut data = Vec::new();
    data.extend_from_slice(name.as_bytes());
    data.extend_from_slice(&dh_public.to_be_bytes());
    toy_hash(&data, RSA_N)
}

/// CA による署名: digest^d mod n (CA の秘密鍵で「暗号化」する操作)。
fn ca_sign(name: &str, dh_public: u64) -> u64 {
    let digest = cert_digest(name, dh_public);
    mod_pow(digest, RSA_D, RSA_N)
}

/// クライアントによる検証: sig^e mod n を計算し、自分で再計算した digest と一致するか見る。
/// 一致すれば「この証明書は確かに CA (秘密鍵 d の持ち主) が署名した」と分かる。
fn verify_cert(name: &str, dh_public: u64, signature: u64) -> bool {
    let recovered = mod_pow(signature, RSA_E, RSA_N);
    recovered == cert_digest(name, dh_public)
}

// =============================================================================
// 共通鍵暗号 (toy ストリーム暗号)
// =============================================================================

/// セッション鍵をタネにキーストリームを生成し、平文と XOR する。
/// XOR 暗号なので「暗号化」と「復号」は同じ関数。本物は AES-GCM 等。
fn xor_cipher(data: &[u8], session_key: u64) -> Vec<u8> {
    let mut state = session_key | 1;
    data.iter()
        .map(|&byte| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let keystream = (state >> 33) as u8;
            byte ^ keystream
        })
        .collect()
}

/// 共有秘密 + 両者の random から実際に使うセッション鍵を導出する。
/// (本物の TLS でいう鍵導出関数 KDF/PRF に相当)。
/// client_random / server_random を混ぜることで、共有秘密が同じでも毎回違う鍵になる。
fn derive_session_key(shared_secret: u64, client_random: u64, server_random: u64) -> u64 {
    let mut mixed = shared_secret
        ^ client_random.rotate_left(17)
        ^ server_random.rotate_left(31);
    mixed |= 1;
    for _ in 0..3 {
        mixed = mixed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
    }
    mixed
}

// =============================================================================
// メッセージのフレーミング (TLS レコード層のミニチュア)
// =============================================================================
// 形式: [種別 1バイト][長さ 4バイト BE][ペイロード]

fn write_msg(stream: &mut TcpStream, msg_type: u8, payload: &[u8]) -> std::io::Result<()> {
    stream.write_all(&[msg_type])?;
    stream.write_all(&(payload.len() as u32).to_be_bytes())?;
    stream.write_all(payload)?;
    stream.flush()
}

fn read_msg(stream: &mut TcpStream) -> std::io::Result<(u8, Vec<u8>)> {
    let mut header = [0u8; 5];
    stream.read_exact(&mut header)?;
    let msg_type = header[0];
    let len = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload)?;
    Ok((msg_type, payload))
}

/// ペイロード先頭 8 バイトを u64 (big-endian) として読む。
fn read_u64(bytes: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[0..8]);
    u64::from_be_bytes(buf)
}

// =============================================================================
// サーバー側のハンドシェイク
// =============================================================================

fn handle_server_conn(mut stream: TcpStream) -> std::io::Result<()> {
    // サーバーの DH 公開鍵 = g^(秘密鍵) mod p
    let server_dh_public = mod_pow(DH_G, SERVER_DH_PRIVATE, DH_P);

    // --- 1. ClientHello を受信 ---
    let (t, payload) = read_msg(&mut stream)?;
    assert_eq!(t, MSG_CLIENT_HELLO, "最初は ClientHello のはず");
    let client_random = read_u64(&payload);
    let ciphers = String::from_utf8_lossy(&payload[8..]);
    println!("[サーバー] ← ClientHello (client_random={client_random:#x}, 提案cipher={ciphers})");

    // --- 2. ServerHello を送信 ---
    let mut seed = rng_seed();
    let server_random = next_rand(&mut seed);
    let mut hello = Vec::new();
    hello.extend_from_slice(&server_random.to_be_bytes());
    hello.extend_from_slice(b"TOY-DHE-RSA-XOR");
    write_msg(&mut stream, MSG_SERVER_HELLO, &hello)?;
    println!("[サーバー] → ServerHello (server_random={server_random:#x}, 採用cipher=TOY-DHE-RSA-XOR)");

    // --- 3. Certificate を送信 (名前 + DH 公開鍵 + CA 署名) ---
    let signature = ca_sign(SERVER_NAME, server_dh_public);
    let mut cert = Vec::new();
    cert.extend_from_slice(&server_dh_public.to_be_bytes());
    cert.extend_from_slice(&signature.to_be_bytes());
    cert.extend_from_slice(SERVER_NAME.as_bytes());
    write_msg(&mut stream, MSG_CERTIFICATE, &cert)?;
    println!("[サーバー] → Certificate (name={SERVER_NAME}, dh_pub={server_dh_public}, CA署名={signature})");

    // --- 4. ServerHelloDone ---
    write_msg(&mut stream, MSG_SERVER_HELLO_DONE, &[])?;
    println!("[サーバー] → ServerHelloDone (送るものは送った)");

    // --- 5. ClientKeyExchange を受信 ---
    let (t, payload) = read_msg(&mut stream)?;
    assert_eq!(t, MSG_CLIENT_KEY_EXCHANGE);
    let client_dh_public = read_u64(&payload);
    println!("[サーバー] ← ClientKeyExchange (client_dh_pub={client_dh_public})");

    // --- 6. 共有秘密とセッション鍵を計算 ---
    let shared_secret = mod_pow(client_dh_public, SERVER_DH_PRIVATE, DH_P);
    let session_key = derive_session_key(shared_secret, client_random, server_random);
    println!("[サーバー] 🔑 共有秘密={shared_secret} → セッション鍵={session_key:#x} を導出");

    // --- 7. Finished の交換 (ここから暗号化) ---
    let (t, enc) = read_msg(&mut stream)?;
    assert_eq!(t, MSG_FINISHED);
    let dec = xor_cipher(&enc, session_key);
    println!(
        "[サーバー] ← Finished (暗号文 {} バイト) → 復号: {:?}",
        enc.len(),
        String::from_utf8_lossy(&dec)
    );
    let my_finished = xor_cipher(b"Finished: server ready", session_key);
    write_msg(&mut stream, MSG_FINISHED, &my_finished)?;
    println!("[サーバー] → Finished (暗号化して送信)");

    // --- 8. アプリケーションデータ (暗号化された HTTP) ---
    let (t, enc) = read_msg(&mut stream)?;
    assert_eq!(t, MSG_APP_DATA);
    let request = xor_cipher(&enc, session_key);
    println!(
        "[サーバー] ← 暗号化リクエスト {} バイト → 復号:\n----\n{}----",
        enc.len(),
        String::from_utf8_lossy(&request)
    );

    let body = "<html><body><h1>Hello over toy-TLS!</h1></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let enc_response = xor_cipher(response.as_bytes(), session_key);
    write_msg(&mut stream, MSG_APP_DATA, &enc_response)?;
    println!("[サーバー] → 暗号化レスポンス {} バイトを送信", enc_response.len());

    Ok(())
}

fn run_server(port: u16) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    println!("[サーバー] 127.0.0.1:{port} で待ち受け開始 (toy-TLS)");
    println!("[サーバー] CA 公開鍵 (e={RSA_E}, n={RSA_N}) はクライアントに配布済みの想定");
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                if let Err(e) = handle_server_conn(s) {
                    eprintln!("[サーバー] 接続処理エラー: {e}");
                }
            }
            Err(e) => eprintln!("[サーバー] accept エラー: {e}"),
        }
    }
    Ok(())
}

// =============================================================================
// クライアント側のハンドシェイク
// =============================================================================

fn run_client(addr: &str) -> std::io::Result<()> {
    let mut stream = TcpStream::connect(addr)?;
    println!("[クライアント] {addr} に TCP 接続完了。toy-TLS ハンドシェイク開始");

    let mut seed = rng_seed();
    let client_random = next_rand(&mut seed);
    // クライアントの DH 秘密鍵はセッションごとに使い捨て (ephemeral)
    let client_dh_private = (next_rand(&mut seed) % (DH_P - 2)) + 1;
    let client_dh_public = mod_pow(DH_G, client_dh_private, DH_P);

    // --- 1. ClientHello ---
    let mut hello = Vec::new();
    hello.extend_from_slice(&client_random.to_be_bytes());
    hello.extend_from_slice(b"TOY-DHE-RSA-XOR,TOY-NULL");
    write_msg(&mut stream, MSG_CLIENT_HELLO, &hello)?;
    println!("[クライアント] → ClientHello (client_random={client_random:#x})");

    // --- 2. ServerHello ---
    let (t, payload) = read_msg(&mut stream)?;
    assert_eq!(t, MSG_SERVER_HELLO);
    let server_random = read_u64(&payload);
    println!(
        "[クライアント] ← ServerHello (server_random={server_random:#x}, cipher={})",
        String::from_utf8_lossy(&payload[8..])
    );

    // --- 3. Certificate を受信して検証 ---
    let (t, cert) = read_msg(&mut stream)?;
    assert_eq!(t, MSG_CERTIFICATE);
    let server_dh_public = read_u64(&cert[0..8]);
    let signature = read_u64(&cert[8..16]);
    let name = String::from_utf8_lossy(&cert[16..]).into_owned();
    println!("[クライアント] ← Certificate (name={name}, dh_pub={server_dh_public}, 署名={signature})");

    if verify_cert(&name, server_dh_public, signature) {
        println!("[クライアント] ✅ 証明書検証 OK: CA の署名が正しい → サーバーを信頼する");
    } else {
        eprintln!("[クライアント] ❌ 証明書検証 失敗: 中間者攻撃の可能性。接続中止");
        return Ok(());
    }
    if name != SERVER_NAME {
        eprintln!("[クライアント] ❌ 証明書の名前 ({name}) がアクセス先と不一致。接続中止");
        return Ok(());
    }

    // --- 4. ServerHelloDone ---
    let (t, _) = read_msg(&mut stream)?;
    assert_eq!(t, MSG_SERVER_HELLO_DONE);
    println!("[クライアント] ← ServerHelloDone");

    // --- 5. ClientKeyExchange ---
    write_msg(
        &mut stream,
        MSG_CLIENT_KEY_EXCHANGE,
        &client_dh_public.to_be_bytes(),
    )?;
    println!("[クライアント] → ClientKeyExchange (client_dh_pub={client_dh_public})");

    // --- 6. 共有秘密とセッション鍵 ---
    let shared_secret = mod_pow(server_dh_public, client_dh_private, DH_P);
    let session_key = derive_session_key(shared_secret, client_random, server_random);
    println!("[クライアント] 🔑 共有秘密={shared_secret} → セッション鍵={session_key:#x} を導出");
    println!("[クライアント]    (盗聴者は p, g, 双方の公開鍵を見ても秘密鍵なしには共有秘密を作れない)");

    // --- 7. Finished の交換 ---
    let my_finished = xor_cipher(b"Finished: client ready", session_key);
    write_msg(&mut stream, MSG_FINISHED, &my_finished)?;
    println!("[クライアント] → Finished (暗号化して送信)");
    let (t, enc) = read_msg(&mut stream)?;
    assert_eq!(t, MSG_FINISHED);
    let dec = xor_cipher(&enc, session_key);
    println!(
        "[クライアント] ← Finished → 復号: {:?}",
        String::from_utf8_lossy(&dec)
    );

    // --- 8. 暗号化された HTTP リクエスト/レスポンス ---
    let request = format!("GET / HTTP/1.1\r\nHost: {SERVER_NAME}\r\n\r\n");
    let enc_request = xor_cipher(request.as_bytes(), session_key);
    write_msg(&mut stream, MSG_APP_DATA, &enc_request)?;
    println!("[クライアント] → 暗号化 HTTP リクエストを送信 ({} バイト)", enc_request.len());

    let (t, enc) = read_msg(&mut stream)?;
    assert_eq!(t, MSG_APP_DATA);
    let response = xor_cipher(&enc, session_key);
    println!(
        "[クライアント] ← 暗号化レスポンス {} バイト → 復号:\n====\n{}====",
        enc.len(),
        String::from_utf8_lossy(&response)
    );

    Ok(())
}

// =============================================================================
// 盗聴デモ: 同じ鍵交換を「盗聴者」視点で見る
// =============================================================================

/// ネットワーク上を流れる公開情報 (p, g, 両者の公開鍵) だけから
/// 盗聴者が共有秘密を復元しようとしても、離散対数問題が壁になることを示す。
fn eavesdropper_demo() {
    println!("--- おまけ: 盗聴者の視点 ---");
    // 盗聴者が見える情報
    let server_pub = mod_pow(DH_G, SERVER_DH_PRIVATE, DH_P);
    let mut seed = rng_seed();
    let client_priv = (next_rand(&mut seed) % (DH_P - 2)) + 1;
    let client_pub = mod_pow(DH_G, client_priv, DH_P);

    println!("盗聴者が観測できるもの: p={DH_P}, g={DH_G}");
    println!("  サーバー公開鍵 = {server_pub}");
    println!("  クライアント公開鍵 = {client_pub}");
    println!("正規の共有秘密 = {}", mod_pow(server_pub, client_priv, DH_P));
    println!(
        "盗聴者は公開鍵から秘密鍵 (g^x mod p = 公開鍵 を満たす x) を逆算する必要があるが、"
    );
    println!("これは離散対数問題で、p が十分大きいと現実的な時間では解けない。");
}

// =============================================================================
// メイン
// =============================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("demo");

    match mode {
        "server" => {
            // 例: cargo run -- server
            if let Err(e) = run_server(PORT) {
                eprintln!("サーバー起動失敗: {e}");
            }
        }
        "client" => {
            // 例: cargo run -- client
            let addr = format!("127.0.0.1:{PORT}");
            if let Err(e) = run_client(&addr) {
                eprintln!("クライアントエラー: {e}");
            }
        }
        "demo" => {
            println!("=== net-04 toy-TLS ハンドシェイク デモ ===\n");
            println!("⚠️ 暗号はすべて教育用おもちゃ。本番では rustls / OpenSSL を使うこと。\n");

            // サーバーを別スレッドで起動
            thread::spawn(|| {
                let _ = run_server(PORT);
            });
            thread::sleep(Duration::from_millis(200));

            let addr = format!("127.0.0.1:{PORT}");
            if let Err(e) = run_client(&addr) {
                eprintln!("クライアントエラー: {e}");
            }

            // サーバー側のログが出きるのを少し待つ
            thread::sleep(Duration::from_millis(100));

            println!();
            eavesdropper_demo();

            println!("\n=== デモ終了 ===");
            println!("\n別ターミナルで試すには:");
            println!("  cargo run -- server   # 待ち受け");
            println!("  cargo run -- client   # 接続してハンドシェイク");
        }
        other => {
            eprintln!("不明なモード: {other}");
            eprintln!("使い方: cargo run -- [demo|server|client]");
        }
    }
}

// =============================================================================
// 演習
// =============================================================================

// --- 演習1: 基礎 ---
// 「証明書の改ざん検知」を体験しよう。
//
// handle_server_conn で Certificate を送る直前に、署名 (signature) を別の値
// (例: signature ^ 1) に書き換えてみる。
// するとクライアント側の verify_cert が false になり、「中間者攻撃の可能性」で
// 接続が中止されるはず。
// → CA 署名があるおかげで「証明書の改ざん」を検知できることを確認する。
//
// 余裕があれば server_dh_public を書き換えた場合も試す
// (こちらは digest が変わるので、やはり検証に失敗する)。

// --- 演習2: 応用 ---
// 「中間者 (MITM) プロキシ」を1つ書いてみよう。
//
//   クライアント  ──→  MITM  ──→  サーバー
//
// MITM はクライアントからの ClientHello を受けて自分でサーバー役を演じ、
// 同時にサーバーへクライアント役で接続する。
// このとき MITM は自分の DH 鍵で2本のセッションを張れてしまうが、
// 「サーバーの証明書 (CA 署名)」を持っていないため、正規 CA 公開鍵を持つ
// クライアントには Certificate 検証で必ずバレる。
// → なぜ TLS が「証明書 + CA」をセットで必要とするのかが腑に落ちる。
//
// ヒント: MITM は CA の秘密鍵 (RSA_D) を知らないので、有効な署名を作れない。

// --- 演習3: チャレンジ ---
// 「信頼の連鎖 (証明書チェーン)」を実装しよう。net-05 (CA) の予習。
//
// 現状は CA が直接サーバー証明書に署名している (ルート CA 直署名)。
// 実際は:
//   ルート CA  ──署名──→  中間 CA  ──署名──→  サーバー証明書
// という多段構造になっている。
//
//   1. ルート CA 鍵ペア (RSA) と中間 CA 鍵ペアを用意する
//   2. ルート CA が「中間 CA の公開鍵」に署名 (= 中間 CA 証明書)
//   3. 中間 CA が「サーバーの名前 + DH 公開鍵」に署名 (= サーバー証明書)
//   4. サーバーは [サーバー証明書, 中間 CA 証明書] の2枚を送る
//   5. クライアントは ルート CA 公開鍵だけを信頼の起点 (トラストアンカー) として持ち、
//      中間 → サーバー の順にチェーンを検証する
//
// 学べること: ブラウザが持つのは「ルート CA 証明書」だけで、
//             中間 CA は通信時にサーバーから受け取って検証する仕組み。
