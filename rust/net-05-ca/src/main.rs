// net-05: 認証局 (CA / Certificate Authority)
// ネットワークの「登場人物」第5弾: CA = 「この証明書は本物」と保証するハンコ屋。
//
// net-04 では CA がサーバー証明書に直接署名する形だった。
// 実際は ルートCA → 中間CA → サーバー証明書 という多段の「信頼の連鎖」になっている。
// このプロジェクトでは:
//   1. 証明書 (Subject/Issuer/期限/公開鍵/署名) を構造体で表現する
//   2. ルートCA が自己署名 → 中間CA に署名 → サーバー証明書に署名
//   3. クライアントが「ルートCAだけを信頼の起点」としてチェーンを検証する
//   4. 改ざん・期限切れ・偽CA がどう弾かれるかを観察する
//
// ⚠️ 暗号はすべて教育用の toy 実装 (鍵が極小)。本番は OpenSSL / rustls + 本物の X.509。

// =============================================================================
// 数学プリミティブ (net-04 と同じ系統の toy RSA)
// =============================================================================

/// 冪剰余: base^exp mod modulus。署名 (m^d) も検証 (s^e) もこれ1つ。
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

/// 最大公約数 (e と φ(n) が互いに素か確かめるのに使う)。
fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// a の mod m における逆元 (拡張ユークリッド)。d = e^-1 mod φ(n) の計算に使う。
fn modinv(a: u64, m: u64) -> u64 {
    let (mut old_r, mut r) = (a as i128, m as i128);
    let (mut old_s, mut s) = (1i128, 0i128);
    while r != 0 {
        let q = old_r / r;
        let new_r = old_r - q * r;
        old_r = r;
        r = new_r;
        let new_s = old_s - q * s;
        old_s = s;
        s = new_s;
    }
    let mm = m as i128;
    ((old_s % mm + mm) % mm) as u64
}

/// toy ハッシュ (FNV-1a 風)。証明書の中身を1つの数値に潰す。本物は SHA-256。
fn toy_hash(data: &[u8]) -> u64 {
    let mut h: u64 = 14_695_981_039_346_656_037;
    for &byte in data {
        h ^= byte as u64;
        h = h.wrapping_mul(1_099_511_628_211);
    }
    h
}

// =============================================================================
// RSA 鍵ペア
// =============================================================================

/// RSA 鍵ペア。公開鍵 = (e, n)、秘密鍵 = d。
/// 署名する人 (CA など) だけが d を持つ。
#[derive(Clone, Copy)]
struct RsaKey {
    e: u64,
    d: u64,
    n: u64,
}

impl RsaKey {
    /// 公開鍵 (e, n) だけを取り出す (証明書に載せたり配布したりする部分)。
    fn public(&self) -> (u64, u64) {
        (self.e, self.n)
    }

    /// digest に秘密鍵で署名: signature = digest^d mod n。
    fn sign(&self, digest: u64) -> u64 {
        mod_pow(digest, self.d, self.n)
    }
}

/// 2つの素数から RSA 鍵ペアを作る。
/// φ(n) と互いに素な最小の公開指数 e を選び、その逆元 d を求める。
fn make_rsa(p: u64, q: u64) -> RsaKey {
    let n = p * q;
    let phi = (p - 1) * (q - 1);
    let mut e = 3u64;
    while gcd(e, phi) != 1 {
        e += 2;
    }
    let d = modinv(e, phi);
    RsaKey { e, d, n }
}

// =============================================================================
// 証明書
// =============================================================================

/// 証明書 = 「この名前 (subject) の公開鍵はこれですよ」という、発行者 (issuer) の保証書。
/// 本物の X.509 もこの項目を持つ (実際はもっと多い)。
#[derive(Clone)]
struct Certificate {
    subject: String,        // 持ち主の名前 (例: "toytls.local")
    issuer: String,         // 発行者 = 署名した人の名前 (例: "Intermediate CA")
    not_after: u64,         // 有効期限 (簡易: YYYYMMDD の数値)
    public_key: (u64, u64), // 持ち主の公開鍵 (e, n)
    signature: u64,         // issuer の秘密鍵による署名
}

