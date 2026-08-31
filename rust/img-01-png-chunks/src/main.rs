//! img-01: PNG チャンクパーサ
//!
//! PNG ファイルを読み込み、「シグネチャ + チャンクの列」という構造を
//! 目に見える形にダンプする。CRC-32 も自前で計算して整合性を検証する。
//!
//! 実行:
//!   cargo run -p img-01-png-chunks                       # sample.png を読む
//!   cargo run -p img-01-png-chunks -- broken-crc.png     # CRC を壊した版

use std::env;
use std::fs;
use std::process;

/// PNG ファイルの先頭 8 バイト（マジックナンバー）。
///
/// ただの目印ではなく、1バイトずつに「転送事故を検出する」意図が込められている。
/// 詳細は print_signature() のコメントを参照。
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// PNG の1チャンク。
///
/// ファイル上のレイアウトは常にこの4フィールドの並び:
///
/// ```text
///   +--------+--------+------------------+--------+
///   | length | type   | data             | CRC    |
///   | 4バイト | 4バイト | length バイト     | 4バイト |
///   +--------+--------+------------------+--------+
/// ```
///
/// `data` は元のバイト列への借用。コピーせずに参照だけ持つので
/// 構造体にライフタイム `'a` が付く（Go なら `[]byte` のスライスに相当）。
struct Chunk<'a> {
    /// ファイル先頭からのオフセット（学習用。仕様には存在しない）
    offset: usize,
    /// データ部の長さ。type と CRC は含まない
    length: u32,
    /// チャンク型。ASCII 4文字だが、大文字小文字がフラグを兼ねる
    chunk_type: [u8; 4],
    data: &'a [u8],
    /// ファイルに記録されていた CRC-32 の値
    crc: u32,
}

impl Chunk<'_> {
    fn type_str(&self) -> &str {
        // 仕様上は必ず ASCII 英字。壊れたファイルでもパニックさせたくないので from_utf8 で判定
        std::str::from_utf8(&self.chunk_type).unwrap_or("????")
    }

    /// 1バイト目が大文字 = 必須(critical)チャンク。
    /// 小文字なら補助(ancillary)で、デコーダは知らなければ読み飛ばしてよい。
    fn is_critical(&self) -> bool {
        self.chunk_type[0].is_ascii_uppercase()
    }

    /// 2バイト目が大文字 = 公開(public)チャンク。仕様で登録済みの型。
    /// 小文字はアプリケーション独自の私用チャンク。
    fn is_private(&self) -> bool {
        self.chunk_type[1].is_ascii_lowercase()
    }

    /// 3バイト目は予約済み。現行仕様では必ず大文字でなければならない。
    fn is_reserved_bit_ok(&self) -> bool {
        self.chunk_type[2].is_ascii_uppercase()
    }

    /// 4バイト目が小文字 = safe-to-copy。
    /// 「画像を編集するツールが、意味を知らなくてもそのまま出力にコピーしてよい」の意味。
    fn is_safe_to_copy(&self) -> bool {
        self.chunk_type[3].is_ascii_lowercase()
    }

    /// CRC-32 を自前で計算する。対象は **型 + データ**（長さフィールドは含まない）。
    fn computed_crc(&self) -> u32 {
        crc32(&[&self.chunk_type[..], self.data])
    }

    fn attributes(&self) -> String {
        let mut attrs = vec![
            if self.is_critical() {
                "必須"
            } else {
                "補助"
            },
            if self.is_private() {
                "私用"
            } else {
                "公開"
            },
            if self.is_safe_to_copy() {
                "コピー可"
            } else {
                "コピー不可"
            },
        ];
        if !self.is_reserved_bit_ok() {
            attrs.push("予約ビット異常");
        }
        attrs.join("/")
    }
}

/// IHDR チャンクの中身（13バイト固定）。
struct Ihdr {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    compression: u8,
    filter: u8,
    interlace: u8,
}

fn main() {
    // 引数がなければ同ディレクトリの sample.png
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| format!("{}/sample.png", env!("CARGO_MANIFEST_DIR")));

    if let Err(e) = run(&path) {
        eprintln!("エラー: {e}");
        process::exit(1);
    }
}

fn run(path: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| format!("{path} を読めない: {e}"))?;

    println!("=== PNG ダンプ: {path} ({} バイト) ===", bytes.len());

    print_signature(&bytes)?;
    let chunks = parse_chunks(&bytes)?;
    print_chunk_table(&chunks);
    print_ihdr(&chunks)?;
    print_idat_summary(&chunks);

    Ok(())
}

