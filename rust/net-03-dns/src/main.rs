// net-03: DNS リゾルバ & サーバー
// ネットワークの「登場人物」第3弾: DNS (ドメイン名解決の仕組み)
//
// HTTPと違って、DNSは **バイナリプロトコル** であり **UDP** を使う。
// このプロジェクトでは生バイト列を組み立て・パースして、
// 本物のDNSサーバー (8.8.8.8) と話す + 自前のDNSサーバーを実装する。

use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr, UdpSocket};
use std::time::Duration;

// =============================================================================
// 定数
// =============================================================================

const DEMO_SERVER_PORT: u16 = 5300; // 自前サーバー用 (53 は root 権限が必要なので別ポート)
const REAL_DNS_SERVER: &str = "8.8.8.8:53"; // Google Public DNS

// リソースレコード種別 (RFC 1035, 3596)
const TYPE_A: u16 = 1; // IPv4 アドレス
const TYPE_CNAME: u16 = 5; // 別名 (Canonical Name)
const TYPE_AAAA: u16 = 28; // IPv6 アドレス
const CLASS_IN: u16 = 1; // Internet

// =============================================================================
// ドメイン名のエンコード/デコード
// =============================================================================

/// ドメイン名 ("example.com") を DNS のラベル形式にエンコードする
///   "example.com" → [7]"example" [3]"com" [0]
fn encode_name(buf: &mut Vec<u8>, name: &str) {
    for label in name.split('.') {
        if label.is_empty() {
            continue;
        }
        // 各ラベルは「長さ1バイト + 中身」
        buf.push(label.len() as u8);
        buf.extend_from_slice(label.as_bytes());
    }
    buf.push(0); // ルートラベル (空ラベル) で終端
}

/// DNSメッセージ内の名前をパースする
///
/// 名前は2形式ある:
///   1. インライン:   [長さ][ラベル][長さ][ラベル]...[0]
///   2. 圧縮ポインタ: 0xC0xx (上位2ビットが 11) → メッセージ先頭からのオフセット
///
/// 戻り値: (名前, start から何バイト進めばよいか)
///        ポインタを追って読んだ場合でも consumed は元の位置からの2バイトだけ
fn parse_name(buf: &[u8], start: usize) -> (String, usize) {
    let mut labels = Vec::new();
    let mut pos = start;
    let mut jumped = false;
    let mut original_end = start;

    loop {
        if pos >= buf.len() {
            break;
        }
        let b = buf[pos];

        if b == 0 {
            // 終端
            pos += 1;
            if !jumped {
                original_end = pos;
            }
            break;
        }

        if b & 0xC0 == 0xC0 {
            // 圧縮ポインタ: 14bit のオフセット
            if pos + 1 >= buf.len() {
                break;
            }
            let offset = (((b & 0x3F) as usize) << 8) | (buf[pos + 1] as usize);
            if !jumped {
                original_end = pos + 2;
                jumped = true;
            }
            pos = offset;
            continue;
        }

        // 通常ラベル
        let len = b as usize;
        pos += 1;
        if pos + len > buf.len() {
            break;
        }
        let label = String::from_utf8_lossy(&buf[pos..pos + len]).into_owned();
        labels.push(label);
        pos += len;
    }

    (labels.join("."), original_end - start)
}

// =============================================================================
// DNSクエリの組み立て
// =============================================================================

/// 標準クエリ (再帰問い合わせ要求) を組み立てる
///
/// DNSメッセージの構造 (RFC 1035):
///   ┌──────────────┐ 12バイト
///   │   Header     │  ID, Flags, QDCOUNT, ANCOUNT, NSCOUNT, ARCOUNT
///   ├──────────────┤
///   │  Question    │  QNAME, QTYPE, QCLASS
///   ├──────────────┤
///   │   Answer     │  RR (リクエスト時は空)
///   ├──────────────┤
///   │  Authority   │
///   ├──────────────┤
///   │  Additional  │
///   └──────────────┘
fn build_query(id: u16, qname: &str, qtype: u16) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);

    // --- ヘッダー (12バイト) ---
    buf.extend_from_slice(&id.to_be_bytes()); // ID (応答との対応付け用)
    // フラグ: QR=0(query), Opcode=0, RD=1(再帰要求)
    // ビットレイアウト: 0 0000 0 0 1 0 000 0000  =  0x0100
    buf.extend_from_slice(&0x0100u16.to_be_bytes());
    buf.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT = 1
    buf.extend_from_slice(&0u16.to_be_bytes()); // ANCOUNT = 0
    buf.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT = 0
    buf.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT = 0

    // --- Question セクション ---
    encode_name(&mut buf, qname);
    buf.extend_from_slice(&qtype.to_be_bytes()); // QTYPE
    buf.extend_from_slice(&CLASS_IN.to_be_bytes()); // QCLASS = IN

    buf
}

