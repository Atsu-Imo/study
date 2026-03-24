// net-01: TCPエコーサーバー
// ネットワークプログラミングの第一歩：
// クライアントが送ったデータをそのまま返す「エコーサーバー」を作る

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

// =============================================================================
// エコーサーバー
// =============================================================================

/// クライアントからの接続を1つ処理する関数
/// 受け取ったデータをそのまま返す（エコー）
fn handle_client(stream: TcpStream) {
    // 接続元のアドレスを表示
    // peer_addr() で「相手は誰か」がわかる
    let addr = stream.peer_addr().expect("接続元アドレスの取得に失敗");
    println!("[サーバー] クライアント接続: {addr}");

    // BufReaderで行単位の読み取りを可能にする
    // TcpStream はバイト列のストリームなので、行の区切りを自分で処理する必要がある
    let reader = BufReader::new(&stream);

    // 書き込み用に stream のクローンを作る
    // TcpStream は読み書き両方できるが、BufReader が &stream を借用しているため
    // 書き込み用に別のハンドルが必要
    let mut writer = stream.try_clone().expect("ストリームのクローンに失敗");

    // 1行ずつ読み取って、そのまま返す
    for line in reader.lines() {
        match line {
            Ok(text) => {
                if text.is_empty() {
                    println!("[サーバー] クライアント {addr} が空行を送信 → 切断");
                    break;
                }
                println!("[サーバー] 受信 from {addr}: {text}");

                // 受け取ったテキストをそのまま返す（末尾に改行を付ける）
                // write_all は全バイトが書き込まれるまでブロックする
                if writer.write_all(format!("{text}\n").as_bytes()).is_err() {
                    println!("[サーバー] クライアント {addr} への書き込み失敗");
                    break;
                }
            }
            Err(e) => {
                println!("[サーバー] クライアント {addr} の読み取りエラー: {e}");
                break;
            }
        }
    }

    println!("[サーバー] クライアント {addr} 切断");
}

/// エコーサーバーを起動する
fn run_server(addr: &str) {
    // TcpListener::bind で指定アドレス:ポートに「バインド」する
    // これは「このポートで待ち受けます」とOSに宣言すること
    let listener = TcpListener::bind(addr).expect("バインドに失敗");
    println!("[サーバー] {addr} でリッスン開始");
    println!("[サーバー] Ctrl+C で停止");

    // incoming() は接続を待ち受けるイテレータ
    // 新しいクライアントが接続するたびに TcpStream を返す
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // 各クライアントを別スレッドで処理する
                // これにより、複数のクライアントを同時に扱える
                thread::spawn(|| {
                    handle_client(stream);
                });
            }
            Err(e) => {
                eprintln!("[サーバー] 接続受付エラー: {e}");
            }
        }
    }
}

// =============================================================================
// エコークライアント
// =============================================================================

/// サーバーに接続してメッセージを送り、エコーを受け取る
fn run_client(addr: &str) {
    // TcpStream::connect でサーバーに接続する
    // ここで TCP の3ウェイハンドシェイク（SYN → SYN-ACK → ACK）が行われる
    let stream = TcpStream::connect(addr).expect("サーバーへの接続に失敗");
    println!("[クライアント] {addr} に接続成功");

    // 読み取り用と書き込み用にストリームを分ける
    let reader = BufReader::new(&stream);
    let mut writer = stream.try_clone().expect("ストリームのクローンに失敗");
    let mut lines = reader.lines();

    // テストメッセージを送信
    let messages = ["Hello, Server!", "Rust networking!", "エコーテスト"];

    for msg in &messages {
        println!("[クライアント] 送信: {msg}");
        writer
            .write_all(format!("{msg}\n").as_bytes())
            .expect("送信に失敗");

        // サーバーからのエコーを読み取り
        if let Some(Ok(response)) = lines.next() {
            println!("[クライアント] 受信: {response}");
        }
    }

    // 空行を送って切断を知らせる
    writer.write_all(b"\n").expect("送信に失敗");
    println!("[クライアント] 切断");
}

// =============================================================================
// メイン
// =============================================================================

// =============================================================================
// TCPハンドシェイク シミュレーション
// =============================================================================
// 実際のTCPハンドシェイクはOSカーネルが行うため、アプリケーションからは見えない。
// ここではその仕組みをチャネルで再現して「何が起きているか」を可視化する。

