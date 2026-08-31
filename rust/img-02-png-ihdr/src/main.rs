//! img-02: PNG IHDR 解釈器
//!
//! img-01 で「入れ物」を切り分けられるようになったので、次は中身の設計図を読む。
//! IHDR の13バイトから「1ピクセルが何ビットか」「zlib を展開したら何バイト出るか」を
//! 確定させるのがこのトピックの仕事。この2つが決まらないと img-03/04 に進めない。
//!
//! 実行:
//!   cargo run -p img-02-png-ihdr                              # sample-rgb8.png
//!   cargo run -p img-02-png-ihdr -- sample-palette4.png       # 個別に見る
//!   cargo run -p img-02-png-ihdr -- --all                     # 全サンプルを一覧比較
//!   cargo run -p img-02-png-ihdr -- --table                   # 合法な組み合わせ表

use std::env;
use std::fs;
use std::process;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// 展開後サイズがこれを超えたら警告する（学習用の閾値）。
/// 実用デコーダも同種の上限を持っていて、これが「展開爆弾」への基本的な防御になる。
const RAW_SIZE_WARN: u64 = 256 * 1024 * 1024;

// ---------------------------------------------------------------------------
// チャンク（img-01 の簡略版。CRC 検証は img-01 に任せ、ここでは切り分けだけ）
// ---------------------------------------------------------------------------

struct Chunk<'a> {
    chunk_type: [u8; 4],
    data: &'a [u8],
}

impl Chunk<'_> {
    fn type_str(&self) -> &str {
        std::str::from_utf8(&self.chunk_type).unwrap_or("????")
    }
}

// ---------------------------------------------------------------------------
// カラータイプ
// ---------------------------------------------------------------------------

/// PNG のカラータイプ。値は仕様で決まっており 1/5 は欠番。
///
/// 「0〜6 の u8」ではなく enum にしておくと、`match` の網羅性をコンパイラが見てくれる。
/// Go なら `const ( Grayscale ColorType = 0; ... )` と書くところだが、
/// Go の場合 `ColorType(7)` も型としては通ってしまう。ここが Rust との差。
#[derive(Clone, Copy, PartialEq, Eq)]
enum ColorType {
    Grayscale,
    Rgb,
    Palette,
    GrayscaleAlpha,
    Rgba,
}

impl ColorType {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Grayscale),
            2 => Some(Self::Rgb),
            3 => Some(Self::Palette),
            4 => Some(Self::GrayscaleAlpha),
            6 => Some(Self::Rgba),
            _ => None, // 1, 5, 7 以上は仕様に存在しない
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::Grayscale => 0,
            Self::Rgb => 2,
            Self::Palette => 3,
            Self::GrayscaleAlpha => 4,
            Self::Rgba => 6,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Grayscale => "グレースケール",
            Self::Rgb => "トゥルーカラー RGB",
            Self::Palette => "パレット (インデックスカラー)",
            Self::GrayscaleAlpha => "グレースケール + アルファ",
            Self::Rgba => "トゥルーカラー + アルファ RGBA",
        }
    }

    /// 1ピクセルあたりのサンプル数。
    /// パレットは「パレット番号ひとつ」なので 1。実際の色数とは無関係。
    fn channels(self) -> u32 {
        match self {
            Self::Grayscale | Self::Palette => 1,
            Self::Rgb => 3,
            Self::GrayscaleAlpha => 2,
            Self::Rgba => 4,
        }
    }

    /// 展開後のバイト列で1ピクセルがどう並ぶか（人間向けの説明用）。
    fn channel_layout(self) -> &'static str {
        match self {
            Self::Grayscale => "[gray]",
            Self::Rgb => "[R][G][B]",
            Self::Palette => "[index]",
            Self::GrayscaleAlpha => "[gray][A]",
            Self::Rgba => "[R][G][B][A]",
        }
    }

    /// このカラータイプで許されるビット深度。
    ///
    /// 全部が全部使えるわけではない、というのがこのトピックの山場のひとつ。
    /// - 2/4/6 が 8/16 に限られるのは「1サンプルが必ずバイト境界に揃う」ようにするため
    /// - 3 が 8 止まりなのはパレットが最大256色だから（16bit のインデックスは無意味）
    /// - 0 だけが 1〜16 の全部を使える（白黒2値の FAX 的な画像を1bitで持てる）
    fn allowed_depths(self) -> &'static [u8] {
        match self {
            Self::Grayscale => &[1, 2, 4, 8, 16],
            Self::Palette => &[1, 2, 4, 8],
            Self::Rgb | Self::GrayscaleAlpha | Self::Rgba => &[8, 16],
        }
    }

    /// PLTE チャンクの要否。
    fn plte_requirement(self) -> &'static str {
        match self {
            Self::Palette => "必須",
            Self::Rgb | Self::Rgba => "任意 (推奨パレットとして使える)",
            Self::Grayscale | Self::GrayscaleAlpha => "禁止",
        }
    }
}

