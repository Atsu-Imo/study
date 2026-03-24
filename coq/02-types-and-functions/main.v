(* 02 - 帰納型と関数 *)
(* 自分で型を定義し、パターンマッチと再帰関数を書く *)

(* ========================================
   1. 帰納型 (Inductive) — 型を自分で定義する
   ======================================== *)

(* Rocqでは型を Inductive で定義する。
   プログラミングでいう enum や代数的データ型に相当する。 *)

(* 例: 曜日を定義してみよう *)
Inductive day : Type :=
  | monday
  | tuesday
  | wednesday
  | thursday
  | friday
  | saturday
  | sunday.

(* 関数はパターンマッチで定義する *)
Definition is_weekend (d : day) : bool :=
  match d with
  | saturday => true
  | sunday => true
  | _ => false       (* _ はワイルドカード。それ以外すべて *)
  end.

(* 動作確認 *)
Compute is_weekend saturday.   (* = true *)
Compute is_weekend monday.     (* = false *)

(* 定義した関数について証明もできる *)
Theorem saturday_is_weekend : is_weekend saturday = true.
Proof. simpl. reflexivity. Qed.

(* ========================================
   2. bool を自分で定義してみる
   ======================================== *)

(* 標準ライブラリの bool は実はこう定義されている *)
(* 名前が衝突するので my_ をつける *)
Inductive mybool : Type :=
  | mytrue
  | myfalse.

(* 否定関数 *)
Definition mynegb (b : mybool) : mybool :=
  match b with
  | mytrue => myfalse
  | myfalse => mytrue
  end.

(* 論理AND *)
Definition myandb (a b : mybool) : mybool :=
  match a with
  | mytrue => b
  | myfalse => myfalse
  end.

Compute myandb mytrue myfalse.  (* = myfalse *)
Compute mynegb mytrue.          (* = myfalse *)

(* 自分で定義した型でも証明できる *)
Theorem mynegb_involutive : forall b : mybool, mynegb (mynegb b) = b.
Proof.
  intro b.
  destruct b.
  - simpl. reflexivity.
  - simpl. reflexivity.
Qed.

(* ========================================
   3. 自然数 (nat) — 再帰的な帰納型
   ======================================== *)

(* nat は Rocq で最も重要な型のひとつ。
   標準ライブラリではこう定義されている:

   Inductive nat : Type :=
     | O        (* ゼロ *)
     | S (n : nat).  (* n の次の数。Successor の S *)

   つまり:
   0 = O
   1 = S O
   2 = S (S O)
   3 = S (S (S O))

   自然数はゼロと「+1する操作(S)」の繰り返しで表現される。
   これをペアノ数（Peano numbers）という。 *)

Check O.       (* O : nat *)
Check S O.     (* 1 : nat *)
Check S (S O). (* 2 : nat *)
Check 3.       (* Rocqは 3 を S (S (S O)) の略記として扱う *)

(* ========================================
   4. 再帰関数 (Fixpoint)
   ======================================== *)

(* 再帰関数は Fixpoint で定義する。
   Definition は非再帰関数、Fixpoint は再帰関数。 *)

(* 偶数判定 *)
Fixpoint evenb (n : nat) : bool :=
  match n with
  | O => true
  | S O => false
  | S (S n') => evenb n'   (* 2引いて再帰 *)
  end.

Compute evenb 0.   (* = true *)
Compute evenb 1.   (* = false *)
Compute evenb 4.   (* = true *)
Compute evenb 7.   (* = false *)

(* 加算を自分で定義してみる（標準ライブラリの + と同じ定義） *)
Fixpoint myplus (n m : nat) : nat :=
  match n with
  | O => m              (* 0 + m = m *)
  | S n' => S (myplus n' m)  (* (n'+1) + m = (n' + m) + 1 *)
  end.

Compute myplus 2 3.  (* = 5 *)
Compute myplus 0 5.  (* = 5 *)

(* 乗算 *)
Fixpoint mymult (n m : nat) : nat :=
  match n with
  | O => O                     (* 0 * m = 0 *)
  | S n' => myplus m (mymult n' m)  (* (n'+1) * m = m + n' * m *)
  end.

Compute mymult 3 4.  (* = 12 *)

(* ========================================
   5. 再帰関数の証明
   ======================================== *)

(* myplus の左側が 0 のとき *)
Theorem myplus_O_n : forall n : nat, myplus O n = n.
Proof.
  intro n.
  simpl.        (* myplus の定義から O のケースが適用される *)
  reflexivity.
Qed.

(* evenb の簡単な性質 *)
Theorem evenb_S_S : forall n : nat, evenb (S (S n)) = evenb n.
Proof.
  intro n.
  simpl.        (* evenb の定義から S (S n') のケースが適用される *)
  reflexivity.
Qed.

(* ========================================
   6. 演習（READMEの演習に対応）
   ======================================== *)

(* 演習1 (基礎): 階乗関数を定義しよう *)
(* factorial 0 = 1, factorial (S n) = S n * factorial n *)
Fixpoint factorial (n : nat) : nat :=
  match n with
  | O => 1
  | S n' => S n' * factorial n'   (* ここは標準の * を使ってよい *)
  end.

(* テスト: 以下が成り立つことを確認しよう *)
Example test_factorial1 : factorial 3 = 6.
Proof. simpl. reflexivity. Qed.

Example test_factorial2 : factorial 5 = 120.
Proof. simpl. reflexivity. Qed.

Example test_factorial3 : factorial 0 = 1.
Proof. simpl. reflexivity. Qed.

(* 演習2 (応用): 以下の関数を定義し、テストを証明しよう *)
(* n 以下の自然数がすべて偶数かどうかを判定する関数...ではなく、
   n が 0 かどうかを判定する関数 *)
Definition is_zero (n : nat) : bool :=
  match n with
  | O => true
  | S _ => false
  end.

Example test_is_zero1 : is_zero 0 = true.
Proof. simpl. reflexivity. Qed.

Example test_is_zero2 : is_zero 5 = false.
Proof. simpl. reflexivity. Qed.

(* 演習2の本題: 以下を証明しよう *)
(* ヒント: destruct n で場合分け *)
Theorem is_zero_correct : forall n : nat, is_zero n = true -> n = 0.
Proof.
 intros n H.
 destruct n.
  - simpl in H.
   reflexivity.
  - simpl in H.
   discriminate.
Qed.

(* 演習3 (チャレンジ): 以下を証明しよう *)
(* ヒント: destruct n で O と S n' に場合分け。
   S n' のケースでは仮定が矛盾するので discriminate *)
Theorem evenb_0 : evenb 0 = true.
Proof. simpl. reflexivity. Qed.

Theorem S_n_not_zero : forall n : nat, is_zero (S n) = false.
Proof.
  intro n.
  destruct n.
   - simpl. reflexivity.
   - simpl. reflexivity.
Qed.