use std::sync::mpsc;

/// TCPセグメントのフラグ（実際のTCPヘッダーにあるフラグを模したもの）
#[derive(Debug)]
enum TcpFlag {
    Syn { seq: u32 },                   // 接続要求
    SynAck { seq: u32, ack: u32 },      // 接続要求 + 確認応答
    Ack { seq: u32, ack: u32 },         // 確認応答
    Data { seq: u32, payload: String }, // データ送信
    Fin { seq: u32 },                   // 切断要求
}

/// TCP接続の状態（RFC 793 で定義された状態遷移の一部）
#[derive(Debug, PartialEq)]
enum TcpState {
    Closed,      // 初期状態
    Listen,      // 接続待ち（サーバー）
    SynSent,     // SYN送信済み（クライアント）
    SynReceived, // SYN受信済み（サーバー）
    Established, // 接続確立（データ送受信可能）
    FinWait,     // 切断要求送信済み
}

impl std::fmt::Display for TcpState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TcpState::Closed => write!(f, "CLOSED"),
            TcpState::Listen => write!(f, "LISTEN"),
            TcpState::SynSent => write!(f, "SYN_SENT"),
            TcpState::SynReceived => write!(f, "SYN_RECEIVED"),
            TcpState::Established => write!(f, "ESTABLISHED"),
            TcpState::FinWait => write!(f, "FIN_WAIT"),
        }
    }
}