/// 署名対象 (TBS: To Be Signed) のバイト列を作る。
/// signature 以外の全項目を連結したもの。1ビットでも変われば digest が変わる。
fn cert_tbs(subject: &str, issuer: &str, not_after: u64, public_key: (u64, u64)) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(subject.as_bytes());
    v.push(b'|');
    v.extend_from_slice(issuer.as_bytes());
    v.push(b'|');
    v.extend_from_slice(&not_after.to_be_bytes());
    v.extend_from_slice(&public_key.0.to_be_bytes());
    v.extend_from_slice(&public_key.1.to_be_bytes());
    v
}

/// 証明書を発行する = issuer が subject の情報に署名する。
fn issue_cert(
    subject: &str,
    issuer_name: &str,
    not_after: u64,
    subject_pubkey: (u64, u64),
    issuer_key: &RsaKey,
) -> Certificate {
    let tbs = cert_tbs(subject, issuer_name, not_after, subject_pubkey);
    let digest = toy_hash(&tbs) % issuer_key.n; // 署名する人の n 未満に収める
    let signature = issuer_key.sign(digest);
    Certificate {
        subject: subject.to_string(),
        issuer: issuer_name.to_string(),
        not_after,
        public_key: subject_pubkey,
        signature,
    }
}

/// 証明書の署名を「発行者の公開鍵」で検証する。
/// s^e mod n を計算し、証明書の中身から再計算した digest と一致するか見る。
fn verify_signature(cert: &Certificate, issuer_pubkey: (u64, u64)) -> bool {
    let (e, n) = issuer_pubkey;
    let tbs = cert_tbs(&cert.subject, &cert.issuer, cert.not_after, cert.public_key);
    let digest = toy_hash(&tbs) % n;
    mod_pow(cert.signature, e, n) == digest
}

fn print_cert(label: &str, c: &Certificate) {
    println!("  ── {label} ──");
    println!("    Subject (持ち主) : {}", c.subject);
    println!("    Issuer  (発行者) : {}", c.issuer);
    println!("    NotAfter (期限)  : {}", c.not_after);
    println!("    PublicKey        : (e={}, n={})", c.public_key.0, c.public_key.1);
    println!("    Signature        : {}", c.signature);
}

// =============================================================================
// チェーン検証 (クライアントの仕事)
// =============================================================================

/// 信頼の連鎖を検証する。
/// クライアントが信頼の起点として持つのは「ルートCAの名前と公開鍵」だけ (trust_anchor)。
/// サーバーから受け取るのは [サーバー証明書, 中間CA証明書] の2枚 (ルートは送られない)。
///
/// 検証手順:
///   1. 中間CA証明書を「ルートCAの公開鍵」で検証 (中間CAはルートが保証したか)
///   2. サーバー証明書を「中間CAの公開鍵」で検証 (サーバーは中間CAが保証したか)
///   3. 各証明書の有効期限を確認
///   4. サーバー証明書の名前がアクセス先ホスト名と一致するか確認
fn verify_chain(
    server: &Certificate,
    intermediate: &Certificate,
    trust_anchor: (&str, (u64, u64)),
    today: u64,
    hostname: &str,
) -> Result<(), String> {
    let (root_name, root_pubkey) = trust_anchor;

    // --- 1. 中間CA証明書の検証 ---
    if intermediate.issuer != root_name {
        return Err(format!(
            "中間CAの発行者 \"{}\" が、信頼するルート \"{}\" と一致しない",
            intermediate.issuer, root_name
        ));
    }
    if !verify_signature(intermediate, root_pubkey) {
        return Err("中間CA証明書の署名がルートCAの公開鍵で検証できない (偽の発行者)".to_string());
    }
    if intermediate.not_after < today {
        return Err(format!(
            "中間CA証明書が期限切れ (期限={}, 今日={today})",
            intermediate.not_after
        ));
    }

    // --- 2. サーバー証明書の検証 ---
    if server.issuer != intermediate.subject {
        return Err(format!(
            "サーバー証明書の発行者 \"{}\" が、中間CA \"{}\" と一致しない (チェーンが繋がらない)",
            server.issuer, intermediate.subject
        ));
    }
    if !verify_signature(server, intermediate.public_key) {
        return Err("サーバー証明書の署名が中間CAの公開鍵で検証できない (改ざんの疑い)".to_string());
    }
    if server.not_after < today {
        return Err(format!(
            "サーバー証明書が期限切れ (期限={}, 今日={today})",
            server.not_after
        ));
    }

    // --- 3. 名前 (ホスト名) の一致 ---
    if server.subject != hostname {
        return Err(format!(
            "証明書の名前 \"{}\" がアクセス先 \"{hostname}\" と一致しない",
            server.subject
        ));
    }

    Ok(())
}