// ---------------------------------------------------------------------------
// ① シグネチャ
// ---------------------------------------------------------------------------

fn print_signature(bytes: &[u8]) -> Result<(), String> {
    println!("\n--- ① シグネチャ (先頭8バイト) ---");

    if bytes.len() < PNG_SIGNATURE.len() || bytes[..8] != PNG_SIGNATURE[..] {
        return Err("PNG シグネチャが一致しない。PNG ファイルではない".to_string());
    }

    // マジックナンバーの1バイトずつに意図がある。
    // 「テキストファイルとして転送されて壊れた」ケースを検出するための設計。
    println!("  89          非ASCIIバイト。7bit伝送路を通ると消えるので検出できる");
    println!("  50 4E 47    \"PNG\" — 人間がファイルを覗いたときの目印");
    println!("  0D 0A       CRLF — LF に潰す転送 (テキストモードFTP等) を検出");
    println!("  1A          Ctrl-Z — MS-DOS の TYPE コマンドで表示が止まる");
    println!("  0A          LF — CRLF に膨らませる転送を検出");
    println!("  → OK");

    Ok(())
}

// ---------------------------------------------------------------------------
// ② チャンクの切り分け
// ---------------------------------------------------------------------------

fn parse_chunks(bytes: &[u8]) -> Result<Vec<Chunk<'_>>, String> {
    let mut chunks = Vec::new();
    let mut pos = PNG_SIGNATURE.len();

    while pos < bytes.len() {
        // チャンクは最低でも length(4) + type(4) + CRC(4) = 12 バイト
        if bytes.len() - pos < 12 {
            return Err(format!(
                "オフセット {pos}: 残り {} バイトではチャンクとして成立しない",
                bytes.len() - pos
            ));
        }

        let length = read_u32(bytes, pos);

        // ここは地味だが重要。length はファイルから読んだ「他人が書いた数値」なので、
        // 検証せずに足し算すると簡単に範囲外アクセスや整数オーバーフローに繋がる。
        // 仕様上 length の上限は 2^31-1。まずそこで弾く。
        if length > 0x7FFF_FFFF {
            return Err(format!(
                "オフセット {pos}: チャンク長 {length} が仕様上限 (2^31-1) を超えている"
            ));
        }

        let data_start = pos + 8;
        let data_end = data_start + length as usize;
        if data_end + 4 > bytes.len() {
            return Err(format!(
                "オフセット {pos}: チャンク長 {length} がファイル末尾を超えている (残り {} バイト)",
                bytes.len() - data_start
            ));
        }

        chunks.push(Chunk {
            offset: pos,
            length,
            chunk_type: [
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ],
            data: &bytes[data_start..data_end],
            crc: read_u32(bytes, data_end),
        });

        pos = data_end + 4;
    }

    // 構造上の最低条件: 先頭は IHDR、末尾は IEND
    match chunks.first() {
        Some(c) if c.chunk_type == *b"IHDR" => {}
        Some(c) => return Err(format!("最初のチャンクが IHDR ではない: {}", c.type_str())),
        None => return Err("チャンクが1つもない".to_string()),
    }
    match chunks.last() {
        Some(c) if c.chunk_type == *b"IEND" => {}
        Some(c) => return Err(format!("最後のチャンクが IEND ではない: {}", c.type_str())),
        None => unreachable!(),
    }

    Ok(chunks)
}

/// ビッグエンディアンの u32 を読む。
/// PNG のマルチバイト整数は**すべてビッグエンディアン**（ネットワークバイトオーダー）。
fn read_u32(bytes: &[u8], pos: usize) -> u32 {
    u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
}

fn print_chunk_table(chunks: &[Chunk]) {
    println!("\n--- ② チャンク一覧 ---");
    println!("  offset  type  length  CRC         判定  属性");
    println!("  ------  ----  ------  ----------  ----  --------------------");

    let mut ng = 0;
    for c in chunks {
        let ok = c.crc == c.computed_crc();
        if !ok {
            ng += 1;
        }
        println!(
            "  {:>6}  {:4}  {:>6}  0x{:08x}  {:4}  {}",
            c.offset,
            c.type_str(),
            c.length,
            c.crc,
            if ok { "OK" } else { "NG" },
            c.attributes(),
        );
    }

    if ng > 0 {
        println!("\n  ⚠ CRC 不一致が {ng} 個ある（ファイルが壊れているか改変されている）");
    }
}