// ---------------------------------------------------------------------------
// IHDR
// ---------------------------------------------------------------------------

/// IHDR チャンクの生の中身（13バイト固定）。
struct Ihdr {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    compression: u8,
    filter: u8,
    interlace: u8,
}

/// IHDR から導出される「展開後のバイト列の形」。
///
/// img-03 (inflate) と img-04 (フィルタ復元) は、この構造体の値だけを見て動ける。
struct Layout {
    color: ColorType,
    bit_depth: u8,
    /// 1ピクセルのビット数 = チャンネル数 × ビット深度
    bits_per_pixel: u32,
    /// フィルタ処理で「左隣のピクセル」を指すためのバイト距離。
    /// 1ピクセルが1バイト未満のときは 1 に切り上げる（img-04 の伏線）。
    filter_bpp: u32,
    /// 1行のピクセル部のバイト数（フィルタタイプの1バイトを含まない）
    row_data_bytes: u64,
    /// 1行の総バイト数 = 1 + row_data_bytes
    bytes_per_row: u64,
    /// 各行の末尾で捨てられるビット数（バイト境界への切り上げ分）
    padding_bits: u64,
    /// 展開後の総バイト数（非インターレース時）
    raw_size: u64,
}

impl Layout {
    fn new(ihdr: &Ihdr) -> Result<Self, String> {
        let color = ColorType::from_u8(ihdr.color_type)
            .ok_or_else(|| format!("カラータイプ {} は仕様に存在しない", ihdr.color_type))?;

        if !color.allowed_depths().contains(&ihdr.bit_depth) {
            return Err(format!(
                "カラータイプ {} にビット深度 {} の組み合わせは不正 (許されるのは {:?})",
                color.code(),
                ihdr.bit_depth,
                color.allowed_depths()
            ));
        }

        let bits_per_pixel = color.channels() * u32::from(ihdr.bit_depth);

        // 幅×ビット数は u32 で溢れうる（幅の上限は 2^31-1）。最初から u64 に広げる。
        let row_bits = u64::from(ihdr.width) * u64::from(bits_per_pixel);
        let row_data_bytes = row_bits.div_ceil(8);
        let bytes_per_row = 1 + row_data_bytes;

        Ok(Self {
            color,
            bit_depth: ihdr.bit_depth,
            bits_per_pixel,
            filter_bpp: bits_per_pixel.div_ceil(8).max(1),
            row_data_bytes,
            bytes_per_row,
            padding_bits: row_data_bytes * 8 - row_bits,
            raw_size: bytes_per_row * u64::from(ihdr.height),
        })
    }

    /// 1バイトに何ピクセル詰まっているか（サブバイト深度のときだけ意味を持つ）。
    fn pixels_per_byte(&self) -> Option<u32> {
        (self.bits_per_pixel < 8).then(|| 8 / self.bits_per_pixel)
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let result = match args.first().map(String::as_str) {
        Some("--table") => {
            print_combination_table();
            Ok(())
        }
        Some("--all") => print_all_summary(),
        Some(path) => inspect(path),
        None => inspect(&format!("{}/sample-rgb8.png", env!("CARGO_MANIFEST_DIR"))),
    };

    if let Err(e) = result {
        eprintln!("エラー: {e}");
        process::exit(1);
    }
}

fn inspect(path: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| format!("{path} を読めない: {e}"))?;
    let chunks = parse_chunks(&bytes)?;

