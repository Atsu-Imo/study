# net-03: DNS リゾルバ & サーバー

## 概要

ネットワークの「登場人物」第3弾は **DNS** (Domain Name System)。

ブラウザで `https://example.com` と入力したとき **最初に起きるのが DNS 問い合わせ**。`example.com` という人間が読める名前を、コンピュータが理解できる IP アドレス `93.184.216.34` に変換するための仕組み。

このプロジェクトでは:
- 本物の Google Public DNS (`8.8.8.8`) と話す**リゾルバ**
- 自前のドメイン名で応答する**簡易DNSサーバー**

の両方を実装する。

## net-01/02 との大きな違い

| | net-01 (TCPエコー) | net-02 (HTTP) | **net-03 (DNS)** |
|---|---|---|---|
| プロトコル形式 | テキスト | テキスト | **バイナリ** |
| トランスポート | TCP | TCP | **UDP** |
| 接続概念 | 必須 (ハンドシェイク) | 必須 | **なし** (1往復で完結) |
| ポート | 任意 | 80/443 | **53** |
| メッセージ単位 | バイトストリーム | バイトストリーム | **データグラム** |

ここで初めて「バイナリプロトコル」「UDP」が登場する。HTTP と DNS は同じネットワーク層 (IP) を共有するが、上位プロトコルとしては全くの別物。

## DNS とは

### 役割: 名前 ↔ IP の変換

```
ブラウザ「example.com の IP を教えて」
   ↓
DNSサーバー「93.184.216.34 だよ」
   ↓
ブラウザ「ありがとう、その IP に TCP 接続するよ」
```

電話帳と同じ発想。人間は「example.com」のような名前を覚え、コンピュータは IP アドレスを使い、その間をつなぐのが DNS。

### なぜ UDP？

DNS は「**1往復だけの軽い問い合わせ**」が典型。TCP の3ウェイハンドシェイクは無駄。

- UDP: 接続なし、1パケット送って1パケット受け取って終了 → 高速
- TCP: 接続確立に SYN→SYN-ACK→ACK の3往復 + データ往復 → 遅い

ただし応答が 512 バイトを超える (大量のレコード、DNSSEC など) と DNS over TCP に切り替わるルールがある。

### 階層構造 (DNS の本当の姿)

DNS は単一の巨大データベースではなく、**ツリー状に分散管理**された階層構造。

```
                 .  (ルート)
                 │
    ┌────────────┼────────────┐
    │            │            │
   .com         .org         .jp
    │            │
example       rust-lang
    │            │
   www         www
```

実際のフル解決はこう動く (反復問い合わせ):

1. ローカルリゾルバ → **ルートサーバー**「`www.example.com` は？」
2. ルート → 「`.com` の権威サーバーはここ」
3. ローカルリゾルバ → `.com` 権威サーバー「`www.example.com` は？」
4. `.com` → 「`example.com` の権威サーバーはここ」
5. ローカルリゾルバ → `example.com` 権威サーバー「`www.example.com` は？」
6. `example.com` → 「IP は X.X.X.X」

普段はキャッシュが効くので毎回これ全部を辿るわけではない。今回作るのは「自分のゾーン (`example.local`) の権威サーバー」相当。

### 再帰 vs 反復

| 種類 | 説明 | 例 |
|---|---|---|
| **再帰問い合わせ** | 「答えを丸ごと教えて」と頼む | ブラウザ → ローカルDNS (8.8.8.8など) |
| **反復問い合わせ** | 「知ってる範囲だけ教えて」と頼む | ローカルDNS → ルート/権威サーバー |

今回作るリゾルバは **RD=1 (再帰要求) をセット**して 8.8.8.8 に丸投げするタイプ。演習3で反復問い合わせの実装に挑戦する。

## DNSメッセージのバイナリフォーマット

ここが net-03 の核心。**HTTP は「テキスト」だったが、DNS は「ビット列」**。

### 全体構造

```
┌──────────────┐  12バイト
│   Header     │  ID, Flags, Counts
├──────────────┤
│  Question    │  問い合わせ内容
├──────────────┤
│   Answer     │  回答
├──────────────┤
│  Authority   │  権威ネームサーバー情報
├──────────────┤
│  Additional  │  追加情報 (NS の IP など)
└──────────────┘
```

### ヘッダー (12バイト = 16bit × 6行)

