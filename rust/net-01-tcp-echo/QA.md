# net-01: TCPエコーサーバー Q&A

日付: 2026-03-23

## Q1: TcpStreamにはHTTPのようなヘッダーやボディはないのか？

TCPにはヘッダー・ボディの概念はない。ヘッダーやボディはHTTPが定義したもの。

TCPはバイト列のストリームであり、送受信されるデータの形式には関与しない。

```
HTTP  →  ヘッダー、ボディ、ステータスコード等を定義（アプリケーション層プロトコル）
TCP   →  バイト列を順序保証付きで転送（トランスポート層）
IP    →  パケットを宛先に配送（ネットワーク層）
```

`write_all(b"Hello\n")` で送信されるのは `48 65 6C 6C 6F 0A` というバイト列のみ。HTTPのような構造化されたフォーマットは、上位プロトコルがTCP上に独自に定義するもの。

---

日付: 2026-05-21

## Q2: `listener.incoming()` はどのように接続を待っているのか？なぜ wait されて、接続が来たら動くのか

`incoming()` は内部で `accept()` を繰り返し呼ぶだけのイテレータ。「待つ」処理の本体は `accept()` システムコールにある。

### レイヤー構造

```
あなたのコード   for stream in listener.incoming() { ... }
       ↓
std::net        Incoming イテレータ → next() で accept() を呼ぶ
       ↓
libc            accept(fd, ...) システムコール
       ↓
OSカーネル      ソケットの accept キューから1つ取り出す。空ならスレッドを sleep
       ↓
NIC + ドライバ  パケットを受信、TCPハンドシェイクを処理
```

### `incoming()` の中身

std の実装は以下と等価:

```rust
loop {
    let (stream, _addr) = listener.accept()?;
    // ...
}
```

### `accept()` システムコールの動作

1. カーネル内に「accept キュー」(別名 backlog) がある
2. キューに確立済み接続があれば即座に返す
3. 空なら**呼び出したスレッドをスリープ状態にする** (CPU は使わない)
4. 新しい接続が来た瞬間、カーネルがスレッドを起こす

ビジーループではなく、OS スケジューラがそのスレッドを実行対象から外す (TASK_INTERRUPTIBLE 状態)。

### 接続がキューに入るまで

`bind()` + `listen()` でカーネルは2つのキューを用意:

| キュー | 状態 |
|---|---|
| SYN キュー | SYN-ACK 送信済み、ACK 待ち (ハーフオープン) |
| accept キュー | 3ウェイハンドシェイク完了済み |

ハンドシェイクはアプリが何もしなくてもカーネルが完結させる。`accept()` を呼んだ時には既に確立済みの接続が手に入る。

## Q3: 同時に接続がいっぱい来たら破綻する？

破綻するシナリオが何段階かある。

### ボトルネック1: accept キューが溢れる

カーネルの accept キューには上限がある:

- アプリ側: `listen(fd, backlog)` の `backlog` 引数
- システム側: `/proc/sys/net/core/somaxconn` (Linux のデフォルトは 4096)

満杯のときに来た SYN はカーネルが黙って捨てる。クライアント側からは接続タイムアウトや Connection refused に見える。

```bash
$ netstat -s | grep -i "listen"
    1234 SYNs to LISTEN sockets dropped
```

### ボトルネック2: accept ループが1スレッドで回っている

`for stream in listener.incoming()` は1スレッド。`accept()` 自体は速いが、`thread::spawn` のコストで詰まる。

| 処理 | 概算コスト |
|---|---|
| `accept()` システムコール | 数マイクロ秒 |
| `thread::spawn` (スレッド生成) | 数十〜数百マイクロ秒 + 2〜8MB のスタック |

### ボトルネック3: スレッド数自体の上限

1接続=1スレッドモデルの致命的な弱点。10万接続=10万スレッド=200GB のメモリ。完全に破綻する。**C10K 問題**。

| 制限 | 値 |
|---|---|
| 1スレッドのスタック | デフォルト 2〜8 MB |
| `/proc/sys/kernel/threads-max` | 数万〜数十万 |
| `ulimit -n` | プロセスあたりの FD 数 |

### 対処法

1. **スレッドプール** — 固定数のワーカーで使い回す。緩和策
2. **非同期 I/O (tokio)** — `epoll` で1スレッドで多重化。**本命** (14章で扱う)
3. **`SO_REUSEPORT`** — ポートを複数プロセスで同時 listen して accept 並列化 (nginx 採用)

### 数値感覚

| モデル | 同時接続の現実的上限 |
|---|---|
| 1接続=1スレッド (std::net) | 数千接続 |
| スレッドプール | プール数 × 並行度 |
| tokio (非同期) | 数十万接続 |
| tokio + SO_REUSEPORT + 複数プロセス | 数百万接続 (C10M問題) |

net-01 は学習用に最もシンプルな構成で、性能改善は後の章で行う。

## Q4: accept はカーネル側に処理を任せるという理解で合っているか？なぜカーネル側が接続を検知して Rust 側とやりとりできるのか。Python でも同じか？