// ---------------------------------------------------------------------------
// CRC-32 (自前実装)
// ---------------------------------------------------------------------------

/// CRC-32 の計算テーブル (IEEE 802.3 多項式の反転表現 0xEDB88320)。
///
/// CRC は「ビット列を多項式とみなした割り算の余り」。
/// 1ビットずつ回すと遅いので、8ビット分の結果を先に256通り作っておく定番の高速化。
///
/// 中身の数学より「1バイト読むごとにテーブル1回引いて混ぜる」という
/// 動きだけ掴めば十分。
fn crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for (n, entry) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *entry = c;
    }
    table
}

/// 複数のバイト列を連結したものとして CRC-32 を計算する。
/// PNG では「型4バイト」と「データ」を連結した値が対象。
fn crc32(parts: &[&[u8]]) -> u32 {
    let table = crc32_table();
    let mut crc = 0xFFFF_FFFFu32; // 初期値は全ビット1
    for part in parts {
        for &byte in *part {
            crc = table[((crc ^ byte as u32) & 0xFF) as usize] ^ (crc >> 8);
        }
    }
    crc ^ 0xFFFF_FFFF // 最後に全ビット反転
}

// ---------------------------------------------------------------------------
// ③ IHDR の解釈
// ---------------------------------------------------------------------------

fn print_ihdr(chunks: &[Chunk]) -> Result<(), String> {
    let ihdr_chunk = chunks
        .iter()
        .find(|c| c.chunk_type == *b"IHDR")
        .ok_or("IHDR チャンクがない")?;
    let ihdr = parse_ihdr(ihdr_chunk.data)?;

    println!("\n--- ③ IHDR (画像ヘッダ) ---");
    println!("  幅          : {} px", ihdr.width);
    println!("  高さ        : {} px", ihdr.height);
    println!("  ビット深度  : {} bit / チャンネル", ihdr.bit_depth);
    println!(
        "  カラータイプ: {} ({})",
        ihdr.color_type,
        color_type_name(ihdr.color_type)
    );
    println!(
        "  圧縮方式    : {} ({})",
        ihdr.compression,
        if ihdr.compression == 0 {
            "deflate/inflate — 現行仕様ではこれ以外は存在しない"
        } else {
            "未定義"
        }
    );
    println!(
        "  フィルタ方式: {} ({})",
        ihdr.filter,
        if ihdr.filter == 0 {
            "adaptive filtering — 行ごとに5種から選ぶ方式"
        } else {
            "未定義"
        }
    );
    println!(
        "  インターレース: {} ({})",
        ihdr.interlace,
        match ihdr.interlace {
            0 => "なし (上から順に1回で送る)",
            1 => "Adam7 (粗い画像から7段階で送る)",
            _ => "未定義",
        }
    );

    // カラータイプとビット深度が決まると「1ピクセル何ビットか」が決まり、
    // そこから「zlib を展開したら何バイト出てくるはず」が計算できる。
    // これは img-02 以降の土台であり、同時に「展開爆弾」の防御にもなる。
    if let Some(ch) = channels(ihdr.color_type) {
        let bits_per_pixel = ch * u32::from(ihdr.bit_depth);
        // 1行 = フィルタタイプ1バイト + ピクセルデータ（バイト境界に切り上げ）
        let bytes_per_row = 1 + (u64::from(ihdr.width) * u64::from(bits_per_pixel)).div_ceil(8);
        let raw_size = bytes_per_row * u64::from(ihdr.height);

        println!(
            "\n  → 1ピクセル {ch} チャンネル × {} bit = {bits_per_pixel} bit",
            ihdr.bit_depth
        );
        println!("  → 1行 {bytes_per_row} バイト (先頭1バイトはフィルタタイプ)");
        println!("  → zlib 展開後の想定サイズ: {raw_size} バイト  ← img-03/04 で実際に確認する");
    }

    Ok(())
}

fn parse_ihdr(data: &[u8]) -> Result<Ihdr, String> {
    if data.len() != 13 {
        return Err(format!("IHDR は13バイトのはずが {} バイト", data.len()));
    }
    Ok(Ihdr {
        width: read_u32(data, 0),
        height: read_u32(data, 4),
        bit_depth: data[8],
        color_type: data[9],
        compression: data[10],
        filter: data[11],
        interlace: data[12],
    })
}

