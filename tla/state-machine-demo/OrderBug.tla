---- MODULE OrderBug ----
\* わざと Ship のガード paid = TRUE を消した「壊れた」版。
\* TLC が「未払いで発送」に到達する反例トレースを出すことを確認するためのもの。
\* Order.tla との差分は Ship の中の paid = TRUE ガード1行だけ。

VARIABLES paid, shipped, cancelled
vars == <<paid, shipped, cancelled>>

TypeOK == /\ paid \in BOOLEAN
          /\ shipped \in BOOLEAN
          /\ cancelled \in BOOLEAN

Init == /\ paid = FALSE
        /\ shipped = FALSE
        /\ cancelled = FALSE

Pay == /\ cancelled = FALSE
       /\ paid' = TRUE
       /\ UNCHANGED <<shipped, cancelled>>

\* ★ ここから paid = TRUE のガードを削除した（バグの再現）
Ship == /\ cancelled = FALSE
        /\ shipped' = TRUE
        /\ UNCHANGED <<paid, cancelled>>

Cancel == /\ shipped = FALSE
          /\ cancelled' = TRUE
          /\ UNCHANGED <<paid, shipped>>

Next == Pay \/ Ship \/ Cancel
Spec == Init /\ [][Next]_vars

Safety == shipped = TRUE => paid = TRUE
====
