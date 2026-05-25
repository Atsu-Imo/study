# Phase 3: ストリーミング配信の仕組み

MediaConvertの主要ユースケースがストリーミング向けの変換であるため、配信の仕組みを理解することが実務では不可欠。

---

## 3-1. 配信プロトコル

### プログレッシブダウンロード vs アダプティブストリーミング

動画をユーザーに届ける方法は大きく2つある。

**プログレッシブダウンロード:**
```
クライアント ← 1本のMP4ファイルを先頭から順にダウンロード ← サーバー

特徴:
  - 単一ファイルを HTTP で配信
  - ダウンロードしながら再生（バッファが溜まれば再生開始）
  - ネットワーク速度が落ちるとバッファ切れ（止まる）
  - 画質は固定（1種類のビットレートしか選べない）
```

**アダプティブストリーミング:**
```
クライアント ← 小さなセグメントを選択的に取得 ← サーバー

特徴:
  - 動画を数秒ごとの「セグメント」に分割
  - 同じシーンを複数の画質（ビットレート）で用意
  - クライアントがネットワーク状況に応じて画質を自動切替
  - バッファ切れが起きにくい
```

**なぜアダプティブストリーミングが主流なのか:**
- モバイル回線のように帯域が不安定な環境で止まらず再生できる
- Wi-Fi → 4G → 3G と回線が変わっても途切れない
- CDN（CloudFront等）との相性が良い（静的ファイルとして配信できる）
- YouTube、Netflix、Amazon Prime Video等、主要サービスはすべてアダプティブストリーミング

---

### HLS（HTTP Live Streaming）

Apple が 2009 年に開発した配信プロトコル。現在最も広くサポートされている。

#### 基本構造

```
HLSの構成:
  マスタープレイリスト（.m3u8）
    ├─ バリアント1: 480p / 1.5 Mbps → メディアプレイリスト（.m3u8）→ セグメント群
    ├─ バリアント2: 720p / 3.0 Mbps → メディアプレイリスト（.m3u8）→ セグメント群
    └─ バリアント3: 1080p / 6.0 Mbps → メディアプレイリスト（.m3u8）→ セグメント群
```

**マスタープレイリスト**（別名: マルチバリアントプレイリスト）は各画質への入口。クライアントはこれを最初に取得して、どの画質が利用可能かを知る。

```m3u8
#EXTM3U
#EXT-X-VERSION:4

#EXT-X-STREAM-INF:BANDWIDTH=1500000,RESOLUTION=854x480,CODECS="avc1.4d401e,mp4a.40.2"
480p/playlist.m3u8

#EXT-X-STREAM-INF:BANDWIDTH=3000000,RESOLUTION=1280x720,CODECS="avc1.4d401f,mp4a.40.2"
720p/playlist.m3u8

#EXT-X-STREAM-INF:BANDWIDTH=6000000,RESOLUTION=1920x1080,CODECS="avc1.640028,mp4a.40.2"
1080p/playlist.m3u8
```

**メディアプレイリスト**は特定の画質のセグメント一覧。

```m3u8
#EXTM3U
#EXT-X-VERSION:4
#EXT-X-TARGETDURATION:6
#EXT-X-MEDIA-SEQUENCE:0

#EXTINF:6.006,
segment_000.ts
#EXTINF:6.006,
segment_001.ts
#EXTINF:6.006,
segment_002.ts
#EXTINF:4.004,
segment_003.ts

#EXT-X-ENDLIST
```

#### セグメントの形式

HLS のセグメントには2つの形式がある。

| セグメント形式 | 拡張子 | 特徴 |
|--------------|--------|------|
| MPEG-TS | .ts | 従来の標準。互換性が高い |
| fMP4 | .mp4 / .m4s | Apple が 2016 年から対応。DASH と共通化可能（CMAF） |

**現在の推奨は fMP4。** MPEG-TS はレガシーだが、古いデバイスのサポートが必要なら残す。

#### 再生の流れ