    let ihdr_chunk = chunks
        .iter()
        .find(|c| c.chunk_type == *b"IHDR")
        .ok_or("IHDR チャンクがない")?;
    let ihdr = parse_ihdr(ihdr_chunk.data)?;
    let layout = Layout::new(&ihdr)?;

    println!("=== IHDR 解析: {path} ({} バイト) ===", bytes.len());
    print_ihdr_bytes(ihdr_chunk.data, &ihdr);
    print_validation(&ihdr, &layout, &chunks);
    print_pixel_layout(&layout);
    print_raw_size(&ihdr, &layout, &chunks);
    print_ancillary(&ihdr, &layout, &chunks);

    Ok(())
}

// ---------------------------------------------------------------------------
// ① IHDR の13バイト
// ---------------------------------------------------------------------------

fn print_ihdr_bytes(data: &[u8], ihdr: &Ihdr) {
    println!("\n--- ① IHDR の13バイト ---");
    println!("  offset  bytes        意味");
    println!("  ------  -----------  --------------------------------------------");
    println!(
        "     0-3  {:02x} {:02x} {:02x} {:02x}  幅 {} px",
        data[0], data[1], data[2], data[3], ihdr.width
    );
    println!(
        "     4-7  {:02x} {:02x} {:02x} {:02x}  高さ {} px",
        data[4], data[5], data[6], data[7], ihdr.height
    );
    println!(
        "       8  {:02x}           ビット深度 {} bit / チャンネル",
        data[8], ihdr.bit_depth
    );
    println!(
        "       9  {:02x}           カラータイプ {} = {}",
        data[9],
        ihdr.color_type,
        ColorType::from_u8(ihdr.color_type).map_or("不明", ColorType::name)
    );
    println!(
        "      10  {:02x}           圧縮方式 {} ({})",
        data[10],
        ihdr.compression,
        if ihdr.compression == 0 {
            "deflate — 現行仕様ではこれ以外は存在しない"
        } else {
            "未定義"
        }
    );
    println!(
        "      11  {:02x}           フィルタ方式 {} ({})",
        data[11],
        ihdr.filter,
        if ihdr.filter == 0 {
            "adaptive — 行ごとに5種から選ぶ"
        } else {
            "未定義"
        }
    );
    println!(
        "      12  {:02x}           インターレース {} ({})",
        data[12],
        ihdr.interlace,
        match ihdr.interlace {
            0 => "なし",
            1 => "Adam7",
            _ => "未定義",
        }
    );
}

// ---------------------------------------------------------------------------
// ② 組み合わせの検証
// ---------------------------------------------------------------------------

fn print_validation(ihdr: &Ihdr, layout: &Layout, chunks: &[Chunk]) {
    println!("\n--- ② 組み合わせの検証 ---");

    let depths: Vec<String> = layout
        .color
        .allowed_depths()
        .iter()
        .map(u8::to_string)
        .collect();
    println!(
        "  カラータイプ {} が許す深度: {}",
        layout.color.code(),
        depths.join(", ")
    );
    println!("  → 深度 {} は合法", layout.bit_depth);

    let plte = chunks.iter().find(|c| c.chunk_type == *b"PLTE");
    println!("  PLTE の要否: {}", layout.color.plte_requirement());

    let mut problems: Vec<String> = Vec::new();

    if ihdr.width == 0 || ihdr.height == 0 {
        problems.push("幅または高さが 0（仕様上 1 以上）".to_string());
    }
    if ihdr.compression != 0 {
        problems.push(format!("圧縮方式 {} は未定義", ihdr.compression));
    }
    if ihdr.filter != 0 {
        problems.push(format!("フィルタ方式 {} は未定義", ihdr.filter));
    }
    if ihdr.interlace > 1 {
        problems.push(format!("インターレース {} は未定義", ihdr.interlace));
    }

    match (layout.color, plte) {
        (ColorType::Palette, None) => {
            problems.push("カラータイプ 3 なのに PLTE がない".to_string());
        }
        (ColorType::Grayscale | ColorType::GrayscaleAlpha, Some(_)) => {
            problems.push("グレースケール画像に PLTE があってはならない".to_string());
        }
        _ => {}
    }

    if let Some(p) = plte {
        let entries = p.data.len() / 3;
        let max = 1usize << layout.bit_depth;
        if p.data.len() % 3 != 0 {
            problems.push(format!("PLTE の長さ {} が3の倍数でない", p.data.len()));
        }
        if layout.color == ColorType::Palette && entries > max {
            problems.push(format!(
                "PLTE が {entries} 色あるが、深度 {} で表せるのは {max} 色まで",
                layout.bit_depth
            ));
        }
        println!(
            "  PLTE: {entries} 色 (深度 {} の上限 {max} 色)",
            layout.bit_depth
        );
    }

    if problems.is_empty() {
        println!("  → 問題なし");
    } else {
        for p in &problems {
            println!("  ⚠ {p}");
        }
    }
}