fn run_handshake_simulation() {
    println!("=== TCP 3ウェイハンドシェイク シミュレーション ===\n");
    println!("実際のTCPハンドシェイクはOSカーネルが行う。");
    println!("ここではチャネルを使ってその動作を再現する。\n");

    // チャネル = ネットワーク回線に見立てる
    // クライアント→サーバー と サーバー→クライアント の2本
    let (client_tx, server_rx) = mpsc::channel::<TcpFlag>();
    let (server_tx, client_rx) = mpsc::channel::<TcpFlag>();

    // --- サーバー側 ---
    let server = thread::spawn(move || {
        let mut state = TcpState::Closed;
        println!("  [サーバー] 状態: {state}");

        // LISTEN状態に遷移
        state = TcpState::Listen;
        println!("  [サーバー] 状態: {state} (接続待ち)");

        // Step 1: SYNを受信
        if let Ok(TcpFlag::Syn { seq }) = server_rx.recv() {
            println!("  [サーバー] ← SYN 受信 (seq={seq})");
            state = TcpState::SynReceived;
            println!("  [サーバー] 状態: {state}");

            // Step 2: SYN-ACKを送信
            // ack = 相手のseq+1 (「あなたのseqまで受け取ったよ」の意味)
            let server_seq = 300;
            println!(
                "  [サーバー] → SYN-ACK 送信 (seq={server_seq}, ack={})",
                seq + 1
            );
            server_tx
                .send(TcpFlag::SynAck {
                    seq: server_seq,
                    ack: seq + 1,
                })
                .ok();

            // Step 3: ACKを受信
            if let Ok(TcpFlag::Ack { seq, ack }) = server_rx.recv() {
                println!("  [サーバー] ← ACK 受信 (seq={seq}, ack={ack})");
                state = TcpState::Established;
                println!("  [サーバー] 状態: {state} ★接続確立！\n");
            }

            // --- データ送受信フェーズ ---
            if let Ok(TcpFlag::Data { seq, payload }) = server_rx.recv() {
                println!("  [サーバー] ← データ受信 (seq={seq}): \"{payload}\"");
                println!("  [サーバー] → データ送信: \"{payload}\" (エコー)");
                server_tx
                    .send(TcpFlag::Data {
                        seq: server_seq + 1,
                        payload,
                    })
                    .ok();
            }

            // --- 切断フェーズ ---
            if let Ok(TcpFlag::Fin { seq }) = server_rx.recv() {
                println!("\n  [サーバー] ← FIN 受信 (seq={seq})");
                println!("  [サーバー] → ACK 送信");
                server_tx
                    .send(TcpFlag::Ack {
                        seq: server_seq + 2,
                        ack: seq + 1,
                    })
                    .ok();
                println!("  [サーバー] → FIN 送信");
                server_tx
                    .send(TcpFlag::Fin {
                        seq: server_seq + 3,
                    })
                    .ok();
                state = TcpState::Closed;
                println!("  [サーバー] 状態: {state}");
            }
        }
    });

    // --- クライアント側 ---
    let client = thread::spawn(move || {
        let mut state = TcpState::Closed;
        println!("  [クライアント] 状態: {state}");

        // サーバーがLISTENになるのを少し待つ
        thread::sleep(std::time::Duration::from_millis(50));

        println!("\n--- 接続フェーズ (3ウェイハンドシェイク) ---\n");

        // Step 1: SYNを送信
        let client_seq = 100;
        println!("  [クライアント] → SYN 送信 (seq={client_seq})");
        state = TcpState::SynSent;
        println!("  [クライアント] 状態: {state}");
        client_tx.send(TcpFlag::Syn { seq: client_seq }).ok();

        // Step 2: SYN-ACKを受信
        if let Ok(TcpFlag::SynAck {
            seq: server_seq,
            ack,
        }) = client_rx.recv()
        {
            println!("  [クライアント] ← SYN-ACK 受信 (seq={server_seq}, ack={ack})");

            // Step 3: ACKを送信 → 接続確立
            println!(
                "  [クライアント] → ACK 送信 (seq={}, ack={})",
                client_seq + 1,
                server_seq + 1
            );
            client_tx
                .send(TcpFlag::Ack {
                    seq: client_seq + 1,
                    ack: server_seq + 1,
                })
                .ok();
            state = TcpState::Established;
            println!("  [クライアント] 状態: {state} ★接続確立！");
        }

        // --- データ送受信フェーズ ---
        println!("\n--- データ送受信フェーズ ---\n");
        let message = "Hello, TCP!".to_string();
        println!("  [クライアント] → データ送信: \"{message}\"");
        client_tx
            .send(TcpFlag::Data {
                seq: client_seq + 1,
                payload: message,
            })
            .ok();

        if let Ok(TcpFlag::Data { seq, payload }) = client_rx.recv() {
            println!("  [クライアント] ← データ受信 (seq={seq}): \"{payload}\"");
        }

        // --- 切断フェーズ ---
        println!("\n--- 切断フェーズ (FIN) ---\n");
        println!("  [クライアント] → FIN 送信 (切断要求)");
        client_tx
            .send(TcpFlag::Fin {
                seq: client_seq + 2,
            })
            .ok();
        state = TcpState::FinWait;
        println!("  [クライアント] 状態: {state}");

        if let Ok(TcpFlag::Ack { .. }) = client_rx.recv() {
            println!("  [クライアント] ← ACK 受信");
        }
        if let Ok(TcpFlag::Fin { .. }) = client_rx.recv() {
            println!("  [クライアント] ← FIN 受信");
            println!("  [クライアント] → ACK 送信");
            state = TcpState::Closed;
            println!("  [クライアント] 状態: {state}");
        }
    });

    server.join().ok();
    client.join().ok();

    println!("\n--- シーケンス図まとめ ---\n");
    println!("  クライアント                    サーバー");
    println!("      │                              │");
    println!("      │  SYN (seq=100)               │  ← 接続");
    println!("      │─────────────────────────────>│");
    println!("      │                              │");
    println!("      │  SYN-ACK (seq=300, ack=101)  │");
    println!("      │<─────────────────────────────│");
    println!("      │                              │");
    println!("      │  ACK (seq=101, ack=301)      │");
    println!("      │─────────────────────────────>│  ← 確立");
    println!("      │                              │");
    println!("      │  DATA \"Hello, TCP!\"          │  ← 通信");
    println!("      │─────────────────────────────>│");
    println!("      │  DATA \"Hello, TCP!\"          │");
    println!("      │<─────────────────────────────│");
    println!("      │                              │");
    println!("      │  FIN                         │  ← 切断");
    println!("      │─────────────────────────────>│");
    println!("      │  ACK + FIN                   │");
    println!("      │<─────────────────────────────│");
    println!("      │                              │");

    println!("\n=== シミュレーション終了 ===");
    println!("\n補足: 実際のハンドシェイクを観察するには:");
    println!("  ターミナル1: sudo tcpdump -i lo port 7878 -nn -S");
    println!("  ターミナル2: cargo run -- server");
    println!("  ターミナル3: cargo run -- client");
}