RFC 1035 の表記。上段が10の位、下段が1の位で、合体させてビット位置を読む (例: 上段`1` 下段`0` → ビット10)。1行 = 16ビット = 2バイト。

```
                                 1  1  1  1  1  1
   0  1  2  3  4  5  6  7  8  9  0  1  2  3  4  5
 +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
 |                      ID                       |
 +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
 |QR|   Opcode  |AA|TC|RD|RA|   Z    |   RCODE   |
 +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
 |                    QDCOUNT                    |
 +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
 |                    ANCOUNT                    |
 +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
 |                    NSCOUNT                    |
 +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
 |                    ARCOUNT                    |
 +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
```

| フィールド | サイズ | 意味 |
|---|---|---|
| ID | 16bit | リクエストと応答を紐付ける識別子 |
| QR | 1bit | 0=query, 1=response |
| Opcode | 4bit | 0=標準クエリ |
| AA | 1bit | Authoritative Answer (権威ある回答か) |
| TC | 1bit | Truncated (応答が切れた→TCPで再問い合わせ) |
| RD | 1bit | Recursion Desired (再帰問い合わせを要求) |
| RA | 1bit | Recursion Available (再帰問い合わせに対応) |
| RCODE | 4bit | 結果コード (0=OK, 3=NXDOMAIN, …) |
| QDCOUNT〜ARCOUNT | 各16bit | 各セクションの件数 |

すべて**ビッグエンディアン** (ネットワークバイトオーダー)。Rust では `to_be_bytes()` / `from_be_bytes()` で扱う。

### Question セクション

```
┌──────────────────────┐
│  QNAME  (可変長)      │  ドメイン名 (ラベル形式)
├──────────────────────┤
│  QTYPE  (2バイト)     │  1=A, 28=AAAA, 5=CNAME, ...
├──────────────────────┤
│  QCLASS (2バイト)     │  1=IN (Internet)
└──────────────────────┘
```

### ドメイン名のラベル形式

`www.example.com` は **このバイト列**になる:

```
03 'w' 'w' 'w' 07 'e' 'x' 'a' 'm' 'p' 'l' 'e' 03 'c' 'o' 'm' 00
↑               ↑                            ↑               ↑
長さ3バイト      長さ7バイト                   長さ3バイト       終端
```

各ラベルは「長さ1バイト + 中身」の連続で、最後に長さ0の終端バイト。Rust 実装はこう:

```rust
fn encode_name(buf: &mut Vec<u8>, name: &str) {
    for label in name.split('.') {
        if label.is_empty() { continue; }
        buf.push(label.len() as u8);
        buf.extend_from_slice(label.as_bytes());
    }
    buf.push(0); // 終端
}
```

### 名前圧縮ポインタ

応答メッセージでは同じドメイン名が繰り返し出てくる (Question と Answer の両方など)。これを毎回フルに書くと無駄なので、**「メッセージ先頭からのオフセット」を指すポインタ**で参照できる:

```
通常ラベル:    [長さ][ラベル][長さ][ラベル][0]
ポインタ:     [11xxxxxx][xxxxxxxx]      ← 上位2ビットが 11 ならポインタ
                  └─── 14ビットのオフセット ──┘
```

たとえば `0xC0 0x0C` は「メッセージの 12 バイト目から始まる名前を見ろ」という意味。`0xC0 = 0b11000000`、`0x0C = 12`。

ポインタを追従するパースが少しややこしいが、なしには応答が成立しない (今回のサーバー実装でも使っている)。

### リソースレコード (Answer/Authority/Additional)

```
┌──────────────────────┐
│  NAME                 │  ドメイン名 (圧縮ポインタが多い)
├──────────────────────┤
│  TYPE   (2バイト)     │  レコード種別
├──────────────────────┤
│  CLASS  (2バイト)     │  1=IN
├──────────────────────┤
│  TTL    (4バイト)     │  キャッシュ寿命 (秒)
├──────────────────────┤
│  RDLENGTH (2バイト)   │  RDATA のバイト数
├──────────────────────┤
│  RDATA  (RDLENGTH)    │  実データ (種別によって解釈が変わる)
└──────────────────────┘
```

RDATA の解釈は TYPE 次第:

| TYPE | 種別 | RDATA の中身 |
|---|---|---|
| 1 (A) | IPv4 アドレス | 4バイト |
| 28 (AAAA) | IPv6 アドレス | 16バイト |
| 5 (CNAME) | 別名 | ドメイン名 (ラベル形式、ポインタ可) |
| 2 (NS) | ネームサーバー | ドメイン名 |
| 15 (MX) | メール交換 | 2バイト優先度 + ドメイン名 |