その理解で正しい。Rust ・Python ・Go ・C の違いはほぼ関係ない。

### 大前提: アプリは直接ネットワークに触れない

| モード | 別名 | 何ができるか |
|---|---|---|
| カーネルモード (Ring 0) | 特権モード | ハードウェア直接操作、割り込み処理 |
| ユーザーモード (Ring 3) | 一般モード | 計算とメモリアクセスのみ |

Rust/Python/Go/C のアプリは全部ユーザーモードで動く。NIC のパケットを見るにはカーネルにお願いするしかない。

### Rust → カーネル の経路

```
1. for stream in listener.incoming() ...    ← Rust コード
2. Incoming::next() を呼ぶ                  ← std::net 内
3. TcpListener::accept() を呼ぶ             ← std::net 内
4. libc::accept(fd, ...) を呼ぶ             ← OS の C ライブラリ
5. syscall 命令を実行                       ← CPU命令
6. CPU がカーネルモードに切り替わる         ← ★ここで境界を越える
7. カーネルの sys_accept() 関数が実行される ← カーネル空間に入る
```

ステップ6 で「Rust が走っていた CPU コアの実行モードがカーネルに切り替わる」。同じスレッドがユーザー空間からカーネル空間へ侵入するイメージ。

### NIC からカーネルへ — 接続を検知する仕組み

Rust が待っているのではなく、NIC とカーネルだけで完結する流れがある:

```
1. クライアントが SYN パケットを送る
2. ネットワーク経由でサーバーの NIC に到達
3. NIC が CPU に ★ ハードウェア割り込み を出す
4. CPU は今やっている処理を中断、カーネルの割り込みハンドラへジャンプ
5. カーネルがパケットを解釈、TCP/IP スタックで処理
6. SYN なら SYN-ACK を NIC 経由で返す
7. クライアントから ACK が届く
8. 3ウェイハンドシェイク完了 → accept キューに登録
9. ソケットで寝ているスレッドがあれば起こす
10. 起こされたスレッドが accept() から戻る → Rust コードに戻る
```

Rust プロセスは何もしていない (寝ている)。CPU 割り込みでカーネルが起動。Rust はカーネルから「来たよ」とプッシュ通知される。

### 「接続を待っているのは誰か?」

| 主語 | 何をしている |
|---|---|
| Rust スレッド | カーネルに `accept()` を頼んで寝ている。何も待っていない |
| カーネル | 割り込みを待ちつつ、他のプロセスを実行中 |
| NIC | パケットが来たら CPU に割り込みを発火 |

### Python だったら違うのか?

ほぼ完全に同じ動作になる。

```python
import socket
s = socket.socket()
s.bind(('127.0.0.1', 7878))
s.listen()
conn, addr = s.accept()   # ← ここでスレッドが寝る
```

Python の `socket.accept()` は CPython 実装の中で `libc::accept()` を呼ぶだけ。Rust と全く同じシステムコールに到達する。カーネル内では Rust か Python かの区別はそもそもつかない (FD と PID しか見ていない)。

### 言語ごとの本当の違い

ネットワーク機能そのものは同じカーネルを使うので、差は次の点に出る:

| 観点 | Rust (std) | Python | Go | Node.js |
|---|---|---|---|---|
| 使う syscall | accept | accept | accept (epoll経由) | accept (epoll経由) |
| ブロック中の CPU 使用 | 0% | 0% | 0% | 0% |
| 並行モデル | 1接続=1スレッド | 1接続=1スレッド (またはasyncio) | ゴルーチン (M:N) | イベントループ |
| 言語オーバーヘッド | 極小 | 大 (インタプリタ) | 中 | 中 |

ネットワーク待ち時間に差は出ない (どの言語でも結局カーネルが待つ)。差が出るのは接続後の処理速度・同時接続数のスケール・メモリ効率。

### 要点

- アプリは NIC を直接触れない。全部カーネルに頼む
- `accept()` は「カーネルに接続をくれと頼んで寝る」関数
- 接続検知は NIC からの割り込みでカーネルが起動 → ハンドシェイク完結 → 寝ているスレッドを起こす
- Python でも C でも Java でも、最終的に同じ `accept()` syscall に到達するので動作原理は同じ
- 言語の差はネットワーク待ち時間ではなく、処理速度や並行モデルに出る

## Q5: NIC ってなに?

**NIC = Network Interface Card (またはController)** — コンピュータをネットワークに物理的につなぐハードウェア。

### 具体的に何か

| 形態 | 例 |
|---|---|
| 有線 LAN | 本体に内蔵されている RJ-45 ポート (LANケーブルを挿す穴) |
| 無線 LAN | Wi-Fi モジュール (内蔵チップやUSBドングル) |
| サーバー向け | PCIe スロットに挿す拡張カード (10GbE / 100GbE など) |
| 仮想 | クラウドVMやコンテナ内の仮想NIC (eth0など) |

「ネットワークカード」「LANカード」「LANアダプタ」「イーサネットコントローラ」と呼ばれることもある。

### 役割