fn main() {
    let addr = "127.0.0.1:7878";

    // コマンドライン引数でサーバーかクライアントかを決める
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("demo");

    match mode {
        "server" => {
            // サーバーモード: cargo run -- server
            run_server(addr);
        }
        "client" => {
            // クライアントモード: cargo run -- client
            run_client(addr);
        }
        "handshake" => {
            // ハンドシェイクシミュレーション: cargo run -- handshake
            run_handshake_simulation();
        }
        "demo" => {
            // デモモード（デフォルト）: サーバーを起動してクライアントを自動実行
            println!("=== TCP エコーサーバー デモ ===\n");
            println!("仕組み:");
            println!("  1. サーバーが 127.0.0.1:7878 で待ち受け開始");
            println!("  2. クライアントが接続してメッセージを送信");
            println!("  3. サーバーが受け取ったメッセージをそのまま返す");
            println!();

            // サーバーを別スレッドで起動
            let server_addr = addr.to_string();
            thread::spawn(move || {
                run_server(&server_addr);
            });

            // サーバーの起動を少し待つ
            thread::sleep(std::time::Duration::from_millis(100));

            // クライアントを実行
            run_client(addr);

            println!("\n=== デモ終了 ===");
            println!("\n別々のターミナルで試すには:");
            println!("  ターミナル1: cargo run -- server");
            println!("  ターミナル2: cargo run -- client");
        }
        other => {
            eprintln!("不明なモード: {other}");
            eprintln!("使い方: cargo run -- [server|client|demo|handshake]");
        }
    }
}

// =============================================================================
// 演習
// =============================================================================

// --- 演習1: 基礎 ---
// エコーサーバーを改造して、受け取った文字列を大文字にして返す
// 「SHOUT サーバー」を作ってみよう
//
// ヒント: String の .to_uppercase() メソッドを使う
//
// fn handle_client_shout(stream: TcpStream) {
//     let addr = stream.peer_addr().expect("アドレス取得失敗");
//     let reader = BufReader::new(&stream);
//     let mut writer = stream.try_clone().expect("クローン失敗");
//
//     for line in reader.lines() {
//         match line {
//             Ok(text) => {
//                 if text.is_empty() { break; }
//                 let shouted = todo!("ここで大文字に変換");
//                 writer.write_all(format!("{shouted}\n").as_bytes()).ok();
//             }
//             Err(_) => break,
//         }
//     }
// }

// --- 演習2: 応用 ---
// 標準入力から読み取った文字列をサーバーに送る対話型クライアントを作ろう
//
// ヒント: std::io::stdin().lock().lines() で標準入力を行単位で読める
//
// fn run_interactive_client(addr: &str) {
//     let stream = TcpStream::connect(addr).expect("接続失敗");
//     let reader = BufReader::new(&stream);
//     let mut writer = stream.try_clone().expect("クローン失敗");
//     let mut responses = reader.lines();
//     let stdin = std::io::stdin();
//
//     println!("メッセージを入力してください（空行で終了）:");
//     for line in stdin.lock().lines() {
//         let text = line.expect("入力エラー");
//         if text.is_empty() { break; }
//         todo!("サーバーに送信して、レスポンスを表示する");
//     }
// }

// --- 演習3: チャレンジ ---
// サーバーに簡単なコマンド機能を追加しよう:
//   - "TIME" と送ると現在時刻を返す
//   - "ECHO xxxx" と送ると xxxx をそのまま返す
//   - "QUIT" と送ると接続を切断する
//   - それ以外は "UNKNOWN COMMAND" と返す
//
// ヒント: text.starts_with("ECHO ") や text.as_str() でマッチング
//
// fn handle_client_command(stream: TcpStream) {
//     // ...
//     // match text.as_str() {
//     //     "TIME" => { /* 時刻を返す */ }
//     //     "QUIT" => { break; }
//     //     _ if text.starts_with("ECHO ") => { /* ECHO以降を返す */ }
//     //     _ => { /* UNKNOWN COMMAND を返す */ }
//     // }
// }
