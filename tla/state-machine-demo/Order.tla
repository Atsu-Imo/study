---- MODULE Order ----
\* 注文の状態機械を TLA+ で書いたもの。
\* coq/state-machine-safety/main.v と同じ題材を「手証明なし・モデル検査」で検証する。
\*
\* 対応関係（Rocq → TLA+）:
\*   Record State の3フィールド → VARIABLES paid, shipped, cancelled
\*   Definition init           → Init
\*   step_pay / ship / cancel  → Pay / Ship / Cancel
\*   step（許される遷移の総体） → Next
\*   reachable                 → Spec (Init から始まり毎ステップ Next)
\*   invariant / ~bad / safety → Safety (TLC に検査させる不変条件)

VARIABLES paid, shipped, cancelled
vars == <<paid, shipped, cancelled>>

\* 各変数が bool であること（Rocq の型に相当）
TypeOK == /\ paid \in BOOLEAN
          /\ shipped \in BOOLEAN
          /\ cancelled \in BOOLEAN

\* 初期状態（Rocq の Definition init）
Init == /\ paid = FALSE
        /\ shipped = FALSE
        /\ cancelled = FALSE

\* 遷移規則。
\*   ' (プライム) = 「次の状態での値」。Rocq の mkState で新状態を作るのに相当。
\*   UNCHANGED <<...>> = 変わらないフィールドの明示（書き忘れると任意の値になる！）。

\* 支払い：キャンセル済みでなければ paid を立てる
Pay == /\ cancelled = FALSE          \* ガード
       /\ paid' = TRUE
       /\ UNCHANGED <<shipped, cancelled>>

\* 発送：支払い済み かつ 未キャンセル のときだけ shipped を立てる
Ship == /\ paid = TRUE               \* ガード（安全性の肝）
        /\ cancelled = FALSE         \* ガード
        /\ shipped' = TRUE
        /\ UNCHANGED <<paid, cancelled>>

\* キャンセル：未発送なら cancelled を立てる
Cancel == /\ shipped = FALSE         \* ガード
          /\ cancelled' = TRUE
          /\ UNCHANGED <<paid, shipped>>

Next == Pay \/ Ship \/ Cancel        \* Rocq の step（許される遷移の総体）

Spec == Init /\ [][Next]_vars        \* Init から始まり毎ステップ Next（reachable に相当）

\* 安全性: 発送済みなら必ず支払い済み（= bad「未払い発送」に到達しない）
Safety == shipped = TRUE => paid = TRUE
====