```
1. クライアントがマスタープレイリスト（master.m3u8）を取得
2. 利用可能な帯域を推定し、最適なバリアントを選択
3. 選択したバリアントのメディアプレイリストを取得
4. セグメントを順番にダウンロード → 再生
5. ネットワーク状況が変化 → 別のバリアントに切替
   （例: 帯域が下がったら 1080p → 720p に自動切替）
```

#### HLSの特徴まとめ

- **Apple 由来** — iOS / Safari でのサポートが最も確実
- **広い互換性** — Android、各種ブラウザ、スマートTV でも対応
- **暗号化** — AES-128 暗号化、FairPlay DRM 対応
- **ライブ配信対応** — プレイリストを動的に更新することで実現
- **CDN 親和性** — すべてが HTTP ベースの静的ファイル

---

### DASH（Dynamic Adaptive Streaming over HTTP）

MPEG が策定した**国際標準**（ISO/IEC 23009-1）。ベンダー非依存。

#### 基本構造

```
DASHの構成:
  MPD（Media Presentation Description、.mpd）
    ├─ Period（時間区間）
    │   ├─ AdaptationSet（映像）
    │   │   ├─ Representation 1: 480p / 1.5 Mbps → セグメント群
    │   │   ├─ Representation 2: 720p / 3.0 Mbps → セグメント群
    │   │   └─ Representation 3: 1080p / 6.0 Mbps → セグメント群
    │   └─ AdaptationSet（音声）
    │       ├─ Representation: AAC 128kbps（日本語）
    │       └─ Representation: AAC 128kbps（英語）
    └─ Period 2（広告挿入など）
```

**MPD（マニフェスト）** は HLS のマスタープレイリストに相当。XML 形式。

```xml
<?xml version="1.0" encoding="UTF-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011"
     type="static"
     mediaPresentationDuration="PT1H30M">
  <Period>
    <AdaptationSet mimeType="video/mp4" segmentAlignment="true">
      <Representation id="480p" bandwidth="1500000" width="854" height="480"
                      codecs="avc1.4d401e">
        <SegmentTemplate media="480p/seg_$Number$.m4s"
                         initialization="480p/init.mp4"
                         duration="6000" timescale="1000"/>
      </Representation>
      <Representation id="720p" bandwidth="3000000" width="1280" height="720"
                      codecs="avc1.4d401f">
        <SegmentTemplate media="720p/seg_$Number$.m4s"
                         initialization="720p/init.mp4"
                         duration="6000" timescale="1000"/>
      </Representation>
    </AdaptationSet>
    <AdaptationSet mimeType="audio/mp4" lang="ja">
      <Representation id="audio_ja" bandwidth="128000" codecs="mp4a.40.2">
        <SegmentTemplate media="audio_ja/seg_$Number$.m4s"
                         initialization="audio_ja/init.mp4"
                         duration="6000" timescale="1000"/>
      </Representation>
    </AdaptationSet>
  </Period>
</MPD>
```

#### DASHの用語整理

| DASH の用語 | HLS での対応概念 | 説明 |
|------------|----------------|------|
| MPD | マスタープレイリスト | 配信全体の構造定義 |
| Period | — | 時間的な区切り（広告挿入の境界等） |
| AdaptationSet | — | メディアタイプ（映像/音声/字幕）のグループ |
| Representation | バリアント | 特定の画質・ビットレート |
| Segment | セグメント | 数秒ごとのメディア断片 |

#### セグメントの形式

DASH は当初から **fMP4** を前提としている（MPEG-TS は使わない）。

構成:
```
初期化セグメント（init.mp4）: コーデック情報、トラック情報
  ↓ 1回だけ取得
メディアセグメント（seg_1.m4s, seg_2.m4s, ...）: 実際の映像・音声データ
  ↓ 順番に取得
```

#### DASHの特徴まとめ

- **国際標準** — ベンダーロックインなし
- **DRM 連携が強力** — Widevine、PlayReady を標準サポート（CENC）
- **Period で時間分割** — 広告挿入やマルチコンテンツ切替が自然にできる
- **映像と音声が分離** — 多言語音声の切替がクリーン
- **ブラウザ対応** — Chrome、Firefox、Edge（MSE経由）。Safari は非対応（HLS を使う）