1. ケーブル/電波の電気信号 ↔ デジタルデータ (ビット列) の変換
2. パケットの受信・送信
3. MAC アドレスを持つ (ハードウェアごとに固有の識別子)
4. **受信したパケットを CPU に割り込みで通知する**

### Linux で確認

```bash
$ ip link
1: lo: <LOOPBACK,UP,LOWER_UP>            ← ループバック (自分宛て、これは仮想)
2: eth0: <BROADCAST,MULTICAST,UP>        ← 物理NIC その1
3: wlan0: <BROADCAST,MULTICAST,UP>       ← Wi-Fi NIC
```

### net-01 文脈での位置づけ

`127.0.0.1:7878` を使うとき、実は**物理 NIC を経由していない**。「ループバックインターフェース (`lo`)」という仮想 NIC をカーネルが内部で扱っているだけ。それでも仕組み (割り込み・accept キュー・wait queue) は本物の NIC と同じ流れで動く。

外部のクライアントから `192.168.x.x` などで接続する場合に物理 NIC が登場する。

## Q6: 「CPU が処理を中断、カーネルの割り込みハンドラへジャンプ」って、CPU がマルチスレッドだとどうなる?

質問の意図を「マルチコア CPU で割り込みが来たらどうなるか」と解釈して説明する。

### 用語整理

| 用語 | 意味 |
|---|---|
| マルチコア | CPU に物理コアが複数 (例: 8コア) |
| ハイパースレッディング (SMT) | 1物理コアを2論理コアに見せる Intel/AMD の技術 |
| マルチスレッド | OS が複数スレッドをスケジューリングする (ソフトウェアの話) |

### 結論: 割り込みは1つのコアだけが処理する

NIC からの1つの割り込みは、**1つのコアだけが処理する**。他のコアは何も知らずに自分の仕事を続ける。

```
NIC からの割り込み
       ↓
APIC (Advanced Programmable Interrupt Controller)
       ↓ ルーティング
   ┌───┴───┐
   ▼       ▼       ▼       ▼
 Core0   Core1   Core2   Core3   ← どれか1つに配送
 (中断)  (Rust)  (Python) (idle)
```

### APIC — 割り込みの交通整理係

x86 CPU には APIC という割り込みコントローラが各コアに付いていて、マザーボード上に IO-APIC という親玉がある。

1. NIC が IO-APIC に「割り込み IRQ #45」と通知
2. IO-APIC がどのコアに振るか判断 → 特定コアの Local APIC へ
3. そのコアの Local APIC が、命令の切れ目で CPU を中断
4. CPU が割り込みハンドラへジャンプ
5. ハンドラ終了後、元の処理に戻る

「中断」はマイクロ秒単位なので、ユーザーから見れば気づかない。

### IRQ affinity — どのコアで処理するかの設定

Linux では `/proc/irq/<番号>/smp_affinity` でマスクを設定可能。

```bash
$ cat /proc/interrupts
           CPU0       CPU1       CPU2       CPU3
 45:    1234567        890         12          0   IR-PCI-MSI   eth0
```

デフォルトは `irqbalance` デーモンが自動分散。高負荷サーバーでは手動固定するチューニングもある。

### 高速 NIC は RSS で並列化

10GbE/100GbE では1コアでは捌けないので **RSS (Receive Side Scaling)** で:

```
NIC 内に複数の受信キュー (例: 8キュー)
   ↓
パケットのハッシュ (送信元IP+ポート) でキュー振り分け
   ↓
各キューが別 IRQ を発行
   ↓
それぞれ別コアで並列処理
```

### 起こされるスレッドはどのコアで再開するか

OS スケジューラが決める:

1. カーネルが accept キューに接続を入れる
2. 「このソケットの wait queue にいるスレッドを runnable に戻せ」
3. スレッドが runnable リストに戻る
4. スケジューラが空いてるコアに乗せる

「割り込みを処理したコア」と「起こされたスレッドを実行するコア」は別でも構わない。CPU キャッシュ局所性のため「同じコアで起こす」と判断することは多いが必須ではない。

### タイムラインの例

```
t0: Core0: Rust(accept待ち)  Core1: Rust  Core2: Python  Core3: idle
t1: NIC が IRQ 発火 → IO-APIC が Core3 にルーティング
    Core0: Rust(accept待ち)  Core1: Rust  Core2: Python  Core3: ★割り込みハンドラ
t2: Core3 でカーネルが TCP ハンドシェイク処理
t3: ハンドシェイク完了、accept キューに登録、スレッドを runnable に
t4: スケジューラが Rust スレッドを起こす(Core0かどこかで)
    Core0: ★Rust(accept戻る)  Core1: Rust  Core2: Python  Core3: 他
```

### ユーザー目線での重要なこと

- マルチコアでも「割り込みの仕組み自体」は変わらない (1コアが対応)
- 並列度を上げるには `tokio` 等で論理タスクを増やす + IRQ を分散
- 「Rust スレッドが寝ている = そのコアが止まる」ではない。コアは他の仕事に使われる
- 寝ているスレッドはメモリ使用量だけ消費、CPU には影響しない

### 要点

