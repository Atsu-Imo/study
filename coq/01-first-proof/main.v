(* 01 - はじめての証明 *)
(* Coqの基本的な使い方と、最初の証明を体験する *)

(* ========================================
   1. Coqは「型」の世界
   ======================================== *)

(* Coqでは、値だけでなく「証明」も型を持つ。
   まずは簡単な値の定義から始めよう。 *)

(* 自然数の定義（Coqには標準ライブラリにnatがある） *)
Check 0.       (* 0 : nat *)
Check 1 + 2.   (* 1 + 2 : nat *)
Check true.    (* true : bool *)

(* Compute で式を評価できる *)
Compute 1 + 2.   (* = 3 : nat *)
Compute 3 * 4.   (* = 12 : nat *)
Compute negb true.  (* = false : bool *)

(* ========================================
   2. 最初の定理と証明
   ======================================== *)

(* 「1 + 1 = 2」を証明してみよう *)
Theorem one_plus_one : 1 + 1 = 2.
Proof.
  (* simpl タクティク: 式を簡約（計算）する *)
  simpl.
  (* reflexivity タクティク: 左辺と右辺が同じなら証明完了 *)
  reflexivity.
Qed.

(* もっと簡単に: simpl を省略しても reflexivity だけで証明できる *)
Theorem two_plus_three : 2 + 3 = 5.
Proof.
  reflexivity.
Qed.

(* ========================================
   3. 全称量化 (forall) を使った証明
   ======================================== *)

(* 「任意の自然数 n について、n + 0 = n」 *)
(* これは一見簡単だが、+ の定義上 simpl だけでは解けない *)
Theorem plus_O_n : forall n : nat, 0 + n = n.
Proof.
  (* intro タクティク: forall で束縛された変数を導入する *)
  intro n.
  (* 0 + n は定義上 n に簡約される *)
  simpl.
  reflexivity.
Qed.

(* ========================================
   4. intros と rewrite
   ======================================== *)

(* 「任意の n m について、n = m ならば n + n = m + m」 *)
Theorem plus_eq : forall n m : nat, n = m -> n + n = m + m.
Proof.
  (* intros: 複数の仮定を一度に導入 *)
  intros n m H.
  (* rewrite: 仮定 H (n = m) を使って n を m に書き換える *)
  rewrite H.
  reflexivity.
Qed.

(* ========================================
   5. bool に関する証明
   ======================================== *)

(* 「二重否定は元に戻る」 *)
(* negb は bool の否定関数 (not bool の略)。negb true = false, negb false = true *)
(* つまり !!b = b をすべての bool b について証明する *)
Theorem negb_involutive : forall b : bool, negb (negb b) = b.
Proof.
  (* destruct タクティク: b が true か false かで場合分け *)
  intro b.
  destruct b.
  - (* b = true の場合 *)
    simpl. reflexivity.
  - (* b = false の場合 *)
    simpl. reflexivity.
Qed.

(* ========================================
   6. 演習（READMEの演習に対応）
   ======================================== *)

(* 演習1 (基礎): orb false b = b を証明しよう *)
(* orb は bool の論理OR関数。orb true _ = true, orb false b = b *)
(* ヒント: negb_involutive と同じ要領。simpl と reflexivity で十分 *)
Theorem orb_false_l : forall b : bool, orb false b = b.
Proof.
  simpl.
  reflexivity.
Qed.

(* 演習2 (応用): orb の可換性を証明しよう *)
(* ヒント: destruct で場合分けが必要 *)
Theorem orb_commutative : forall a b : bool, orb a b = orb b a.
Proof.
  (* ここに証明を書く *)
Admitted.

(* 演習3 (チャレンジ): andb a b = orb a b ならば a = b を証明しよう *)
(* ヒント: まず a で destruct、次に b で destruct。
   矛盾するケースでは simpl in H で仮定を簡約し、discriminate を使う *)
Theorem andb_eq_orb :
  forall b c : bool, andb b c = orb b c -> b = c.
Proof.
  (* ここに証明を書く *)
Admitted.