---

### HLS vs DASH 比較

| 項目 | HLS | DASH |
|------|-----|------|
| 策定元 | Apple | MPEG（国際標準） |
| マニフェスト形式 | .m3u8（テキスト） | .mpd（XML） |
| セグメント形式 | .ts または .fmp4 | .m4s（fMP4） |
| Apple デバイス | ネイティブ対応 | 非対応 |
| Android / Chrome | 対応（MSE経由） | 対応（MSE経由） |
| DRM | FairPlay, AES-128 | Widevine, PlayReady（CENC） |
| ライブ配信 | 対応 | 対応 |
| 広告挿入 | やや複雑 | Period で自然に対応 |

**実務的な選択:**
- **Apple デバイスが重要 → HLS は必須**
- **幅広いDRM対応 → DASH が有利**
- **両方出す → CMAF で統合**（後述）

---

### CMAF（Common Media Application Format）

HLS と DASH の**セグメント形式を統一**するための規格（ISO/IEC 23000-19）。2018年に策定。

#### なぜ CMAF が必要か

HLS と DASH の両方をサポートしたい場合、従来は**同じ映像を2通りにエンコード**する必要があった。

```
従来（CMAF なし）:
  元の映像 → HLS用エンコード（.ts セグメント） → S3に保存
           → DASH用エンコード（.m4s セグメント）→ S3に保存

  ストレージ: 2倍
  エンコード時間: 2倍
  コスト: 2倍
```

```
CMAF あり:
  元の映像 → CMAF エンコード（fMP4 セグメント）→ S3に保存
           → HLS マニフェスト（.m3u8）生成
           → DASH マニフェスト（.mpd）生成

  セグメントは共通、マニフェストだけ別
  ストレージ: 約1倍
  エンコード時間: 約1倍
```

#### CMAF の仕組み

**核心: fMP4 セグメントを HLS と DASH で共有する。**

```
CMAF の出力:
  output/
    ├─ video/
    │   ├─ 480p/
    │   │   ├─ init.mp4          ← 初期化セグメント
    │   │   ├─ segment_0001.m4s  ← メディアセグメント（HLSとDASHで共有）
    │   │   ├─ segment_0002.m4s
    │   │   └─ ...
    │   ├─ 720p/
    │   │   └─ ...
    │   └─ 1080p/
    │       └─ ...
    ├─ audio/
    │   └─ ...
    ├─ master.m3u8              ← HLS用マニフェスト（セグメントは上記を参照）
    └─ manifest.mpd             ← DASH用マニフェスト（セグメントは上記を参照）
```

#### CMAF + Low Latency

CMAF は **CMAF Chunk** という仕組みで低遅延配信にも対応。

```
従来のセグメント:
  [====== 6秒 ======]  ← セグメント全体が完成するまで配信できない
                         → 最小遅延は6秒+α

CMAF Chunk:
  [==][==][==][==][==][==]  ← セグメントを更に細分化（例: 1秒ごと）
   ↑ 最初のチャンクが完成次第すぐ配信
   → 最小遅延は1秒+α
```

Apple の LL-HLS（Low Latency HLS）や DASH の Low Latency DASH はこの仕組みを使っている。

#### MediaConvert での CMAF

MediaConvertでは **CMAF Output Group** を選択すると、fMP4 セグメント + HLS/DASH 両方のマニフェストを1回のジョブで生成できる。

**CMAF まとめ:**
- HLS と DASH のセグメントを統一 → ストレージ・エンコードコスト削減
- fMP4 ベース
- 低遅延配信（CMAF Chunk）にも対応
- **新規プロジェクトなら CMAF を第一選択にすべき**

---

## 3-2. ABR（Adaptive Bitrate）

### ABR の仕組み

**ABR = ネットワーク状況に応じて、再生中に画質を自動的に切り替える仕組み。**