/// 検証結果を見やすく出力する。
fn check(label: &str, result: Result<(), String>) {
    match result {
        Ok(()) => println!("[検証] {label}\n        → ✅ OK: このサーバーは信頼できる"),
        Err(e) => println!("[検証] {label}\n        → ❌ 失敗: {e}"),
    }
}

// =============================================================================
// メイン: 信頼の連鎖を組み立てて、いろいろな検証を試す
// =============================================================================

const TODAY: u64 = 20260609; // 今日 (YYYYMMDD)
const HOST: &str = "toytls.local";

fn main() {
    println!("=== net-05 認証局 (CA) と信頼の連鎖 デモ ===");
    println!("⚠️ 鍵は極小の toy。本番は OpenSSL / 本物の X.509 を使うこと。\n");

    // --- 各登場人物の鍵ペアを用意 ---
    let root_key = make_rsa(101, 113); // ルートCA (信頼の起点)
    let inter_key = make_rsa(107, 127); // 中間CA
    let server_key = make_rsa(131, 139); // サーバー

    // --- 信頼の連鎖を組み立てる (発行 = 署名) ---
    // ルートCA は自分で自分に署名する (自己署名証明書)
    let root_cert = issue_cert("Root CA", "Root CA", 20350101, root_key.public(), &root_key);
    // ルートCA が中間CA に署名
    let inter_cert = issue_cert(
        "Intermediate CA",
        "Root CA",
        20300101,
        inter_key.public(),
        &root_key,
    );
    // 中間CA がサーバーに署名
    let server_cert = issue_cert(
        HOST,
        "Intermediate CA",
        20270609,
        server_key.public(),
        &inter_key,
    );

    println!("--- 発行された証明書チェーン ---");
    print_cert("ルートCA証明書 (自己署名・信頼の起点)", &root_cert);
    print_cert("中間CA証明書 (ルートが署名)", &inter_cert);
    print_cert("サーバー証明書 (中間CAが署名)", &server_cert);

    // クライアントが最初から信頼しているのは「ルートCAの名前と公開鍵」だけ。
    // (= ブラウザ/OS に同梱されたルート証明書ストア)
    let trust_anchor = ("Root CA", root_key.public());
    println!("\nクライアントの信頼の起点 (トラストアンカー):");
    println!("  Root CA の公開鍵 (e={}, n={}) のみ", root_key.e, root_key.n);
    println!("  ※ 中間CA・サーバー証明書はサーバーから受け取って検証する\n");

    // === シナリオA: 正規のチェーン ===
    println!("--- シナリオA: 正規のチェーン ---");
    check(
        "正規のサーバー証明書 + 中間CA証明書",
        verify_chain(&server_cert, &inter_cert, trust_anchor, TODAY, HOST),
    );

    // === シナリオB: 改ざん (攻撃者が有効期限を勝手に延長) ===
    println!("\n--- シナリオB: 証明書の改ざん (期限を勝手に延長) ---");
    let mut tampered = server_cert.clone();
    tampered.not_after = 20990101; // 署名はそのままに中身だけ書き換える
    check(
        "期限を 2099 に書き換えたサーバー証明書",
        verify_chain(&tampered, &inter_cert, trust_anchor, TODAY, HOST),
    );
    println!("        (中身を変えると digest が変わり、署名と合わなくなる)");

    // === シナリオC: 期限切れ ===
    println!("\n--- シナリオC: 期限切れの証明書 ---");
    let expired_cert = issue_cert(HOST, "Intermediate CA", 20250101, server_key.public(), &inter_key);
    check(
        "期限が 2025-01-01 (今日より前) のサーバー証明書",
        verify_chain(&expired_cert, &inter_cert, trust_anchor, TODAY, HOST),
    );

    // === シナリオD: 攻撃者が自前のCAで偽証明書を発行 (名前は Root CA を騙る) ===
    println!("\n--- シナリオD: 偽CA (名前だけ \"Root CA\" を騙る中間者) ---");
    let evil_root_key = make_rsa(149, 151); // 攻撃者のルート鍵
    let evil_inter_key = make_rsa(157, 163);
    let evil_server_key = make_rsa(167, 173);
    // 攻撃者は自分の鍵で、名前だけ "Root CA"/"Intermediate CA" を騙って署名する
    let evil_inter = issue_cert(
        "Intermediate CA",
        "Root CA",
        20300101,
        evil_inter_key.public(),
        &evil_root_key,
    );
    let evil_server = issue_cert(HOST, "Intermediate CA", 20270609, evil_server_key.public(), &evil_inter_key);
    check(
        "攻撃者が自前の鍵で発行した偽チェーン",
        verify_chain(&evil_server, &evil_inter, trust_anchor, TODAY, HOST),
    );
    println!("        (名前は一致しても、本物のルート鍵で署名されていないので検証に失敗する)");
    println!("        → 「信頼は名前ではなく『鍵による署名』で成り立つ」ことがわかる");

    println!("\n=== デモ終了 ===");
}