- 割り込みは1コアだけが処理する。他コアは作業継続
- ルーティングは APIC というハードウェアが担当
- 高速 NIC は RSS で IRQ を複数キューに分け、複数コアで並列処理
- 「割り込み処理コア」と「起こされたスレッドのコア」は別でよく、OS スケジューラが決める
- 寝ているスレッドが乗っているコアは止まらない。OS が別スレッドを乗せて使う

## Q7: プロキシサーバーや DNS サーバーも同じように NIC→カーネル→サービス で処理しているのか?

**全部同じ仕組み**。サーバーの「役割」に関係なく、ネットワーク I/O は必ず NIC → カーネル → アプリの経路を通る。

| サーバー種別 | 使うsyscall | カーネルの仕組み |
|---|---|---|
| HTTP/TCP サーバー (Apache, nginx) | `accept()`, `read()`, `write()` | accept キュー、wait queue |
| DNS サーバー (BIND, CoreDNS) | `recv()`, `send()` | ソケットバッファ |
| プロキシ (nginx, HAProxy, Squid) | 上記の組み合わせ | 両方 |
| メールサーバー (Postfix) | `accept()`, `read()`, `write()` | accept キュー |

### TCP と UDP の違い — DNS は UDP

`accept()` は TCP 用。DNS は基本 UDP で動くので `recv_from()` を使う:

```rust
let socket = UdpSocket::bind("0.0.0.0:53")?;
let mut buf = [0; 512];
loop {
    let (n, addr) = socket.recv_from(&mut buf)?;  // ← ここで寝る
    socket.send_to(&response, addr)?;
}
```

UDP には接続概念がないので `bind()` 直後に `recv()` で読める。**accept キューが「ソケット受信バッファ」に変わるだけ**で構造は同じ。

### プロキシは「サーバー兼クライアント」

プロキシは同時に2つのソケットを持つ:

```
クライアント ──TCP──> プロキシ ──TCP──> オリジンサーバー
              (Aの接続)        (Bの接続)
```

- ソケット A: クライアントからの接続を `accept()` で受け取る
- ソケット B: 自分が `connect()` でオリジンに繋ぎに行く

カーネルから見るとプロキシプロセスは2本のソケット (FD) を持っているだけ。仕組みは同じ。

### L4 ロードバランサー (LVS など) はカーネル内で完結

ユーザー空間に上がらず、カーネル内のテーブルでルーティングして転送するだけ。HTTP の中身は見られないが高速。

### カーネルバイパス (DPDK, XDP) もある

超高性能用途では NIC を OS から取り上げてユーザー空間で直接制御する。Cloudflare、AWS、株式取引などで使用。

### 要点

- HTTP/DNS/プロキシ/ロードバランサー、何でもカーネル経由は同じ
- TCP は `accept()`、UDP は `recv_from()` という違いはあるが、内部の仕組みは同じ
- プロキシは「サーバーソケットとクライアントソケットを両方持つ」だけ
- L4 LB や eBPF はカーネル内で完結する高速化技術

## Q8: ポートを開いて listen する情報はどこに保持されている?

**カーネル内のソケットテーブル**。アプリが `bind()` + `listen()` でここに登録する。

```
┌──────────────────────────────────────────────────────────────┐
│ Linux カーネル ソケットテーブル                              │
├──────────────────────────────────────────────────────────────┤
│ プロトコル | ローカル:ポート | リモート:ポート | 状態 | PID  │
│ TCP       | 0.0.0.0:7878    | *:*             | LISTEN | 12345│
│ TCP       | 0.0.0.0:80      | *:*             | LISTEN | 23456│
│ UDP       | 0.0.0.0:53      | *:*             | (なし) | 34567│
│ TCP       | 192.168.1.5:7878| 192.168.1.10:54321 | ESTABLISHED | 12345 │
└──────────────────────────────────────────────────────────────┘
```

### `bind()` と `listen()` で起きること

```rust
let listener = TcpListener::bind("0.0.0.0:7878").unwrap();
```

| syscall | カーネル内処理 |
|---|---|
| `socket()` | ソケット構造体を確保、プロセスの FD テーブルに登録 |
| `bind()` | ソケットテーブルに `0.0.0.0:7878` を予約。重複なら EADDRINUSE エラー |
| `listen()` | 状態を LISTEN に。accept キュー (backlog 用) を確保 |

ハードウェア的に何かが変わるわけではなく、**カーネル内データ構造に1行追加されるだけ**。

### パケット受信時の処理フロー

1. パケット受信、TCP/IP ヘッダを解釈
2. ヘッダから5要素を取り出す: プロトコル、宛先 IP、宛先ポート、送信元 IP、送信元ポート
3. ソケットテーブルを検索
   - まず ESTABLISHED な5タプル一致を探す (既存接続のパケットか?)
   - なければ LISTEN しているソケットを3タプル (プロトコル, IP, ポート) で探す
4. 見つかればそのソケットに配送
5. 見つからなければ TCP は RST、UDP は ICMP Port Unreachable を返す

### 同じポート番号でも TCP/UDP は別

`TCP:80` と `UDP:80` は共存可能 (DNS が両方使うのは典型例)。一方、`UDP:53` を1つのプロセスが掴むと他は bind できない (EADDRINUSE)。