```
視聴者の帯域が変化する例:

時間 →
帯域
 10 Mbps |  ████
  8 Mbps |  ████████
  6 Mbps |  ████████████
  4 Mbps |              ████████████
  2 Mbps |                          ████████
  1 Mbps |                                  ████████████

再生される画質:
         → 1080p → 1080p → 720p → 480p → 360p
         （帯域に応じてシームレスに切替）
```

**切替の仕組み:**
1. プレイヤーがセグメントをダウンロードする速度を計測
2. 次のセグメントを要求する際、帯域に見合った画質を選択
3. セグメント境界で画質が切り替わる（途中で変わるわけではない）

セグメント境界で切り替えるため、各画質のセグメントは**同じ位置にキーフレーム（IDRフレーム）**を持つ必要がある。これが Phase 2 で学んだ GOP 設定とつながる。

```
GOPアライメント:
  1080p: [I---P---P---][I---P---P---][I---P---P---]
  720p:  [I---P---P---][I---P---P---][I---P---P---]
  480p:  [I---P---P---][I---P---P---][I---P---P---]
          ↑ セグメント境界      ↑ セグメント境界
          すべての画質で同じ位置にIフレーム
```

MediaConvert は同一ジョブ内で複数出力を生成する際、自動で GOP アライメントを揃えてくれる。

---

### エンコードラダーの設計

**エンコードラダー = ABR 配信で用意する「解像度 × ビットレート」の組み合わせリスト。**

「ラダー（はしご）」と呼ぶのは、低画質から高画質まで段階的に並べるため。

#### Apple 推奨のエンコードラダー（HLS 向け、参考値）

| 解像度 | ビットレート | fps | 用途 |
|--------|------------|-----|------|
| 416×234 | 145 kbps | 30 | 極低帯域（2G回線） |
| 640×360 | 365 kbps | 30 | 低帯域 |
| 768×432 | 730 kbps | 30 | モバイル |
| 768×432 | 1,100 kbps | 30 | モバイル（高画質） |
| 960×540 | 2,000 kbps | 30 | 標準 |
| 1280×720 | 3,000 kbps | 30 | HD |
| 1280×720 | 4,500 kbps | 30 | HD（高画質） |
| 1920×1080 | 6,000 kbps | 30 | Full HD |
| 1920×1080 | 7,800 kbps | 30 | Full HD（高画質） |

#### ラダー設計の指針

**ビットレートの刻み方:**
- 各段の間隔はおよそ1.5〜2倍
- 間隔が狭すぎると切替が頻発して視聴体験が悪い
- 間隔が広すぎると帯域の変化に追従できない

**解像度とビットレートの関係:**
- 解像度を上げてもビットレートが不足すると、低解像度+十分なビットレートのほうが画質が良い
- 例: 1080p / 2 Mbps よりも 720p / 2 Mbps のほうがきれいに見える
- これを「解像度が高すぎてビットレートが足りない状態」と呼ぶ

```
ビットレートが同じ場合の見え方:

720p / 3 Mbps  → 十分なビットレート → きれい
1080p / 3 Mbps → ビットレート不足   → ブロックノイズが見える
```

**最低ラングを忘れない:**
- 極低帯域（200〜500 kbps）のラングを用意しないと、ネットワークが悪い環境で完全に止まる
- 見た目が悪くても「止まるよりマシ」

---

### Per-Title Encoding / コンテンツ適応エンコード

固定のエンコードラダーには問題がある。

```
問題:
  アクション映画（動き多い、複雑） → 5 Mbps でも画質不足
  プレゼン資料の動画（ほぼ静止画） → 1 Mbps でも十分

  同じラダーを使うと:
  - アクション映画には足りない
  - プレゼン動画には無駄
```

**Per-Title Encoding（コンテンツ別最適化）:**
- コンテンツの複雑さを事前に解析
- コンテンツごとに最適なラダーを自動生成
- 複雑なシーンが多い映像は高ビットレート寄り
- 単純な映像は低ビットレートで十分な品質

Netflix がこの手法を先駆けて導入し、帯域コストを大幅に削減した。