// =============================================================================
// DNSレスポンスのパース
// =============================================================================

#[derive(Debug)]
struct DnsAnswer {
    name: String,
    rtype: u16,
    ttl: u32,
    data: String, // 表示用文字列に変換済み
}

fn type_name(t: u16) -> &'static str {
    match t {
        TYPE_A => "A",
        TYPE_CNAME => "CNAME",
        TYPE_AAAA => "AAAA",
        _ => "?",
    }
}

fn parse_response(buf: &[u8]) -> Result<Vec<DnsAnswer>, String> {
    if buf.len() < 12 {
        return Err("DNSヘッダーが12バイト未満".to_string());
    }

    // ヘッダー
    let flags = u16::from_be_bytes([buf[2], buf[3]]);
    let qdcount = u16::from_be_bytes([buf[4], buf[5]]);
    let ancount = u16::from_be_bytes([buf[6], buf[7]]);

    let rcode = flags & 0x000F;
    if rcode != 0 {
        let msg = match rcode {
            1 => "Format error",
            2 => "Server failure",
            3 => "NXDOMAIN (名前が存在しない)",
            4 => "Not implemented",
            5 => "Refused",
            _ => "Unknown",
        };
        return Err(format!("DNS RCODE={rcode} ({msg})"));
    }

    let mut pos = 12;

    // Question セクションをスキップ
    for _ in 0..qdcount {
        let (_, consumed) = parse_name(buf, pos);
        pos += consumed;
        pos += 4; // QTYPE + QCLASS
    }

    // Answer セクションを読む
    let mut answers = Vec::new();
    for _ in 0..ancount {
        let (name, consumed) = parse_name(buf, pos);
        pos += consumed;
        if pos + 10 > buf.len() {
            break;
        }

        let rtype = u16::from_be_bytes([buf[pos], buf[pos + 1]]);
        let ttl = u32::from_be_bytes([buf[pos + 4], buf[pos + 5], buf[pos + 6], buf[pos + 7]]);
        let rdlength = u16::from_be_bytes([buf[pos + 8], buf[pos + 9]]) as usize;
        pos += 10;

        if pos + rdlength > buf.len() {
            break;
        }
        let rdata = &buf[pos..pos + rdlength];

        // RDATA を種別ごとに解釈
        let data = match rtype {
            TYPE_A if rdlength == 4 => {
                Ipv4Addr::new(rdata[0], rdata[1], rdata[2], rdata[3]).to_string()
            }
            TYPE_AAAA if rdlength == 16 => {
                let mut octets = [0u8; 16];
                octets.copy_from_slice(rdata);
                Ipv6Addr::from(octets).to_string()
            }
            TYPE_CNAME => {
                // CNAME の RDATA はドメイン名 (圧縮ポインタを含む可能性あり)
                let (cname, _) = parse_name(buf, pos);
                cname
            }
            _ => format!("(type={rtype}, {rdlength} bytes)"),
        };

        answers.push(DnsAnswer {
            name,
            rtype,
            ttl,
            data,
        });
        pos += rdlength;
    }

    Ok(answers)
}

// =============================================================================
// リゾルバ
// =============================================================================

fn resolve(server: &str, qname: &str, qtype: u16) -> std::io::Result<Vec<DnsAnswer>> {
    // UDP ソケットを作って 0番ポートにバインド (= OSが自動割り当て)
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(3)))?;

    let id = (std::process::id() & 0xFFFF) as u16;
    let query = build_query(id, qname, qtype);

    socket.send_to(&query, server)?;

    // 応答受信 (DNSは通常512バイト以内、EDNS等で拡張あり)
    let mut buf = [0u8; 4096];
    let (n, _) = socket.recv_from(&mut buf)?;

    parse_response(&buf[..n]).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

fn run_resolver(server: &str, qname: &str, qtype: u16) {
    println!(
        "[リゾルバ] → {server} に問い合わせ: {qname} {}",
        type_name(qtype)
    );
    match resolve(server, qname, qtype) {
        Ok(answers) if answers.is_empty() => println!("[リゾルバ] 回答なし"),
        Ok(answers) => {
            for ans in answers {
                println!(
                    "[リゾルバ] ← {} {} TTL={} → {}",
                    ans.name,
                    type_name(ans.rtype),
                    ans.ttl,
                    ans.data
                );
            }
        }
        Err(e) => eprintln!("[リゾルバ] エラー: {e}"),
    }
}