### Linux で観察

```bash
$ ss -tlnp
State   Recv-Q  Send-Q  Local Address:Port   Peer Address:Port   Process
LISTEN  0       128     0.0.0.0:22           0.0.0.0:*           users:(("sshd",pid=1234))
LISTEN  0       128     127.0.0.1:5432       0.0.0.0:*           users:(("postgres",pid=2345))
LISTEN  0       4096    0.0.0.0:7878         0.0.0.0:*           users:(("net-01",pid=3456))
```

カーネルのソケットテーブルを直接見ているのに相当。

### bind されていないポートに来たら

例: 誰も bind してない 9999 番に SYN を送ると、カーネルが RST を返送 → クライアント側 `ECONNREFUSED` ("Connection refused")。`telnet localhost 9999` で即エラーが返るのはこれ。

### SO_REUSEPORT — 複数プロセスで同じポートを listen

通常は EADDRINUSE になるが、`SO_REUSEPORT` を指定するとカーネルがハッシュ分散で複数 listener に配送する。nginx の worker_processes 複数化の仕組み。

### 要点

- 「Rust が特定ポートで待つ」の実態は「カーネル内のソケットテーブルに登録する」こと
- パケット受信時のカーネルはこのテーブルを引いて、対応するアプリ FD に配送
- 5タプル (プロトコル, src IP, src port, dst IP, dst port) で接続を一意に識別
- 同一ポートを複数プロセスで listen するには `SO_REUSEPORT` が必要

## Q9: NIC やカーネルのバグで本来処理すべきでないペイロードを処理してしまわないか? 完全に防ぐのは無理だろう

直感は正しい。**ネットワークスタックは攻撃面が極めて広く、過去に何度も実例がある**。

### 過去の実例

| CVE | 概要 |
|---|---|
| CVE-2019-11477 (SACK Panic) | Linux TCP SACK 実装の整数オーバーフロー。細工した SYN でリモート DoS |
| CVE-2018-5390 (SegmentSmack) | TCP セグメント再構築の計算量攻撃 |
| CVE-2020-16898 (Bad Neighbor) | Windows IPv6 RA パケットでカーネル権限の RCE |
| BlueBorne (2017) | Bluetooth スタックのバグ。Bluetooth が ON なだけで RCE |

どれも**正規プロトコル仕様の範囲内**のパケットで発火、カーネル内で完結 (アプリが accept する前)、カーネル特権で動くので最悪 root 取得。

### NIC ファームウェアにもバグはある

- Broadcom Wi-Fi チップ (Project Zero, 2017) — Wi-Fi 経由でチップ上のコード実行
- Cable Haunt (2020) — ケーブルモデムチップ、2億台超に影響

NIC は「ハードウェア+独自ファームウェア」で、そこにもバグはある。

### 業界の対応 — Defense in Depth (多層防御)

```
[ ISP/エッジファイアウォール ] ← 既知悪性 IP をブロック
[ ハードウェアファイアウォール ] ← パケット種別でフィルタ
[ NIC 上の eBPF/XDP ] ← カーネル本体に届く前に弾く
[ Linux netfilter/iptables ] ← カーネル内でフィルタ
[ カーネルの TCP/IP スタック ] ← バグが踏まれる可能性
[ プロセス分離 (SELinux等) ] ← 仮にバグを踏んでも被害最小化
[ アプリ層 (Rust/Goの安全性) ] ← ここで弾けば一番安心
```

### 最近の動き

- **eBPF / XDP** — NIC ドライバ直後で動く小プログラムで早期フィルタ。Cloudflare の DDoS 対策で活用
- **Rust for Linux** — カーネルドライバを Rust で書く動き。整数オーバーフロー等を構造的に防ぐ
- **マイクロカーネル/ユニカーネル** — TCP/IP スタックをカーネルから追い出す思想
- **スマート NIC (DPU)** — NVIDIA BlueField, AWS Nitro。NIC 側で処理してカーネル攻撃面を減らす

### 要点

- 100% 安全は無理。「枯れたコード+多層防御」で実用範囲に収めるのが現状
- 発見されたら即パッチ、層を重ねる、危険箇所は安全な言語で書き直す、被害を局所化
- Rust でアプリを書く意義の1つは「カーネル層のバグは仕方ないが、せめて自分のアプリでは同じ過ちを繰り返さない」

## Q10: 比較的脆弱なプロトコルとかある?

種類別に分類できる。

### 脆弱性の種類

| 種類 | 何が問題か | 例 |
|---|---|---|
| 平文通信 | 暗号化なし。盗聴・改ざんが容易 | Telnet, FTP, HTTP |
| 認証なし/弱認証 | なりすまし放題 | NTP, DNS, ARP, BGP |
| 増幅攻撃の踏み台 | 小さなリクエストで大きなレスポンス | DNS, NTP, Memcached |
| ステート管理の脆弱性 | プロトコル状態を悪用 | TCP SYN flood |
| 設計時の前提が崩れた | 信頼できる小規模ネット前提だった | 古いプロトコル全般 |