fn color_type_name(color_type: u8) -> &'static str {
    match color_type {
        0 => "グレースケール",
        2 => "トゥルーカラー RGB",
        3 => "パレット (インデックスカラー)",
        4 => "グレースケール + アルファ",
        6 => "トゥルーカラー + アルファ RGBA",
        _ => "不明",
    }
}

/// 1ピクセルあたりのチャンネル数。
/// パレット画像は「パレット番号1つ」なので 1。
fn channels(color_type: u8) -> Option<u32> {
    match color_type {
        0 | 3 => Some(1),
        2 => Some(3),
        4 => Some(2),
        6 => Some(4),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// ④ IDAT のまとめ
// ---------------------------------------------------------------------------

fn print_idat_summary(chunks: &[Chunk]) {
    let idats: Vec<&Chunk> = chunks.iter().filter(|c| c.chunk_type == *b"IDAT").collect();
    let total: usize = idats.iter().map(|c| c.data.len()).sum();

    println!("\n--- ④ IDAT (画像データ) ---");
    println!("  IDAT チャンク数: {}", idats.len());
    println!("  データ合計      : {total} バイト");
    println!("  → 全 IDAT の data を**順に連結すると1本の zlib ストリーム**になる。");
    println!("    分割されているのはストリーミング書き出しのため（1チャンクの上限ではない）。");

    if total >= 2 {
        // zlib ヘッダ (RFC 1950) の先頭2バイトを覗いておく
        let data = idats[0].data;
        let cmf = data[0];
        let flg = data[1];
        println!(
            "  → 先頭2バイト 0x{cmf:02x} 0x{flg:02x} = zlib ヘッダ (圧縮方式 {}, ウィンドウ {} バイト)",
            cmf & 0x0F,
            1u32 << ((cmf >> 4) + 8)
        );
    }
}

// ===========================================================================
// 演習
// ===========================================================================

// --- 演習1: 基礎 ---
// 「特定のチャンク型だけ表示する」オプションを足そう。
//
//   cargo run -p img-01-png-chunks -- sample.png --only IDAT
//
// ヒント:
//   - env::args() を集めて Vec<String> にし、"--only" の次の要素を取る
//   - print_chunk_table に filter: Option<&str> を渡し、
//     chunks.iter().filter(|c| filter.is_none_or(|f| c.type_str() == f)) で絞る
//
// Go なら flag パッケージを使うところだが、std だけで書くとこうなる、を体感する。

// --- 演習2: 応用 ---
// tEXt チャンクの中身を読んでみよう。
//
// tEXt のデータ部のレイアウトは:
//
//   キーワード (1〜79バイト) + 0x00 (区切り) + テキスト本体
//
// 文字コードは UTF-8 ではなく Latin-1 (ISO 8859-1)。
// ASCII 範囲ならそのまま、それ以外は char::from(byte) で1バイト=1文字として変換できる。
//
//   fn parse_text_chunk(data: &[u8]) -> Option<(String, String)> {
//       let sep = data.iter().position(|&b| b == 0)?;
//       let keyword = data[..sep].iter().map(|&b| char::from(b)).collect();
//       let text    = data[sep + 1..].iter().map(|&b| char::from(b)).collect();
//       Some((keyword, text))
//   }
//
// sample.png には Comment キーワードの tEXt が入っている。
// 余裕があれば zTXt (zlib 圧縮テキスト) と iTXt (UTF-8) の違いも仕様で確認してみる。

// --- 演習3: チャレンジ ---
// CRC 不一致を「チャンクの属性に応じて」扱い分ける堅牢なパーサにしよう。
//
// 同梱の broken-crc.png は tEXt チャンクの CRC を 1 バイト壊してある。
// 現状の実装は NG と表示するだけで読み進めてしまう。本来こう振る舞うべき:
//
//   - 必須(critical)チャンクの CRC 不一致 → 画像として信用できないのでエラー終了
//   - 補助(ancillary)チャンクの CRC 不一致 → 警告を出してそのチャンクだけ捨て、続行
//
// これは「未知の補助チャンクは読み飛ばしてよい」という PNG の前方互換設計と
// 同じ思想。デコーダが多少壊れたファイルでも画像を出せるのはこのおかげ。
//
// さらに踏み込むなら:
//   - IHDR が2つある / IEND の後にデータが続く といった構造異常も検出する
//   - 実際の画像ビューアや ImageMagick に broken-crc.png を食わせて挙動を比べてみる
//     （多くのツールは補助チャンクの CRC エラーを黙って無視する）