// ---------------------------------------------------------------------------
// ③ 1ピクセルのレイアウト
// ---------------------------------------------------------------------------

fn print_pixel_layout(layout: &Layout) {
    println!("\n--- ③ 1ピクセルのレイアウト ---");
    println!("  チャンネル構成: {}", layout.color.channel_layout());
    println!(
        "  1ピクセル      : {} ch × {} bit = {} bit",
        layout.color.channels(),
        layout.bit_depth,
        layout.bits_per_pixel
    );

    match layout.pixels_per_byte() {
        Some(n) => {
            println!("  バイト詰め     : 1バイトに {n} ピクセル（上位ビット側から順に）");
            println!(
                "                   例: 深度 {} なら 1バイト = {}",
                layout.bit_depth,
                (0..n)
                    .map(|i| format!("px{i}"))
                    .collect::<Vec<_>>()
                    .join("|")
            );
        }
        None => println!(
            "  バイト詰め     : 1ピクセル = {} バイト（バイト境界に揃う）",
            layout.bits_per_pixel / 8
        ),
    }

    // ここが img-04 に直結する値。フィルタは「左隣のピクセル」を参照するが、
    // 1ピクセルが1バイト未満のときは 1バイト前を見る、と仕様で決まっている。
    println!(
        "  フィルタ用 bpp : {} バイト  = max(1, ceil({} bit / 8))  ← img-04 で使う",
        layout.filter_bpp, layout.bits_per_pixel
    );
}

// ---------------------------------------------------------------------------
// ④ 展開後のサイズ
// ---------------------------------------------------------------------------

fn print_raw_size(ihdr: &Ihdr, layout: &Layout, chunks: &[Chunk]) {
    println!("\n--- ④ zlib 展開後のサイズ ---");

    println!(
        "  1行 = 1 (フィルタタイプ) + ceil({} px × {} bit / 8) = 1 + {} = {} バイト",
        ihdr.width, layout.bits_per_pixel, layout.row_data_bytes, layout.bytes_per_row
    );
    println!(
        "  全体 = {} × {} 行 = {} バイト",
        layout.bytes_per_row, ihdr.height, layout.raw_size
    );

    if layout.padding_bits > 0 {
        println!(
            "  詰め物 : 各行の末尾 {} ビットは未使用。行はバイト境界で切り上げられ、",
            layout.padding_bits
        );
        println!("           次の行へまたいで詰めることはない");
    }

    let idat_total: usize = chunks
        .iter()
        .filter(|c| c.chunk_type == *b"IDAT")
        .map(|c| c.data.len())
        .sum();
    if idat_total > 0 && layout.raw_size > 0 {
        let ratio = idat_total as f64 / layout.raw_size as f64 * 100.0;
        println!(
            "  IDAT 実データ {idat_total} バイト → 展開すると {} バイトになるはず (圧縮率 {ratio:.1}%)",
            layout.raw_size
        );
        println!("  この数字が img-03 (inflate) の答え合わせに使える");
    }

    if ihdr.interlace == 1 {
        println!();
        println!("  ⚠ この画像は Adam7 インターレース。上の式は使えない。");
        println!("    Adam7 は画像を7つのパスに分けて送り、パスごとに独立した");
        println!("    スキャンライン群（＝パスごとにフィルタタイプの1バイト）を持つ。");
        println!("    実際の展開後サイズは7パスの合計になる → 演習3");
    }

    if layout.raw_size > RAW_SIZE_WARN {
        let mib = layout.raw_size as f64 / 1024.0 / 1024.0;
        println!();
        println!(
            "  ⚠ 展開後 {} バイト ({mib:.1} MiB) は閾値を超えている。",
            layout.raw_size
        );
        println!("    IHDR の幅・高さは「他人が書いた数値」なので、展開する前にここで弾く");
    }
}