**MediaConvert での対応:**
- MediaConvert自体にPer-Title Encodingの自動機能はないが、**QVBR**（Quality-defined VBR）を使うことで、コンテンツの複雑さに応じてビットレートが自動調整される
- 厳密なPer-Title Encodingをするなら、事前にコンテンツを解析して動的にジョブパラメータを変えるパイプラインを組む必要がある

---

## 3-3. DRM（デジタル著作権管理）

### DRM の概要と目的

**DRM = コンテンツの不正コピー・再配布を防ぐための技術的保護手段。**

```
DRMなし:
  サーバー → 平文の動画セグメント → クライアント
  → ダウンロードして自由にコピー可能

DRMあり:
  サーバー → 暗号化された動画セグメント → クライアント
  → 復号キーがないと再生できない
  → キーはDRMシステムが管理（保護されたメモリ領域に保管）
  → コピーしても暗号化されたまま
```

**DRM が必要なケース:**
- 有料コンテンツ（映画、ドラマ、スポーツのライブ配信）
- ライセンス契約でDRM必須とされるコンテンツ（映画スタジオの要求）
- コンテンツの保護レベルが厳格に求められるケース

**DRM が不要なケース:**
- 無料コンテンツ（YouTube の無料動画等）
- 広告収入モデル（見てもらうことが最優先）
- 社内向け動画（別の手段でアクセス制御）

---

### 3大 DRM システム

業界で事実上の標準となっている DRM は3つ。

| DRM | 開発元 | 対応プラットフォーム |
|-----|--------|-------------------|
| Widevine | Google | Chrome, Android, Chromecast, Firefox |
| FairPlay | Apple | Safari, iOS, macOS, Apple TV |
| PlayReady | Microsoft | Edge, Windows, Xbox, 多くのスマートTV |

**なぜ3つもあるのか:**
- 各プラットフォームベンダーが自社のDRMを推進
- ユニバーサルなDRMは存在しない
- **すべてのデバイスをカバーするには、複数のDRMを組み合わせる必要がある**

#### デバイスカバレッジの例

```
視聴者のデバイス     必要な DRM
─────────────────────────────
iPhone / Safari   → FairPlay
Android / Chrome  → Widevine
Windows / Edge    → PlayReady（または Widevine）
macOS / Chrome    → Widevine
スマートTV        → PlayReady または Widevine
```

#### DRM のセキュリティレベル

Widevine を例にすると、セキュリティレベルが3段階ある。

| レベル | 処理場所 | 用途 |
|--------|---------|------|
| L1 | ハードウェア（TEE） | HD / 4K コンテンツ。映画スタジオが要求 |
| L2 | ソフトウェア+部分ハードウェア | あまり使われない |
| L3 | ソフトウェアのみ | SD コンテンツ。デスクトップブラウザは通常これ |

映画スタジオのコンテンツを4Kで配信するには **L1 が必須** という契約条件がつくことが多い。L3デバイスには SD画質だけ配信する、という制御が必要になる。

---

### CENC（Common Encryption）

**CENC = 暗号化方式を統一し、1回の暗号化で複数のDRMに対応する仕組み。**（ISO/IEC 23001-7）

```
CENCなし:
  元の映像 → FairPlay 用に暗号化 → Apple 向け配信
           → Widevine 用に暗号化 → Android/Chrome 向け配信
           → PlayReady 用に暗号化 → Windows 向け配信
  3回暗号化、3倍のストレージ

CENCあり:
  元の映像 → CENC で1回暗号化 → 全デバイスで共通のセグメント
           → DRM ごとのライセンスサーバーが復号キーを管理
  1回の暗号化、1倍のストレージ
```

**暗号化方式:**
- **CTR（Counter Mode）** — Widevine、PlayReady が使用
- **CBC（Cipher Block Chaining）** — FairPlay が使用
- **CBCS（CBC with Subsample encryption）** — Apple が 推奨、Widevine/PlayReady も対応

> FairPlay は厳密には CENC とは別体系だが、CBCS 暗号化方式を使うことで CMAF + 複数DRM の共存が実現できる。

---

### MediaConvert での DRM 設定（SPEKE 連携）

