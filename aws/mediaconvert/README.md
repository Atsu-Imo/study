# AWS MediaConvert 学習カリキュラム

MediaConvertを使いこなすために必要な映像の前提知識から、サービス自体の理解までを段階的に学ぶ。
生データを直接いじることは想定せず、「MediaConvertに適切な設定を渡せる」ことをゴールとする。

---

## Phase 1: 映像の基礎概念

MediaConvertの設定項目が何を意味しているか理解するための土台。

### 1-1. 映像の仕組み

- フレームとフレームレート（fps）
  - 静止画の連続としての動画
  - 24fps / 30fps / 60fps の違いと用途
  - インターレース vs プログレッシブ（i と p の意味。1080i vs 1080p）
- 解像度
  - ピクセルとアスペクト比
  - SD / HD / Full HD / 4K / 8K
  - DAR（Display Aspect Ratio）と SAR（Sample Aspect Ratio）

### 1-2. 色と画質の基礎

- 色空間（BT.601 / BT.709 / BT.2020）
  - SDR と HDR の違い
  - HDR10 / HLG / Dolby Vision の概要
- ビット深度（8bit / 10bit / 12bit）
  - 色の階調数との関係
  - バンディング（グラデーションの段差）

### 1-3. 音声の基礎

- サンプリングレートとビット深度（音声側）
- チャンネル構成（モノラル / ステレオ / 5.1ch / 7.1ch）
- ラウドネス（LUFS）と音量正規化

---

## Phase 2: コーデックとコンテナ

MediaConvertの設定の大半はここに関わる。最重要フェーズ。

### 2-1. コーデックとは何か

- エンコード / デコードの概念
- なぜ圧縮が必要か（生データのサイズ感）
- 非可逆圧縮 vs 可逆圧縮

### 2-2. 映像コーデック

- H.264（AVC）— 最も普及、互換性最強
- H.265（HEVC）— H.264の後継、圧縮効率2倍、ライセンス問題
- VP9 — Google製、YouTube標準
- AV1 — ロイヤリティフリー、次世代標準
- 各コーデックの使い分け（配信先・デバイス・コストの観点）

### 2-3. 音声コーデック

- AAC — 最も一般的
- AC-3（Dolby Digital）/ E-AC-3（Dolby Digital Plus）
- Opus — 低ビットレートに強い
- MP3 — レガシー

### 2-4. コンテナフォーマット

- コンテナとコーデックの関係（箱と中身）
- MP4（.mp4）— 最も汎用的
- MKV（.mkv）— 柔軟、字幕・多音声向き
- MOV（.mov）— Apple系
- MPEG-TS（.ts）— 放送・ライブ配信向き
- fMP4（Fragmented MP4）— ストリーミング向き
- WebM — Web向き（VP9/AV1 + Opus）

### 2-5. ビットレートとエンコード設定

- CBR（固定ビットレート）vs VBR（可変ビットレート）
- QVBR（Quality-defined VBR）— MediaConvert推奨
- 2パスエンコード vs 1パスエンコード
- GOP（Group of Pictures）とキーフレーム間隔
  - I / P / B フレームの役割
  - ストリーミングとの関係（セグメント境界）
- プロファイルとレベル（H.264 Baseline / Main / High など）

---

## Phase 3: ストリーミング配信の仕組み

MediaConvertの主要ユースケースがストリーミング向けの変換であるため。

### 3-1. 配信プロトコル

- プログレッシブダウンロード vs アダプティブストリーミング
- HLS（HTTP Live Streaming）
  - マニフェスト（.m3u8）とセグメント（.ts / .fmp4）
  - Apple由来、広くサポート
- DASH（Dynamic Adaptive Streaming over HTTP）
  - MPD マニフェストとセグメント
  - 国際標準、DRM連携
- CMAF（Common Media Application Format）
  - HLS と DASH の統合
  - fMP4ベース

### 3-2. ABR（Adaptive Bitrate）

- ABRの仕組み（ネットワーク状況に応じて品質切替）
- エンコードラダー（複数解像度 × 複数ビットレート）の設計
- Per-Title Encoding / コンテンツ適応エンコードの考え方

### 3-3. DRM（デジタル著作権管理）

- DRMの概要と目的
- Widevine / FairPlay / PlayReady
- CENC（Common Encryption）と暗号化方式
- MediaConvertでのDRM設定（SPEKE連携）

### 3-4. 字幕・キャプション

- 焼き込み字幕（Burn-in）vs サイドカー字幕
- SRT / WebVTT / TTML / CEA-608 / CEA-708
- 字幕の扱いがコンテナやプロトコルで異なる点

---

## Phase 4: AWS MediaConvert 自体の理解

前提知識が揃った上で、サービスの設定と使い方を学ぶ。

### 4-1. MediaConvertの位置づけ

- AWS メディア系サービスの全体像
  - MediaConvert（ファイルベース変換）
  - MediaLive（ライブエンコード）
  - MediaPackage（パッケージング・配信）
  - CloudFront（CDN）
- MediaConvertのユースケース
  - VOD（ビデオオンデマンド）パイプライン
  - アーカイブ変換
  - 広告挿入用素材の準備

### 4-2. ジョブの構造

- Job / Queue / Preset / Job Template の関係
- Input の設定
  - ソースファイル（S3）
  - クリッピング（開始・終了時間）
  - 音声セレクタ / 字幕セレクタ
- Output Group の設定
  - File Group（単一ファイル出力）
  - HLS Group / DASH ISO Group / CMAF Group
  - Microsoft Smooth Streaming Group
- Output の設定（コーデック・解像度・ビットレート等）

### 4-3. 主要な設定項目

- 映像設定
  - コーデック選択と各パラメータ
  - 解像度・フレームレート変換
  - QVBR の Quality Level 設定
- 音声設定
  - コーデック選択
  - 音声正規化（ラウドネス補正）
  - 多言語音声トラックの扱い
- フィルタ（デインターレース、ノイズリダクション等）
- タイムコードの扱い

### 4-4. 運用面

- オンデマンドキュー vs リザーブドキュー（コスト）
- ジョブの優先度設定
- IAMロール（S3読み書き権限）
- CloudWatch連携（ジョブの監視）
- EventBridge連携（ジョブ完了通知 → 後続処理）
- エラーハンドリングとリトライ
- 料金体系（出力の長さ × 解像度 × コーデック）

### 4-5. 実践的パイプライン構成

- S3 → EventBridge / Lambda → MediaConvert → S3 → CloudFront
- Step Functionsでのワークフロー管理
- MediaConvert + MediaPackage の組み合わせ

---

## Phase 5: ハンズオン

### 5-1. 基本変換

- MP4入力 → HLS出力（複数ビットレート）
- Terraform / CloudFormation でのリソース定義
- AWS CLI / SDK からのジョブ投入

### 5-2. 実践パイプライン

- S3アップロード → Lambda → MediaConvert → S3 の自動変換パイプライン構築
- EventBridgeでのジョブステータス監視

---

## 学習順序の目安

1. Phase 1（1-2日）— 映像・音声の基礎用語を押さえる
2. Phase 2（2-3日）— コーデック・コンテナを理解する。ここが最重要
3. Phase 3（1-2日）— ストリーミングの仕組みを知る
4. Phase 4（2-3日）— MediaConvertの設定を理解する
5. Phase 5（1-2日）— 手を動かして定着させる