// ---------------------------------------------------------------------------
// ⑤ 補助チャンク
// ---------------------------------------------------------------------------

fn print_ancillary(ihdr: &Ihdr, layout: &Layout, chunks: &[Chunk]) {
    println!("\n--- ⑤ 補助チャンク ---");

    let mut found = false;
    for c in chunks {
        if matches!(&c.chunk_type, b"IHDR" | b"IDAT" | b"IEND") {
            continue;
        }
        found = true;
        println!("  {:5} {}", c.type_str(), describe(c, ihdr, layout));
    }

    if !found {
        println!("  （なし）");
    }
}

fn describe(c: &Chunk, ihdr: &Ihdr, layout: &Layout) -> String {
    match &c.chunk_type {
        b"PLTE" => format!(
            "パレット {} 色 (3バイト×{} = {} バイト)",
            c.data.len() / 3,
            c.data.len() / 3,
            c.data.len()
        ),
        b"tRNS" => describe_trns(c, layout),
        b"gAMA" if c.data.len() == 4 => {
            let v = read_u32(c.data, 0);
            format!(
                "ガンマ値 {:.5} (整数 {v} / 100000)",
                f64::from(v) / 100_000.0
            )
        }
        b"sRGB" if c.data.len() == 1 => format!(
            "sRGB 色空間。レンダリング意図 {} ({})",
            c.data[0],
            match c.data[0] {
                0 => "Perceptual",
                1 => "Relative colorimetric",
                2 => "Saturation",
                3 => "Absolute colorimetric",
                _ => "不明",
            }
        ),
        b"pHYs" if c.data.len() == 9 => {
            let x = read_u32(c.data, 0);
            let y = read_u32(c.data, 4);
            if c.data[8] == 1 {
                format!(
                    "物理解像度 {x}×{y} px/m (≒ {:.0} DPI)",
                    f64::from(x) * 0.0254
                )
            } else {
                format!("縦横比のみ {x}:{y} (単位未指定)")
            }
        }
        b"tEXt" => match c.data.iter().position(|&b| b == 0) {
            Some(sep) => format!(
                "テキスト \"{}\"",
                c.data[..sep]
                    .iter()
                    .map(|&b| char::from(b))
                    .collect::<String>()
            ),
            None => "テキスト (区切りの 0x00 がない)".to_string(),
        },
        _ => format!(
            "{} バイト（このトピックでは未解釈。カラータイプ {} には影響しない）",
            c.data.len(),
            ihdr.color_type
        ),
    }
}

/// tRNS は「カラータイプによって中身の意味がまるごと変わる」チャンク。
/// 同じ型なのに解釈が違うので、IHDR を先に読んでいないと処理できない好例。
fn describe_trns(c: &Chunk, layout: &Layout) -> String {
    match layout.color {
        ColorType::Palette => format!(
            "パレット {} エントリ分のアルファ値 (残りは不透明扱い)",
            c.data.len()
        ),
        ColorType::Grayscale => "この輝度値を透明として扱う (2バイト)".to_string(),
        ColorType::Rgb => "この RGB 値を透明として扱う (2バイト×3)".to_string(),
        ColorType::GrayscaleAlpha | ColorType::Rgba => {
            "⚠ すでにアルファチャンネルがあるので tRNS は禁止".to_string()
        }
    }
}