### レガシープロトコル (性善説時代の遺産)

| プロトコル | 用途 | 何がヤバいか | 代替 |
|---|---|---|---|
| Telnet | リモートログイン | パスワード平文 | SSH |
| FTP | ファイル転送 | パスワード平文、データチャネル別接続 | SFTP, FTPS |
| HTTP | Web | 平文、中間者が書き換え可能 | HTTPS |
| SMTP (plain) | メール送信 | 平文中継、なりすまし可 | SMTP+STARTTLS |
| POP3/IMAP plain | メール受信 | 平文 | POP3S/IMAPS |
| SNMP v1/v2c | ネットワーク機器管理 | community string 平文 | SNMP v3 |
| TFTP | 簡易ファイル転送 | 認証ゼロ | (用途次第) |
| rsh/rlogin | リモート実行 | IP ベース信用 | SSH |
| Finger | ユーザー情報照会 | 情報ダダ漏れ | 廃止 |

理由: 初期インターネット (ARPANET) は大学・研究機関だけの「みんな知り合い」前提。90年代に商業化されて問題化。

### 現役で構造的に弱いプロトコル

#### DNS
- UDP ベース + 認証なし (DNSSEC 普及率低)
- キャッシュポイズニング (Kaminsky 攻撃, 2008)
- 増幅攻撃の踏み台

#### BGP
- 「これらの IP は私の AS のもの」と宣言するだけで信用される
- BGP ハイジャック — 他人の IP 範囲を偽宣言してトラフィック横取り
- 2008年パキスタン政府が YouTube を遮断しようとして全世界の YouTube を吸い込んだ
- 対策: RPKI (普及途上)

#### NTP
- 認証オプションはあるがほぼ未使用
- 増幅攻撃で最強クラス

#### ARP
- LAN 内: 認証なしで「IP X の MAC は Y」と宣言可能
- ARP スプーフィング — 中間者攻撃の入口
- 公衆 Wi-Fi の傍受で常套手段

#### ICMP
- Ping of Death, Smurf 攻撃, ICMP redirect

#### DHCP
- 認証なし。不正な DHCP サーバーで全クライアントを攻撃者ルーターに誘導

### 増幅攻撃の踏み台 (Amplification)

「小さなリクエストで大きなレスポンス」が踏み台にされる。送信元 IP を偽装して被害者に応答を集中させる。

| プロトコル | 増幅率 |
|---|---|
| NTP (monlist) | **約 556 倍** |
| Memcached (UDP) | **数万〜数十万倍** (2018 GitHub の 1.35 Tbps DDoS 原因) |
| DNS | 約 28-54 倍 |
| SSDP (UPnP) | 約 30 倍 |
| LDAP | 約 46 倍 |
| SNMP | 約 6 倍 |

ほぼ全部 UDP。TCP は3ウェイハンドシェイクで送信元偽装が難しいので踏み台にされにくい。

### TCP 自体の問題

- SYN flood — 3ウェイハンドシェイク途中状態を大量生成して accept キュー枯渇
- TCP Reset 注入 — シーケンス番号を当てて接続強制切断 (Great Firewall で使用)
- シーケンス番号予測攻撃 — Mitnick 攻撃 (1994)

### IP レベル

- IP スプーフィング — 送信元 IP を自由に書き換え可能
- IP フラグメンテーション — 再構築バグで多数の CVE
- IPv6 拡張ヘッダ — 仕様が複雑で実装バグの温床

### アプリ層の「危ないクセ」

プロトコル自体は枯れていても実装で問題:

- HTTP Smuggling — フロント/バックエンドの Content-Length/Transfer-Encoding 解釈ずれ
- HTTP/2 Rapid Reset (CVE-2023-44487) — ストリーム開閉繰り返しで DDoS
- TLS ダウングレード攻撃 — POODLE, BEAST, FREAK

### 現代設計の安全なプロトコル

| プロトコル | 用途 | 特徴 |
|---|---|---|
| TLS 1.3 | 暗号化全般 | 古い暗号スイートを排除 |
| SSH | リモート操作 | 公開鍵認証、暗号化必須 |
| QUIC / HTTP/3 | Web | TLS 1.3 内包、UDP 上で実装 |
| WireGuard | VPN | 暗号スイート固定、コード行数も少ない |
| DoT / DoH | DNS | DNS を TLS/HTTPS で暗号化 |
| Signal Protocol | メッセージング | E2EE、フォワード秘匿性 |

WireGuard は「セキュア設計の手本」と評される。OpenVPN/IPsec の数十万行 vs WireGuard 4000 行。機能を削ることで攻撃面を最小化。

### 要点

- レガシー (Telnet, FTP, HTTP, SMTP plain) は性善説時代の遺産
- 現役で弱いのは DNS, BGP, NTP, ARP, ICMP — 認証や暗号化が後付け
- UDP ベースで増幅率高いものは DDoS 踏み台 (NTP, Memcached, DNS)
- TCP は比較的堅牢だが SYN flood や RST 注入はある
- 現代設計 (TLS 1.3, SSH, QUIC, WireGuard) はセキュリティ込み
- アプリ実装時は **プロトコル選択でレガシーを避ける**のが最大の防御