// =============================================================================
// 簡易DNSサーバー (権威ネームサーバー風)
// =============================================================================

/// 受信したクエリをパースし、応答バイト列を組み立てる
fn handle_query(buf: &[u8], records: &HashMap<String, Vec<Ipv4Addr>>) -> Vec<u8> {
    let id = u16::from_be_bytes([buf[0], buf[1]]);
    let qdcount = u16::from_be_bytes([buf[4], buf[5]]);

    // Question セクションをパース (1個だけ想定)
    let mut pos = 12;
    let (qname, consumed) = parse_name(buf, pos);
    pos += consumed;
    let qtype = u16::from_be_bytes([buf[pos], buf[pos + 1]]);
    pos += 4; // QTYPE(2) + QCLASS(2)
    let question_end = pos;

    println!(
        "[サーバー] 受信クエリ: {qname} {} (id={id})",
        type_name(qtype)
    );

    // 辞書ルックアップ
    let addrs: Vec<Ipv4Addr> = if qtype == TYPE_A {
        records.get(&qname).cloned().unwrap_or_default()
    } else {
        Vec::new() // A 以外は未対応
    };

    let found = records.contains_key(&qname);
    let ancount = addrs.len() as u16;
    let rcode: u16 = if found { 0 } else { 3 }; // 3 = NXDOMAIN

    // --- 応答を組み立てる ---
    let mut resp = Vec::new();

    // ヘッダー
    resp.extend_from_slice(&id.to_be_bytes()); // ID をエコー
    // フラグ: QR=1(応答), Opcode=0, AA=1(権威), RD=エコー, RA=0
    // 1 0000 1 0 1 0 000 + rcode  →  0x8400 | rcode
    let flags = 0x8400u16 | rcode;
    resp.extend_from_slice(&flags.to_be_bytes());
    resp.extend_from_slice(&qdcount.to_be_bytes());
    resp.extend_from_slice(&ancount.to_be_bytes());
    resp.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT
    resp.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT

    // Question セクションをそのままエコー
    resp.extend_from_slice(&buf[12..question_end]);

    // Answer セクション
    for addr in &addrs {
        // NAME: 圧縮ポインタで Question セクションを指す (オフセット12 = ヘッダー直後)
        resp.push(0xC0);
        resp.push(0x0C);
        resp.extend_from_slice(&TYPE_A.to_be_bytes()); // TYPE
        resp.extend_from_slice(&CLASS_IN.to_be_bytes()); // CLASS
        resp.extend_from_slice(&60u32.to_be_bytes()); // TTL = 60秒
        resp.extend_from_slice(&4u16.to_be_bytes()); // RDLENGTH
        resp.extend_from_slice(&addr.octets()); // RDATA
    }

    resp
}

fn run_server(port: u16) {
    let addr = format!("127.0.0.1:{port}");
    let socket = UdpSocket::bind(&addr).expect("バインドに失敗");
    println!("[サーバー] {addr} で UDP リッスン開始");

    // 簡易ゾーン (権威データ): ドメイン名 → IPv4 アドレス
    let mut records: HashMap<String, Vec<Ipv4Addr>> = HashMap::new();
    records.insert(
        "example.local".to_string(),
        vec![Ipv4Addr::new(127, 0, 0, 1)],
    );
    records.insert(
        "www.example.local".to_string(),
        vec![Ipv4Addr::new(192, 168, 1, 100)],
    );
    records.insert(
        "api.example.local".to_string(),
        vec![Ipv4Addr::new(10, 0, 0, 1), Ipv4Addr::new(10, 0, 0, 2)],
    );
    println!("[サーバー] 登録ドメイン:");
    for (name, ips) in &records {
        println!("  {name} → {ips:?}");
    }

    let mut buf = [0u8; 4096];
    loop {
        match socket.recv_from(&mut buf) {
            Ok((n, src)) if n >= 12 => {
                let response = handle_query(&buf[..n], &records);
                socket.send_to(&response, src).ok();
                println!("[サーバー] → {src} に応答 ({} bytes)", response.len());
            }
            Ok((n, src)) => {
                eprintln!("[サーバー] {src} から短すぎる UDP データ ({n} bytes)");
            }
            Err(e) => eprintln!("[サーバー] 受信エラー: {e}"),
        }
    }
}

