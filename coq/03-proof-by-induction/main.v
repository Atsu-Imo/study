(* 03 - 帰納法による証明 *)
(* destruct では証明できない性質を、induction で証明する *)

(* ========================================
   1. destruct の限界
   ======================================== *)

(* トピック01で飛ばした定理を思い出そう:
   forall n : nat, n + 0 = n

   0 + n = n は simpl で証明できた（+ の定義が左側を分解するから）。
   しかし n + 0 = n は simpl だけでは証明できない。
   n が具体的な値でないと + の定義を展開できないから。

   destruct n してみると: *)

Theorem plus_n_O_try : forall n : nat, n + 0 = n.
Proof.
  intro n.
  destruct n.
  - (* n = O のケース: O + 0 = O → simpl で解ける *)
    simpl. reflexivity.
  - (* n = S n' のケース: S n' + 0 = S n' *)
    simpl.
    (* ゴール: S (n' + 0) = S n'
       n' + 0 = n' が必要だが、n' は任意のままなので証明できない！ *)
Abort.  (* Abort で証明を中断 *)

(* ========================================
   2. 帰納法 (induction) の登場
   ======================================== *)

(* 帰納法は「すべての自然数について成り立つ」ことを証明する方法。
   高校数学の数学的帰納法と同じ:

   1. n = 0 のとき成り立つことを示す（基底ケース）
   2. n = k のとき成り立つと仮定して、n = k+1 でも成り立つことを示す（帰納ステップ）

   この2つが示せれば、すべての自然数について成り立つ。

   ドミノ倒しのイメージ:
   - 最初の1枚（0）が倒れることを示す
   - 「1枚倒れたら次も倒れる」ことを示す
   - → すべてが倒れる *)

Theorem plus_n_O : forall n : nat, n + 0 = n.
Proof.
  intro n.
  (* induction タクティク: n についての帰納法 *)
  induction n as [| n' IHn'].
  (* [| n' IHn'] は場合分けの変数名を指定する:
     - 基底ケース (O): 変数なし
     - 帰納ステップ (S n'): n' と帰納法の仮定 IHn' *)
  - (* 基底ケース: n = O *)
    (* ゴール: 0 + 0 = 0 *)
    simpl. reflexivity.
  - (* 帰納ステップ: n = S n' *)
    (* 仮定 IHn': n' + 0 = n' （n' のとき成り立つと仮定）*)
    (* ゴール: S n' + 0 = S n' *)
    simpl.
    (* ゴール: S (n' + 0) = S n' *)
    (* IHn' を使って n' + 0 を n' に書き換える *)
    rewrite IHn'.
    reflexivity.
Qed.

(* ========================================
   3. もうひとつの例: 加算の結合法則
   ======================================== *)

(* (a + b) + c = a + (b + c) *)
Theorem plus_assoc : forall a b c : nat, (a + b) + c = a + (b + c).
Proof.
  intros a b c.
  (* a について帰納法 — + の定義が左側を分解するから *)
  induction a as [| a' IHa'].
  - (* 基底ケース: a = 0 *)
    (* (0 + b) + c = 0 + (b + c) → simpl で両辺 b + c になる *)
    simpl. reflexivity.
  - (* 帰納ステップ: a = S a' *)
    (* 仮定 IHa': (a' + b) + c = a' + (b + c) *)
    (* ゴール: (S a' + b) + c = S a' + (b + c) *)
    simpl.
    (* ゴール: S ((a' + b) + c) = S (a' + (b + c)) *)
    rewrite IHa'.
    reflexivity.
Qed.

(* ========================================
   4. 帰納法のパターン
   ======================================== *)

(* n + S m = S (n + m) — + の定義は左を分解するので、
   右側の S を外に出すには帰納法が必要 *)
Theorem plus_n_Sm : forall n m : nat, n + S m = S (n + m).
Proof.
  intros n m.
  induction n as [| n' IHn'].
  - simpl. reflexivity.
  - simpl.
    (* ゴール: S (n' + S m) = S (S (n' + m)) *)
    rewrite IHn'.
    reflexivity.
Qed.

(* ========================================
   5. 帰納法を使った実践例: 加算の交換法則
   ======================================== *)

(* a + b = b + a
   これは plus_n_O と plus_n_Sm を補題として使う *)
Theorem plus_comm : forall a b : nat, a + b = b + a.
Proof.
  intros a b.
  induction a as [| a' IHa'].
  - (* 基底ケース: 0 + b = b + 0 *)
    simpl.
    (* ゴール: b = b + 0 *)
    rewrite plus_n_O.
    reflexivity.
  - (* 帰納ステップ: S a' + b = b + S a' *)
    simpl.
    (* ゴール: S (a' + b) = b + S a' *)
    rewrite plus_n_Sm.
    (* ゴール: S (a' + b) = S (b + a') *)
    rewrite IHa'.
    reflexivity.
Qed.

(* ========================================
   6. 演習（READMEの演習に対応）
   ======================================== *)

(* 演習1 (基礎): 二重加算の定理 *)
(* ヒント: n について induction。simpl と rewrite で進める *)
Theorem plus_n_n_eq_double : forall n : nat, n + n = 2 * n.
Proof.
  (* ここに証明を書く *)
Admitted.

(* 演習2 (応用): 加算の右側を S で包む *)
(* ヒント: a について induction。本文の plus_n_Sm と似たパターン *)
Theorem plus_Sn_m : forall a b : nat, S a + b = S (a + b).
Proof.
  (* ここに証明を書く *)
Admitted.

(* 演習3 (チャレンジ): 乗算の右側の単位元 *)
(* ヒント: n について induction。plus_n_O を補題として使う *)
Theorem mult_n_1 : forall n : nat, n * 1 = n.
Proof.
  (* ここに証明を書く *)
Admitted.