---

日付: 2026-05-22

## Q11: `writer.write_all` の定義は `(self, buf)` なのに `buf` だけ渡せばいいのはなぜ？

`.` の左側がレシーバ (`&mut self`) として暗黙的に第1引数になるから。Goのメソッドレシーバと同じ。

```rust
fn write_all(&mut self, buf: &[u8]) -> io::Result<()>
```

`writer.write_all(buf)` と書くと:
- `writer` → `&mut self` に渡される (`.` の左側)
- `buf` → 第2引数として明示的に渡す

これは UFCS で明示的にも書ける:

```rust
Write::write_all(&mut writer, buf)   // self を明示
writer.write_all(buf)                // 上と等価。普段はこちら
```

同名メソッドが複数のトレイトにあって曖昧なときに UFCS を使う、というくらいで、普段はメソッド呼び出し形式で OK。

Go の `writer.Write(buf)` も `func (w *Writer) Write(buf []byte)` のレシーバが暗黙で渡されているのと完全に同じ仕組み。

## Q12: エラー処理で `if .. is_err()` を使うときと `expect()` を使うのの違いは？

エラー発生時の振る舞いが根本的に違う。

| やり方 | エラー時の動作 | 用途 |
|---|---|---|
| `expect("msg")` | **panic してプロセスを落とす** | 復旧不能な状況、絶対失敗しない処理、試作コード |
| `unwrap()` | 同上 (メッセージなし) | 同上 |
| `?` 演算子 | エラーを呼び出し元に伝播 (`Result` を返す関数内のみ) | エラーをそのまま上に投げたい |
| `if .. is_err()` | panic しない。エラーかどうかだけ判定 | ループから break する、ログ出力する等 |
| `match` / `if let Err(e) = ..` | panic しない。エラーの中身も使える | エラー内容に応じて分岐したい |

`is_err()` は bool しか返さないので、成功時の戻り値もエラーの中身も両方失われる点に注意。`write_all` の戻り値は `Result<(), io::Error>` で成功時は `()` (中身なし) なので問題ない。エラーの内容を見たいなら `match` のほうが情報量が多い。

```rust
// 失敗してもループを抜けて切断処理に進みたい場面
if writer.write_all(buf).is_err() {
    println!("送信に失敗");
    break;
}

// もう終了直前で、失敗したら落としていい場面
writer.write_all(b"\n").expect("送信に失敗");
```

Go との対応:
- `expect()` ≒ `if err != nil { panic(err) }`
- `if .. is_err()` ≒ `if err != nil { /* 処理 */ }`

## Q13: `if let Some(Ok(response)) = responses.next()` の `Some`, `Ok`, `response` は何？

2段の入れ子型を1行でパターンマッチしている。

`BufRead::lines()` のイテレータの `.next()` の戻り値:

```rust
Option<io::Result<String>>
// = Option<Result<String, io::Error>>
```

| 外側の `Option` | 意味 |
|---|---|
| `Some(x)` | 次の行がある (中身 `x` は `Result<String, io::Error>`) |
| `None` | もうない (EOF / 接続終了) |

| 内側の `Result` | 意味 |
|---|---|
| `Ok(s)` | 成功 (中身 `s` は `String`) |
| `Err(e)` | I/O エラー |

`Some(Ok(response))` は **「次の行があって、かつ読み取りも成功した場合だけ」** マッチして、`response` という名前で `String` を束縛する。

- `Some(...)` `Ok(...)` → enum のコンストラクタ名 (型側の話)
- `response` → 自分が定義する変数名 (ここで初めて束縛される)

Go で書くと2段の `if`:

```go
line, ok := responses.Next()
if !ok { return }                       // None
response, err := line.Value, line.Err
if err != nil { return }                // Err
// response (string) が使える
```

`if let` だと「マッチしなければ何もしない」挙動なので、EOFやI/Oエラーを区別したい場合は `match` で3パターン書くのが安全:

```rust
match responses.next() {
    Some(Ok(response)) => println!("< {response}"),
    Some(Err(e)) => { println!("受信エラー: {e}"); break; }
    None => { println!("サーバーが切断"); break; }
}
```

## Q14: サーバーからのレスポンスを受け取るのって `next()` でいいの？サーバーの性質にもよる？

サーバーのプロトコル次第。`next()` でうまくいくのは以下を満たすとき:

- 行ベースのテキストプロトコル
- 1リクエスト = 1行のレスポンス

`reader.lines()` は改行 `\n` で区切られた文字列を返すイテレータで、`.next()` は **改行が来るまでブロック**し1行返したら止まる。今回のエコーサーバーや TIME/ECHO/QUIT のコマンドサーバーはこの条件を満たすので問題ない。

### `next()` だと壊れるパターン