## Goとの比較

Go で DNS をやるなら標準ライブラリの `net.Resolver` を使う:

```go
ips, err := net.LookupIP("example.com")
for _, ip := range ips { fmt.Println(ip) }
```

これで終わり。バイト列パースはランタイムが面倒見てくれる。

Rust でも `hickory-dns` (旧 trust-dns) や `tokio` 系のクレートを使えば同じレベルの抽象化はある。今回はあえて `std::net::UdpSocket` から手書きで実装する — DNS の「中身」を学ぶため。

| 概念 | Go | Rust (今回) |
|---|---|---|
| 名前解決 | `net.LookupIP("example.com")` | `build_query` + `UdpSocket::send_to` |
| UDP ソケット | `net.ListenPacket("udp", ...)` | `UdpSocket::bind(...)` |
| バイト列の big-endian 変換 | `encoding/binary.BigEndian.PutUint16` | `u16::to_be_bytes()` |
| エラー | `net.DNSError` | `std::io::Error` |

## コード解説

### クエリの組み立て

```rust
fn build_query(id: u16, qname: &str, qtype: u16) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);
    // ヘッダー 12バイト
    buf.extend_from_slice(&id.to_be_bytes());
    buf.extend_from_slice(&0x0100u16.to_be_bytes()); // RD=1
    buf.extend_from_slice(&1u16.to_be_bytes());      // QDCOUNT=1
    buf.extend_from_slice(&0u16.to_be_bytes());      // ANCOUNT=0
    buf.extend_from_slice(&0u16.to_be_bytes());      // NSCOUNT=0
    buf.extend_from_slice(&0u16.to_be_bytes());      // ARCOUNT=0
    // Question
    encode_name(&mut buf, qname);
    buf.extend_from_slice(&qtype.to_be_bytes());
    buf.extend_from_slice(&CLASS_IN.to_be_bytes());
    buf
}
```

ビット列を順に積み上げるだけ。`to_be_bytes()` で big-endian バイト列に変換しているのがポイント。

### 名前パース (圧縮ポインタ対応)

```rust
fn parse_name(buf: &[u8], start: usize) -> (String, usize) {
    let mut labels = Vec::new();
    let mut pos = start;
    let mut jumped = false;
    let mut original_end = start;

    loop {
        let b = buf[pos];
        if b == 0 { /* 終端 */ pos += 1; if !jumped { original_end = pos; } break; }
        if b & 0xC0 == 0xC0 {
            // ポインタを追従
            let offset = (((b & 0x3F) as usize) << 8) | (buf[pos + 1] as usize);
            if !jumped { original_end = pos + 2; jumped = true; }
            pos = offset;
            continue;
        }
        // 通常ラベル
        let len = b as usize;
        pos += 1;
        labels.push(String::from_utf8_lossy(&buf[pos..pos + len]).into_owned());
        pos += len;
    }
    (labels.join("."), original_end - start)
}
```

ポイント:
- `b & 0xC0 == 0xC0` で「上位2ビットが11か」を判定 → ポインタ
- ポインタを追ったあとは元の位置に戻れない (情報がない) ので、最初にポインタを踏んだ位置 `original_end` を覚えておく
- `consumed = original_end - start` で「呼び出し側が次に進むべきバイト数」を返す

### UDP 通信

```rust
let socket = UdpSocket::bind("0.0.0.0:0")?;       // OS が空きポートを割り当て
socket.set_read_timeout(Some(Duration::from_secs(3)))?;
socket.send_to(&query, server)?;                   // 送る
let (n, _) = socket.recv_from(&mut buf)?;          // 受け取る (ブロック)
```

TCP と違って `connect` も `accept` もない。`bind` してすぐ `send_to`/`recv_from` できる。データグラム1個 = メッセージ1個なので、HTTPみたいに「改行で区切る」とかの処理は不要。

## 動作確認

### デモモード

```bash
cd net-03-dns
cargo run
```

実行すると:
1. **Phase 1**: 本物の `8.8.8.8:53` に `example.com` と `www.rust-lang.org` を問い合わせ
2. **Phase 2**: ローカルサーバーを起動して `example.local` 系の問い合わせ

`www.rust-lang.org` を問い合わせると、CNAME チェーン (`www.rust-lang.org` → `rust-lang.github.io`) が観察できる。