MediaConvert 自体は DRM のライセンスサーバーを持っていない。外部の DRM プロバイダーと **SPEKE（Secure Packager and Encoder Key Exchange）** プロトコルで連携する。

```
DRM付き配信のフロー:

1. MediaConvert がジョブ実行
2. MediaConvert → SPEKE API → DRM キープロバイダー（例: PallyCon, BuyDRM, EZDRM）
3. キープロバイダーが暗号化キーを返却
4. MediaConvert が映像を暗号化してセグメントを生成
5. 暗号化されたセグメントを S3 に出力

再生時:
1. プレイヤーが暗号化セグメントを取得
2. プレイヤー → DRM ライセンスサーバー（キープロバイダー）にキーを要求
3. ライセンスサーバーが認証・認可後にキーを返却
4. プレイヤーがキーでセグメントを復号 → 再生
```

**SPEKE v2:**
- SPEKE v1 は暗号化方式（CTR/CBC）をDRMごとに個別指定
- SPEKE v2 は CPIX（Content Protection Information Exchange）ベースで、マルチDRM設定を統一的に管理
- MediaConvert は SPEKE v1 / v2 両方に対応

**MediaConvert でのDRM設定箇所:**
- Output Group の設定で「DRM encryption」を有効化
- SPEKE エンドポイント（キープロバイダーの API URL）を指定
- Resource ID（コンテンツ識別子）を指定
- System ID（使用する DRM を識別する GUID）を指定

---

## 3-4. 字幕・キャプション

### 焼き込み字幕 vs サイドカー字幕

字幕の扱いは大きく2つに分かれる。

**焼き込み字幕（Burn-in / Open Caption）:**
```
映像フレーム自体に字幕テキストを画像として合成

メリット:
  - 確実に表示される（プレイヤーの対応不要）
  - フォント・位置が完全に制御できる

デメリット:
  - 消せない（ON/OFF できない）
  - 多言語対応が困難（言語ごとに映像を用意する必要がある）
  - エンコード後に修正できない
```

**サイドカー字幕（Closed Caption / Sidecar）:**
```
映像とは別のファイルまたはトラックとして字幕を保持

メリット:
  - ON/OFF 切替可能
  - 複数言語を1つの映像で提供可能
  - 後から修正・追加可能

デメリット:
  - プレイヤーが字幕形式に対応している必要がある
  - 表示の見え方がプレイヤー依存
```

**実務的な選択:**
- 基本は**サイドカー字幕**（柔軟性が圧倒的に高い）
- 規制要件やデバイス互換性の理由で焼き込みが必要な場合のみ焼き込み

---

### 主要な字幕フォーマット

| フォーマット | 形式 | 主な用途 |
|-------------|------|---------|
| SRT | テキスト | 最もシンプル。タイムスタンプ + テキスト |
| WebVTT | テキスト | Web 標準。HTML5 の `<track>` で使用。SRT の進化形 |
| TTML | XML | 放送・配信の業界標準。スタイル指定が豊富 |
| CEA-608 | データ | 北米の放送向け。映像ストリームに埋め込み |
| CEA-708 | データ | CEA-608 の後継。HD放送向け |

#### SRT（SubRip）

```srt
1
00:00:01,000 --> 00:00:04,000
これは最初の字幕です。

2
00:00:05,000 --> 00:00:08,000
これは2番目の字幕です。
```

- 最もシンプル。番号 + タイムスタンプ + テキスト
- スタイル指定はできない（フォント、色、位置はプレイヤー依存）
- 入力ソースとしてよく使われるが、配信には WebVTT のほうが適切

#### WebVTT（Web Video Text Tracks）

```vtt
WEBVTT

00:00:01.000 --> 00:00:04.000
これは最初の字幕です。

00:00:05.000 --> 00:00:08.000 position:10% align:left
<b>太字</b>の字幕も可能。
```

- W3C 標準
- HTML タグでスタイル指定可能（太字、斜体、色など）
- CSS で表示をカスタマイズ可能
- HLS / DASH の字幕トラックとして広く使われる

#### TTML（Timed Text Markup Language）