| サーバーの性質 | 壊れ方 | 適切な読み方 |
|---|---|---|
| 改行を付けないレスポンス | `next()` が永遠にブロック | プロトコルに合わせた区切り検出 |
| 1リクエストに複数行返す | 1行目しか取れずズレる | `lines()` をループして終端マーカーで break |
| HTTP のような構造化プロトコル | 行 = 意味単位ではない | `httparse`、`hyper`、`reqwest` |
| バイナリプロトコル (長さプレフィックス) | `\n` (0x0A) がペイロードに混ざると誤分割 | `read_exact(&mut len)` → `read_exact(&mut payload)` |
| 固定長レコード | 区切り文字がない | `read_exact(&mut [0u8; N])` |
| プッシュ型 (WebSocket, SSE) | リクエスト/レスポンス対応が崩れる | 別スレッドで受信ループ or `tokio` |

### Go との対応

`bufio.Scanner.Scan()` が `lines().next()` と同じ位置づけ。改行区切りプロトコル専用。バイナリなら `binary.Read` や `io.ReadFull` を使うのも Rust と同じ発想。

「行プロトコル前提」という暗黙の契約を意識しておくと、別プロトコルを扱うときに躓かない。今回 `writer.write_all(format!("{}\n", text).as_bytes())` で必ず `\n` を付けているのは、行プロトコルとして整合させているから。

## Q15: `TIME` や `QUIT` の純粋な一致は match でできるが、`ECHO XX` のような前方一致は match で判断できない。どうするのがよい？

3パターンある。一番おすすめは「**先に分割してから match**」。

### ① マッチガード (`_ if ..`)

```rust
match text.as_str() {
    "TIME" => { ... }
    "QUIT" => { break; }
    _ if text.starts_with("ECHO ") => text[5..].to_string(),
    _ => "UNKNOWN COMMAND".to_string(),
}
```

動くがマジックナンバー `5` (= `"ECHO "` の長さ) がハードコードされる。`text[5..]` はバイトインデックスなので、もしプレフィックスにマルチバイト文字を含めるとパニックする落とし穴もある。

### ② `strip_prefix` + `if let`

```rust
if let Some(rest) = text.strip_prefix("ECHO ") {
    rest.to_string()
} else {
    match text.as_str() { ... }
}
```

`strip_prefix` は「prefix が一致したら残りを `Some`、しなければ `None`」を返す。マジックナンバーが消える。

### ③ 先に分割してから match (おすすめ)

```rust
let mut parts = text.splitn(2, ' ');
let verb = parts.next().unwrap_or("");
let args = parts.next().unwrap_or("");

let response = match verb {
    "TIME" => { ... }
    "QUIT" => break,
    "ECHO" => args.to_string(),
    _ => "UNKNOWN COMMAND".to_string(),
};
```

- 動詞と引数を最初に分けることで、`match` は純粋な完全一致だけになって読みやすい
- 将来 `SET key value` のような複数引数コマンドを足すときも同じ枠で書ける

Go ならまさにこのスタイル (`strings.SplitN` + `switch`)。「コマンドパーサは先に分解、その後で `match`/`switch`」は言語問わず定石。

### 使い分けの目安

| 状況 | 適した書き方 |
|---|---|
| 動詞 + 引数 の構造を持つコマンド群 | ③ 先に splitn してから match |
| 1個だけ前方一致条件が混ざる | ② `if let strip_prefix` |
| アドホックに条件を1つだけ足したい | ① マッチガード |

## Q16: `splitn` の `n` ってなに？

「**最大何個の要素に分割するか**」の上限。`split` + `n` (umber)。

```rust
"a b c d".splitn(2, ' ')   // → ["a", "b c d"]      最大2個 → 1回だけ分割
"a b c d".splitn(3, ' ')   // → ["a", "b", "c d"]    最大3個 → 2回分割
"a b c d".splitn(10, ' ')  // → ["a", "b", "c", "d"] 上限に達しないので全部
"a b c d".split(' ')       // → ["a", "b", "c", "d"] split は無制限
```

注意: `n` は「**最終的に何要素にするか**」であって「何回分割するか」ではない。`splitn(2, ...)` は「分割回数1回」ではなく「結果が最大2要素」。

### ECHO コマンドで `splitn(2, ' ')` を使う理由

`ECHO hello world` のようなコマンド:

- `split(' ')` → `["ECHO", "hello", "world"]` で引数が分割されてしまう
- `splitn(2, ' ')` → `["ECHO", "hello world"]` で動詞と「残り全部」に分けられる

「最初のスペース1個だけで切りたい」という意図の表現。

### Go との対応

`strings.SplitN("ECHO hello world", " ", 2)` と完全に同じ意味。Rust も Go も **N は「最終的に何個の要素にするか」の上限**。

### 仲間

| メソッド | 動作 |
|---|---|
| `split(sep)` | 無制限に分割 |
| `splitn(n, sep)` | **前から** 最大 n 個 |
| `rsplitn(n, sep)` | **後ろから** 最大 n 個 |
| `split_once(sep)` | 最初の1回だけ分割して `Option<(&str, &str)>` |

「動詞と残り」のケースは `split_once` の方がシンプル:

```rust
let (verb, args) = text.split_once(' ').unwrap_or((text.as_str(), ""));
```

`splitn(2, ...)` でも書けるが、「2要素に分けたい」と意図がより明確なのは `split_once`。