// ---------------------------------------------------------------------------
// --table / --all
// ---------------------------------------------------------------------------

fn print_combination_table() {
    println!("=== カラータイプ × ビット深度 の合法な組み合わせ ===\n");
    println!("  型  ch  許される深度      1ピクセルのビット数   名前");
    println!("  --  --  ----------------  --------------------  ----------------------------");

    for color in [
        ColorType::Grayscale,
        ColorType::Rgb,
        ColorType::Palette,
        ColorType::GrayscaleAlpha,
        ColorType::Rgba,
    ] {
        let depths: Vec<String> = color.allowed_depths().iter().map(u8::to_string).collect();
        let bpps: Vec<String> = color
            .allowed_depths()
            .iter()
            .map(|d| (color.channels() * u32::from(*d)).to_string())
            .collect();
        println!(
            "  {:>2}  {:>2}  {:16}  {:20}  {}",
            color.code(),
            color.channels(),
            depths.join(", "),
            bpps.join(", "),
            color.name()
        );
    }

    println!("\n  1 と 5 は欠番。7 以上も存在しない。");
    println!("  2/4/6 が 8/16 に限られるのは、1サンプルを必ずバイト境界に揃えるため。");
    println!("  3 が 8 止まりなのはパレットが最大256色だから。");
    println!("  0 だけが 1〜16 の全部を使える（白黒2値を 1bit で持てる）。");
}

fn print_all_summary() -> Result<(), String> {
    let dir = env!("CARGO_MANIFEST_DIR");
    let mut paths: Vec<String> = fs::read_dir(dir)
        .map_err(|e| format!("{dir} を読めない: {e}"))?
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".png"))
        .collect();
    paths.sort();

    println!("=== サンプル一覧 ===\n");
    println!("  file                   幅×高さ  深度  型  ch  bpp  1行     展開後   詰め物/行");
    println!("  ---------------------- -------  ----  --  --  ---  ------  -------  ---------");

    for name in paths {
        let bytes = fs::read(format!("{dir}/{name}")).map_err(|e| e.to_string())?;
        let chunks = parse_chunks(&bytes)?;
        let ihdr_chunk = chunks
            .iter()
            .find(|c| c.chunk_type == *b"IHDR")
            .ok_or("IHDR がない")?;
        let ihdr = parse_ihdr(ihdr_chunk.data)?;
        let layout = Layout::new(&ihdr)?;

        println!(
            "  {:22} {:>3}×{:<3} {:>4}  {:>2}  {:>2}  {:>3}  {:>4} B  {:>5} B  {:>5} bit{}",
            name,
            ihdr.width,
            ihdr.height,
            layout.bit_depth,
            layout.color.code(),
            layout.color.channels(),
            layout.bits_per_pixel,
            layout.bytes_per_row,
            layout.raw_size,
            layout.padding_bits,
            if ihdr.interlace == 1 {
                "  ← Adam7"
            } else {
                ""
            },
        );
    }

    println!("\n  「展開後」は非インターレース前提の値。Adam7 の行は実際とは異なる（演習3）。");
    Ok(())
}

// ---------------------------------------------------------------------------
// パース
// ---------------------------------------------------------------------------

fn parse_chunks(bytes: &[u8]) -> Result<Vec<Chunk<'_>>, String> {
    if bytes.len() < PNG_SIGNATURE.len() || bytes[..8] != PNG_SIGNATURE[..] {
        return Err("PNG シグネチャが一致しない".to_string());
    }

    let mut chunks = Vec::new();
    let mut pos = PNG_SIGNATURE.len();

    while pos < bytes.len() {
        if bytes.len() - pos < 12 {
            return Err(format!("オフセット {pos}: チャンクとして成立しない"));
        }
        let length = read_u32(bytes, pos);
        if length > 0x7FFF_FFFF {
            return Err(format!(
                "オフセット {pos}: チャンク長 {length} が仕様上限を超える"
            ));
        }
        let data_start = pos + 8;
        let data_end = data_start + length as usize;
        if data_end + 4 > bytes.len() {
            return Err(format!(
                "オフセット {pos}: チャンク長 {length} がファイル末尾を超える"
            ));
        }

        chunks.push(Chunk {
            chunk_type: [
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ],
            data: &bytes[data_start..data_end],
        });
        pos = data_end + 4;
    }

    Ok(chunks)
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