// =============================================================================
// 演習
// =============================================================================

// --- 演習1: 基礎 ---
// 証明書に「有効期間の開始日 (not_before)」を足そう。
//
// 現状は not_after (期限) だけで「期限切れ」しか検出できない。
// not_before を足して、まだ有効になっていない証明書 (today < not_before) も弾く。
//   1. Certificate に not_before: u64 を追加
//   2. cert_tbs にも not_before を含める (署名対象に入れる = 改ざん検知の対象にする)
//   3. verify_chain に today < not_before のチェックを足す
// ヒント: not_before を tbs に入れ忘れると、攻撃者に開始日を書き換えられてしまう。

// --- 演習2: 応用 ---
// 「信頼ストア」を複数のルートCAに対応させよう。
//
// 現実のブラウザは1つではなく何十ものルートCAを信頼している。
//   1. trust_anchor を Vec<(String, (u64,u64))> (ルート名→公開鍵 のリスト) にする
//   2. verify_chain は、中間CAの issuer 名に一致するルートを探し、その鍵で検証する
//   3. リストに無いルートで署名された証明書は「未知のルート」で弾く (= ブラウザの警告)
// 学べること: 自己署名証明書がなぜ「信頼されていない」と警告されるのか。

// --- 演習3: チャレンジ ---
// 証明書の「失効 (revocation)」を実装しよう。
//
// 秘密鍵が漏れた等の理由で、期限内でも証明書を無効化したいことがある。
//   1. 失効リスト (CRL): 失効した証明書のシリアル番号の集合を用意する
//      (Certificate に serial: u64 を足す)
//   2. verify_chain で、チェーン上の各証明書が CRL に載っていないか確認する
//   3. 載っていれば「失効済み」で弾く
// 発展: net-04 の TLS ハンドシェイクと接続し、サーバーが送ってきた証明書チェーンを
//       実際に verify_chain で検証してから鍵交換に進む、という流れにしてみる。