// =============================================================================
// メイン
// =============================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("demo");

    match mode {
        "resolver" => {
            // 例: cargo run -- resolver example.com 8.8.8.8:53
            let qname = args.get(2).map(String::as_str).unwrap_or("example.com");
            let server = args.get(3).map(String::as_str).unwrap_or(REAL_DNS_SERVER);
            run_resolver(server, qname, TYPE_A);
        }
        "server" => {
            run_server(DEMO_SERVER_PORT);
        }
        "demo" => {
            println!("=== net-03 DNS デモ ===\n");

            println!("--- Phase 1: 本物の Google Public DNS に問い合わせ ---");
            println!("(インターネット接続が無いとここは失敗するが、Phase 2 は動く)\n");
            run_resolver(REAL_DNS_SERVER, "example.com", TYPE_A);
            run_resolver(REAL_DNS_SERVER, "www.rust-lang.org", TYPE_A);

            println!("\n--- Phase 2: 自前の簡易DNSサーバーを起動して問い合わせ ---\n");
            std::thread::spawn(|| run_server(DEMO_SERVER_PORT));
            std::thread::sleep(Duration::from_millis(150));

            let local = format!("127.0.0.1:{DEMO_SERVER_PORT}");
            run_resolver(&local, "example.local", TYPE_A);
            run_resolver(&local, "www.example.local", TYPE_A);
            run_resolver(&local, "api.example.local", TYPE_A);
            run_resolver(&local, "nonexistent.local", TYPE_A);

            println!("\n=== デモ終了 ===");
            println!("\n別ターミナルで試すには:");
            println!("  cargo run -- server                                  # サーバー起動");
            println!(
                "  cargo run -- resolver example.local 127.0.0.1:5300   # 自前サーバーに問い合わせ"
            );
            println!("  cargo run -- resolver example.com 8.8.8.8:53         # 本物に問い合わせ");
            println!("  dig @127.0.0.1 -p 5300 example.local                 # dig でも当然OK");
        }
        other => {
            eprintln!("不明なモード: {other}");
            eprintln!("使い方: cargo run -- [resolver <name> [server]|server|demo]");
        }
    }
}

// =============================================================================
// 演習
// =============================================================================

// --- 演習1: 基礎 ---
// サーバーに AAAA レコード (IPv6) サポートを追加しよう。
//
// 現状の records は HashMap<String, Vec<Ipv4Addr>> だが、IPv6 も保持できる構造に変える。
// たとえば:
//
//   struct Zone {
//       a:    HashMap<String, Vec<Ipv4Addr>>,
//       aaaa: HashMap<String, Vec<Ipv6Addr>>,
//   }
//
// handle_query で qtype が TYPE_AAAA のときに 16バイトの RDATA を返すように分岐する。
// 動作確認: dig @127.0.0.1 -p 5300 ipv6.example.local AAAA

// --- 演習2: 応用 ---
// 圧縮ポインタを「書く」側の処理を増やそう。
//
// 現状は Answer の NAME を Question を指すポインタ (0xC0 0x0C) で書いている。
// CNAME レコードの RDATA はドメイン名なので、ここにも圧縮ポインタを使うとレスポンスが小さくなる。
//
// records に CNAME 用のフィールドを足し:
//   "blog.example.local" -CNAME-> "www.example.local"
// のような連鎖を返せるようにしてみる。
//
// ヒント:
//   - 同じドメイン名を複数回書くなら、最初に書いた位置にポインタを向ければOK
//   - 自前の追跡が面倒なら、毎回フルに書いてもよい

// --- 演習3: チャレンジ ---
// 反復問い合わせ (iterative resolution) を実装してみよう。
//
// 通常の `resolve()` は「8.8.8.8 に丸投げ」する **再帰問い合わせ**。
// 反復問い合わせはこう動く:
//
//   1. ルートサーバー (a.root-servers.net = 198.41.0.4) に "example.com A?" を問い合わせ
//   2. ルートは「.com の権威サーバーはここ」と Authority+Additional で返す
//   3. .com の権威サーバーに同じクエリ → 「example.com の権威ネームサーバーはここ」
//   4. example.com の権威サーバーに同じクエリ → A レコードが返る
//
// 学習のポイント:
//   - Authority / Additional セクションのパース
//   - 「再帰要求 (RD=1) を立てない」クエリの作り方
//   - ネームサーバー追跡のロジック
//
// 完成すると、ルートからゆっくり順に辿っていく様子が観察できて壮観。