fn read_u32(bytes: &[u8], pos: usize) -> u32 {
    u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
}

// ===========================================================================
// 演習
// ===========================================================================

// --- 演習1: 基礎 ---
// サブバイト深度の行から「n 番目のピクセルの値」を取り出す関数を書こう。
//
// 深度 4 の行 `[0x12, 0x34, 0x50]` は 5 ピクセル分 (1,2,3,4,5) を意味する。
// 上位ビット側から詰まっている点に注意（0x12 の上位4bitが px0 = 1）。
//
//   fn sample_at(row: &[u8], index: usize, depth: u8) -> u8 {
//       let per_byte = 8 / depth as usize;          // 1バイトに何個入っているか
//       let byte = row[index / per_byte];
//       let slot = index % per_byte;                // バイト内での位置（左から0）
//       let shift = 8 - depth as usize * (slot + 1); // 右へ何ビット寄せるか
//       (byte >> shift) & ((1 << depth) - 1)
//   }
//
// sample-gray1.png (深度1, 幅12) と sample-palette4.png (深度4, 幅9) の
// 1行目を取り出して、行末の詰め物ビットを読まずに済んでいるか確認すること。
// ヒント: 展開後のバイト列は img-03 が必要なので、まずはテスト用のバイト配列で動かす。

// --- 演習2: 応用 ---
// パレット画像のインデックスを実際の色に変換しよう。
//
// sample-palette4.png は PLTE 6色 + tRNS 6エントリを持つ。
//
//   struct Palette { rgb: Vec<[u8; 3]>, alpha: Vec<u8> }
//
//   fn resolve(pal: &Palette, index: u8) -> [u8; 4] {
//       let rgb = pal.rgb[index as usize];
//       // tRNS は「先頭から順に」対応する。足りない分は 255 (不透明)
//       let a = pal.alpha.get(index as usize).copied().unwrap_or(255);
//       [rgb[0], rgb[1], rgb[2], a]
//   }
//
// 注意点:
//   - PLTE のエントリ数より大きいインデックスが来たらエラー（壊れたファイル）
//   - tRNS は PLTE より短くてよい。長いのは仕様違反
//   - パレット画像は「1ピクセル4bit」でも、展開すると RGBA 32bit になる。
//     つまり展開後サイズとメモリ上のサイズは別物

// --- 演習3: チャレンジ ---
// Adam7 インターレースの展開後サイズを計算しよう。
//
// Adam7 は画像を7回に分けて送る。各パスは (開始x, x間隔, 開始y, y間隔) で決まる:
//
//   const ADAM7: [(u32, u32, u32, u32); 7] = [
//       (0, 8, 0, 8), (4, 8, 0, 8), (0, 4, 4, 8), (2, 4, 0, 4),
//       (0, 2, 2, 4), (1, 2, 0, 2), (0, 1, 1, 2),
//   ];
//
// パスごとに:
//   pass_width  = ceil((width  - x0) / dx)
//   pass_height = ceil((height - y0) / dy)
//   幅か高さが 0 のパスは丸ごと存在しない（バイトを1つも出さない）
//   そのパスのサイズ = pass_height × (1 + ceil(pass_width × bpp / 8))
//
// 全パスの合計が展開後サイズになる。ポイントは:
//   - フィルタタイプの1バイトが「パスの行ごとに」付くので、パス数だけ余分に増える
//   - 詰め物もパスごとに発生する（幅が小さいパスほど無駄が多い）
//
// sample-interlaced.png (8×8 RGB 8bit) で試すと 207 バイトになるはず。
// 同じ絵の非インターレース版 sample-rgb8.png は 200 バイトなので、
// 「インターレースにすると展開後が少し太る」ことが数字で確認できる。