### 個別実行

```bash
# 自前サーバーだけ起動 (UDP 5300)
cargo run -- server

# 本物のDNSに問い合わせ
cargo run -- resolver example.com 8.8.8.8:53
cargo run -- resolver github.com 1.1.1.1:53

# 自前サーバーに問い合わせ
cargo run -- resolver example.local 127.0.0.1:5300

# dig コマンドからも当然OK
dig @127.0.0.1 -p 5300 example.local
dig @127.0.0.1 -p 5300 api.example.local
```

### tcpdump で実物を観察

```bash
sudo tcpdump -i lo -nn -X port 5300
```

別ターミナルで `dig @127.0.0.1 -p 5300 example.local` を打つと、生のバイナリ DNS メッセージが流れる様子が見られる。

## 実装上の注意

### ポート 53 は root が必要

Linux の特権ポート (0〜1023) は root でないと bind できない。今回は `5300` を使うことで sudo を回避している。本物の DNS サーバー (BIND, dnsmasq など) は root で起動するか setcap で許可を与える。

### UDP の制限: 512バイト

DNS の元々の仕様では **応答が 512 バイトを超えるとフラグ TC=1 で「切れた」**ことを示し、クライアントは TCP で再問い合わせするべきとされている。EDNS で拡張 (4096 など) もあるが、今回は単純化のため考慮していない。

### NXDOMAIN の扱い

辞書に無いドメインは RCODE=3 (NXDOMAIN) を返す。これによりリゾルバ側で「名前が存在しない」と判定できる。HTTPの404に似た位置づけ。

### 同名ヘッダーならぬ同名レコード

`api.example.local` には 2つの IP を登録している (`10.0.0.1`, `10.0.0.2`)。これは1つの応答に Answer を複数並べることで実現する。ラウンドロビン DNS の仕組みでもある。

## 演習

### 演習1: 基礎 — AAAA レコード対応

サーバーが IPv6 アドレス (AAAA レコード) も返せるようにする。

ヒント:
- `records` を `HashMap<String, Vec<Ipv4Addr>>` から、IPv4/IPv6 両方持てる構造に変える
- `handle_query` で `qtype == TYPE_AAAA` (= 28) のときに 16バイトの RDATA を返す
- 動作確認: `dig @127.0.0.1 -p 5300 ipv6.example.local AAAA`

### 演習2: 応用 — CNAME と圧縮ポインタの送信

CNAME レコード (別名) のサポートを追加する。

```
records = {
    "blog.example.local" → CNAME → "www.example.local"
    "www.example.local"  → A     → 192.168.1.100
}
```

このとき `dig blog.example.local` への応答には:
1. CNAME レコード (`blog.example.local` → `www.example.local`)
2. A レコード (`www.example.local` → `192.168.1.100`)

の2つの Answer が含まれるべき。RDATA のドメイン名部分に圧縮ポインタを使えるとなお良い。

### 演習3: チャレンジ — 反復問い合わせの実装

リゾルバを「ルートから順に辿る」反復問い合わせに改造する。

1. ルートサーバー (`a.root-servers.net = 198.41.0.4`) に `RD=0` で問い合わせ
2. レスポンスの Authority/Additional セクションをパースして、次に問い合わせるべきネームサーバーを抽出
3. そのネームサーバーに同じクエリを送る
4. 最終的に A レコードが得られるまで繰り返す

学べること:
- Authority / Additional セクションのパース
- 再帰なしのクエリ (RD=0)
- ネームサーバー追跡ロジック

実装すれば「`example.com` を解決するために、ルート → `.com` → `example.com` の3段階を辿った」というログが出るようになる。

## まとめ

- DNS は **バイナリプロトコル + UDP + ポート53**。HTTPと全く別物
- メッセージ構造: Header + Question + Answer + Authority + Additional
- ドメイン名は **ラベル形式** ([長さ][中身]...[0])、レスポンスでは **圧縮ポインタ** で省略
- 整数はすべて **ビッグエンディアン** (`to_be_bytes` / `from_be_bytes`)
- 再帰問い合わせ (RD=1) と反復問い合わせ (RD=0) の使い分け
- `8.8.8.8` のような **パブリックリゾルバ**は再帰問い合わせを受けて答えを全部解決してくれる便利屋
- 権威ネームサーバーは「自分のゾーン」だけ知っている (今回作ったのはこれに近い)