```xml
<?xml version="1.0" encoding="UTF-8"?>
<tt xml:lang="ja" xmlns="http://www.w3.org/ns/ttml">
  <body>
    <div>
      <p begin="00:00:01.000" end="00:00:04.000">
        これは最初の字幕です。
      </p>
      <p begin="00:00:05.000" end="00:00:08.000">
        これは2番目の字幕です。
      </p>
    </div>
  </body>
</tt>
```

- XML ベースで非常に豊富なスタイル指定
- DASH（TTML in fMP4）、Netflix 等の配信サービスで採用
- IMSC（Internet Media Subtitles and Captions）は TTML のサブセットで、配信向けのプロファイル

#### CEA-608 / CEA-708

- 北米の放送規格に基づく字幕（Closed Captioning）
- 映像ストリーム内にデータとして埋め込まれる
- FCC（米国連邦通信委員会）規制で米国向け放送・配信にはClosed Captioning が必須
- MediaConvert は CEA-608 / 708 の抽出・挿入・変換に対応

---

### 配信プロトコルと字幕形式の対応

| プロトコル | 推奨字幕形式 | 配信方法 |
|-----------|-------------|---------|
| HLS | WebVTT | 別ファイル（.vtt）をプレイリストで参照 |
| DASH | TTML in fMP4 または WebVTT | AdaptationSet として定義 |
| CMAF | WebVTT または IMSC（TTML サブセット） | 別トラックとして管理 |

**HLS での字幕参照例（マスタープレイリスト）:**

```m3u8
#EXTM3U

#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID="subs",LANGUAGE="ja",NAME="Japanese",
  DEFAULT=YES,AUTOSELECT=YES,URI="subtitles/ja/playlist.m3u8"

#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID="subs",LANGUAGE="en",NAME="English",
  DEFAULT=NO,AUTOSELECT=NO,URI="subtitles/en/playlist.m3u8"

#EXT-X-STREAM-INF:BANDWIDTH=6000000,RESOLUTION=1920x1080,SUBTITLES="subs"
1080p/playlist.m3u8
```

---

### MediaConvert での字幕の扱い

**入力側:**
- Input の Caption Selectors で字幕ソースを指定
- 対応入力形式: SRT, SCC, STL, TTML, 埋め込み（CEA-608/708）、SMI 等

**出力側:**
- Output の Caption で出力形式を指定
- 焼き込み（Burn-in）: 映像に直接合成
- サイドカー: 別ファイルとして出力
- 埋め込み: セグメントまたはトラック内に格納

**よくある変換パターン:**

| 入力 | 出力 | 用途 |
|------|------|------|
| SRT → WebVTT | HLS 配信 | 最も一般的 |
| SRT → Burn-in | SNS 向け動画 | スマホでの視聴（字幕ON/OFF不可環境） |
| CEA-608 → CEA-708 | 放送→配信変換 | 規制対応 |
| TTML → WebVTT | DASH → HLS 変換 | プロトコル変更時 |

---

## まとめ: Phase 3 で押さえたこと

| 概念 | MediaConvert での関連箇所 |
|------|------------------------|
| HLS / DASH / CMAF | Output Group Type の選択 |
| セグメント長 | Segment length 設定（通常 6 秒）。GOP と合わせる |
| ABR ラダー | 1つの Output Group 内に複数 Output（解像度×ビットレート） |
| DRM | Output Group の暗号化設定 + SPEKE エンドポイント |
| 字幕 | Input の Caption Selectors + Output の Caption 設定 |

**実務的な判断フロー:**

```
新規 VOD 配信プロジェクト:
  1. CMAF Output Group を選択（HLS + DASH 両対応）
  2. fMP4 セグメント、6秒間隔
  3. エンコードラダーを設計（最低 3〜5 段）
  4. 有料コンテンツなら DRM を SPEKE 連携で設定
  5. 字幕は WebVTT でサイドカー出力
  6. S3 → CloudFront で CDN 配信
```

---

次のステップ: [Phase 4: AWS MediaConvert 自体の理解](./04_mediaconvert.md) — サービスの設定と使い方へ。
