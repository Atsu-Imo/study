# Quint サンプル: tic-tac-toe

Quint 公式の examples から持ってきた三目並べ（tic-tac-toe）の仕様。
TLA+ サンプル（`../../tla/`）との比較で Quint の書き味とツールを体験するための教材。

- 出典: https://github.com/informalsystems/quint/tree/main/examples/games/tictactoe
- 元ネタ: https://elliotswart.github.io/pragmaticformalmodeling/

## モデルの中身

- **X は常に最善手**（勝てるなら勝つ → 相手の勝ちを防ぐ → 中央 → 勝ち筋を作る → 残りに置く、の優先順位）。
- **O はランダムな合法手**（`MoveToEmpty` で空きマスを `oneOf` で非決定的に選ぶ）。
- 盤面 `board: int -> (int -> Square)`、手番 `nextTurn` の2つが状態変数。
- `step` で毎ステップ X か O が1手指す。決着後は状態を据え置き。

## 不変条件（invariant）

| 名前 | 意味 | 成立するか |
|---|---|---|
| `OHasNotWon` | O は勝てない | **成立**（O は最善の X 相手に引き分けが上限） |
| `XHasNotWon` | X は勝てない | 成立しない（X はだいたい勝つ） |
| `NotStalemate` | 引き分けにならない | 成立しない（O が上手く動くと引き分け→反例が出る） |

`val inv = OHasNotWon` がデフォルトの検査対象。

## 動かし方

```bash
# インストール（未導入なら）
npm i -g @informalsystems/quint

# 1) 型検査（TLA+ には無い段階。書いた時点で型が合うか見る）
quint typecheck tictactoe.qnt

# 2) シミュレータで乱択実行 — O は勝てない（反例なし）
quint run tictactoe.qnt --invariant=OHasNotWon

# 3) わざと成立しない不変条件 — 引き分け盤面が反例として出る
quint run tictactoe.qnt --invariant=NotStalemate

# 4) 対話的に状態遷移を試す
quint repl
#   >>> init      （初期状態をセット）
#   >>> step      （1手進める。何度も叩くと進行が見える）
#   >>> board     （現在の盤面を表示）
```

### 網羅的検査（任意・要 Apalache）

`quint run` は乱択シミュレーションで「速く浅く」反例を探す。
網羅的に保証したい場合は Apalache バックエンドで:

```bash
quint verify tictactoe.qnt --invariant=OHasNotWon
```

（Apalache の別途インストールが必要。まずは `run` で十分体感できる）

## TLA+ と見比べるポイント

- `\A` / `\E` → `.forall(...)` / `.exists(...)` のメソッド風。
- 次状態のプライム記法 `board'` は TLA+ と同じ。
- `nondet x = S.oneOf()` で非決定的選択（TLA+ の `\E x \in S` に相当する手番選択）。
- `match` による代数的データ型（`Square = Occupied(Player) | Empty`）が使える。
- `temporal XMustEventuallyWin = eventually(won(X))` は時相性プロパティ（成立しない例）。
