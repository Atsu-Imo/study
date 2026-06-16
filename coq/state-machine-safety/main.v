(* 状態機械の安全性証明 — 「危険な状態には決して到達しない」 *)
(*
   実務例：注文(order)の状態機械をモデル化し、
   「支払っていないのに発送済み」という危険状態に
   絶対に到達しないことを証明する。

   ※ これはカリキュラム 09〜11 (ミニ言語/ホーア論理) 相当の
     先取りプレビュー。Inductive な命題と帰納法を使う。
*)

(* ========================================
   1. 状態の定義
   ======================================== *)

(* 注文の状態を3つのフラグで表現する。
   Record は Go の struct に相当する。 *)
Record State := mkState {
  paid      : bool;   (* 支払い済みか *)
  shipped   : bool;   (* 発送済みか *)
  cancelled : bool    (* キャンセル済みか *)
}.

(* 初期状態：何もしていない注文 *)
Definition init : State := mkState false false false.

(* ========================================
   2. 遷移規則（許される状態変化）
   ======================================== *)

(* step s s' = 「状態 s から s' への遷移が許される」という命題。
   Inductive で「許可された遷移だけ」を列挙する。
   ここに書かれていない遷移は決して起こらない、という点が重要。 *)
Inductive step : State -> State -> Prop :=
  (* 支払い：キャンセル済みでなければ paid を立てる *)
  | step_pay : forall s,
      cancelled s = false ->
      step s (mkState true (shipped s) (cancelled s))
  (* 発送：支払い済み かつ 未キャンセル のときだけ shipped を立てる。
     ← この「ガード(前提条件)」が安全性の肝。 *)
  | step_ship : forall s,
      paid s = true ->
      cancelled s = false ->
      step s (mkState (paid s) true (cancelled s))
  (* キャンセル：未発送なら cancelled を立てる *)
  | step_cancel : forall s,
      shipped s = false ->
      step s (mkState (paid s) (shipped s) true).

(* ========================================
   3. 到達可能性
   ======================================== *)

(* reachable s = 「初期状態から有限回の遷移で s に到達できる」 *)
Inductive reachable : State -> Prop :=
  | reach_init : reachable init
  | reach_step : forall s s',
      reachable s -> step s s' -> reachable s'.

(* ========================================
   4. 危険な状態と、守りたい不変条件
   ======================================== *)

(* bad = 「支払っていないのに発送済み」= 絶対に避けたい状態 *)
Definition bad (s : State) : Prop :=
  shipped s = true /\ paid s = false.

(* invariant = 「発送済みなら必ず支払い済み」= 常に保ちたい性質。
   これは bad の否定にあたる。 *)
Definition invariant (s : State) : Prop :=
  shipped s = true -> paid s = true.

Definition invariant2 (s: State) : Prop :=
  cancelled s = true -> shipped s = false.

(* ========================================
   5. 安全性の証明
   ========================================
   戦略 (帰納法による不変条件の証明):
     (1) 初期状態は不変条件を満たす          … init_inv
     (2) どの1ステップ遷移も不変条件を保つ    … step_preserves_inv
     (3) ゆえに到達可能な全状態が満たす        … reachable_inv
     (4) ゆえに危険状態には到達しない          … safety
*)

(* (1) 初期状態は不変条件を満たす *)
Lemma init_inv : invariant init.
Proof.
  unfold invariant, init. simpl.
  intros H. discriminate H.   (* 初期は未発送。前提 false = true が矛盾 *)
Qed.

(* invariant2*)
Lemma init_inv2 : invariant2 init.
Proof.
  unfold invariant2, init. simpl.
  intros H. discriminate H.
Qed.

(* (2) どんな1ステップの遷移も不変条件を保つ（保存性 preservation） *)
Lemma step_preserves_inv : forall s s',
  invariant s -> step s s' -> invariant s'.
Proof.
  unfold invariant.
  intros s s' Hinv Hstep.
  inversion Hstep; subst; simpl in *.
  - (* 支払い：発送状態は不変で paid=true になるので自明 *)
    intros _. reflexivity.
  - (* 発送：ガードにより paid s = true が前提にある *)
    intros _. assumption.
  - (* キャンセル：paid も shipped も不変なので元の不変条件で済む *)
    intros Hsh. apply Hinv. exact Hsh.
Qed.

(* invariant2*)
Lemma step_preserves_inv2 : forall s s',
  invariant2 s -> step s s' -> invariant2 s'.
Proof.
  unfold invariant2.
  intros s s' Hinv2 Hstep.
  inversion Hstep; subst; simpl in *.
  - exact Hinv2.
  - intros Hc. congruence.
  - intros _. assumption.
Qed.

Lemma reachable_holds (P : State -> Prop) :
  P init ->
  (forall s s', P s -> step s s' -> P s') ->
  forall s, reachable s -> P s.
Proof.
  intros Hbase Hpres s Hr.
  induction Hr as [| s s' Hreach IH Hstep].
  - exact Hbase.
  - exact (Hpres s s' IH Hstep).
Qed.

(* (3) 到達可能な全状態が不変条件を満たす（reachable に関する帰納法） *)
Theorem reachable_inv : forall s, reachable s -> invariant s.
Proof.
  intros s Hr.
  induction Hr as [| s s' Hreach IH Hstep].
  - apply init_inv.                              (* 初期状態の場合 *)
  - exact (step_preserves_inv s s' IH Hstep).    (* 1ステップ進めても保たれる *)
Qed.

(* (4) ゆえに、危険な状態には決して到達しない *)
Theorem safety : forall s, reachable s -> ~ bad s.
Proof.
  intros s Hr Hbad.
  unfold bad in Hbad. destruct Hbad as [Hsh Hpa].
  apply reachable_inv in Hr. unfold invariant in Hr.
  apply Hr in Hsh.          (* shipped s = true から paid s = true を得る *)
  rewrite Hsh in Hpa.       (* すると paid s = false が true = false になる *)
  discriminate Hpa.         (* 矛盾。よって bad は到達不能 *)
Qed.


(* おまけ：具体的な遷移列も実際に到達可能と示せる
   （初期 → 支払い → 発送 という正常系） *)
Example normal_flow_reachable :
  reachable (mkState true true false).
Proof.
  eapply reach_step.
  - eapply reach_step.
    + apply reach_init.
    + apply (step_pay init). reflexivity.   (* init から支払い *)
  - apply (step_ship (mkState true false false)).
    + reflexivity.   (* paid = true *)
    + reflexivity.   (* cancelled = false *)
Qed.
