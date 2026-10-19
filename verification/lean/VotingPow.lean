/-
Soundness model for the Uint256 voting-power arithmetic in dao-voting.

The integers here are the semantic contract of `cosmwasm_std::Uint256` and
`Uint512` at cosmwasm `c156aef60266be0cd568a815199984ad283ed2c8`:

* `checked_*` fails exactly when the mathematical result does not fit
* `full_mul` widens and does not overflow a 512-bit product of two 256-bit values
* `+` and `-` panic when the result does not fit

`compare_vote_count` multiplies by `10^9` in `Uint512`, then by the decimal
atomics, then divides by `10^18`.
-/

namespace VotingPow

def precision : Nat := 10 ^ 9
def decimalDenom : Nat := 10 ^ 18

theorem precision_pos : 0 < precision := by decide

theorem denom_pos : 0 < decimalDenom := by decide

theorem denom_is_precision_sq : decimalDenom = precision * precision := by decide

/-- `floor(total * 10^9 * atomics / 10^18)`, the threshold `compare_vote_count` builds. -/
def floorThreshold (total atomics : Nat) : Nat :=
  (total * precision * atomics) / decimalDenom

def compareGeq (votes total atomics : Nat) : Bool :=
  decide (votes * precision ≥ floorThreshold total atomics)

def compareGt (votes total atomics : Nat) : Bool :=
  decide (votes * precision > floorThreshold total atomics)

/-- V1, identity. The Rust function returns this boolean whenever the wide multiply fits. -/
theorem compare_geq_is_scaled_floor (votes total atomics : Nat) :
    compareGeq votes total atomics =
      decide (votes * precision ≥ (total * precision * atomics) / decimalDenom) := rfl

theorem compare_gt_is_scaled_floor (votes total atomics : Nat) :
    compareGt votes total atomics =
      decide (votes * precision > (total * precision * atomics) / decimalDenom) := rfl

/-- Every `Uint256` times `10^9` still fits in `Uint512`. `votes.full_mul(PRECISION_FACTOR)` does not overflow. -/
theorem votes_full_mul_fits (votes : Nat) (hv : votes < 2 ^ 256) :
    votes * precision < 2 ^ 512 := by
  have hv' : votes ≤ 2 ^ 256 - 1 := by omega
  have hprec : precision ≤ 2 ^ 30 - 1 := by native_decide
  have hstep : votes * precision ≤ (2 ^ 256 - 1) * (2 ^ 30 - 1) :=
    Nat.mul_le_mul hv' hprec
  have hmax : (2 ^ 256 - 1) * (2 ^ 30 - 1) < 2 ^ 512 := by native_decide
  exact Nat.lt_of_le_of_lt hstep hmax

/--
V1, overflow. For every voting power and every decimal atomics that fit in
`u128` (which is every `Decimal`), `(total * 10^9) * atomics` fits in `Uint512`.
Both `expect("fatal")` calls in `compare_vote_count` are unreachable.
-/
theorem compare_wide_mul_fits (total atomics : Nat)
    (ht : total < 2 ^ 256) (ha : atomics < 2 ^ 128) :
    total * precision * atomics < 2 ^ 512 := by
  have ht' : total ≤ 2 ^ 256 - 1 := by omega
  have ha' : atomics ≤ 2 ^ 128 - 1 := by omega
  have h1 : total * precision ≤ (2 ^ 256 - 1) * precision :=
    Nat.mul_le_mul_right _ ht'
  have h2 : total * precision * atomics ≤ (2 ^ 256 - 1) * precision * (2 ^ 128 - 1) :=
    Nat.mul_le_mul h1 ha'
  have hmax : (2 ^ 256 - 1) * precision * (2 ^ 128 - 1) < 2 ^ 512 := by native_decide
  exact Nat.lt_of_le_of_lt h2 hmax

theorem compare_divisor_nonzero : decimalDenom ≠ 0 := by decide

/-- V2. Majority compares `full_mul(2)`, which always fits, against the option count. -/
def majorityPass (yes options : Nat) : Bool :=
  decide (yes * 2 > options)

def majorityFail (no options : Nat) : Bool :=
  decide (no * 2 ≥ options)

theorem majority_double_fits (votes : Nat) (hv : votes < 2 ^ 256) :
    votes * 2 < 2 ^ 512 := by
  have hv' : votes ≤ 2 ^ 256 - 1 := by omega
  have h : votes * 2 ≤ (2 ^ 256 - 1) * 2 := Nat.mul_le_mul_right _ hv'
  have hmax : (2 ^ 256 - 1) * 2 < 2 ^ 512 := by native_decide
  exact Nat.lt_of_le_of_lt h hmax

/--
V3, majority. When yes and no are drawn from `options` without double counting,
a proposal is not both passed and failed by the majority rule.
-/
theorem majority_disjoint (yes no options : Nat) (hsum : yes + no ≤ options) :
    ¬ (majorityPass yes options = true ∧ majorityFail no options = true) := by
  intro h
  have hy : options < yes * 2 := by simpa [majorityPass] using h.1
  have hn : options ≤ no * 2 := by simpa [majorityFail] using h.2
  omega

/--
V4. `does_vote_count_pass` returns false when `options` is zero, before the
comparison. `compare_vote_count` itself is true for `Geq` on zeros
(`0 >= floor(0)`), which the Rust tests record. Fail is true for both rules.
-/
def passes (yes options atomics : Nat) (majority : Bool) : Bool :=
  if options = 0 then false
  else if majority then majorityPass yes options
  else compareGeq yes options atomics

def fails (no options atomics : Nat) (majority : Bool) : Bool :=
  if options = 0 then true
  else if majority then majorityFail no options
  else compareGt no options atomics

/-- `Decimal::one() - percent`. `none` when the passing fraction is above one. -/
def complementAtomics? (atomics : Nat) : Option Nat :=
  if atomics ≤ decimalDenom then some (decimalDenom - atomics) else none

theorem abstain_only (atomics : Nat) :
    passes 0 0 atomics true = false ∧
      passes 0 0 atomics false = false ∧
      fails 0 0 atomics true = true ∧
      fails 0 0 atomics false = true := by
  simp [passes, fails]

/--
V3, percent. Whole-percent thresholds are what `Decimal::percent` builds
(`atomics = k * 10^16`). Arbitrary atomics are not exclusive: the witness below
makes pass and fail both true, and `current_status` matches `is_passed` first.
-/
def overlapAtomics : Nat := (decimalDenom + 2) / 3

theorem percent_can_pass_and_fail :
    overlapAtomics + (decimalDenom - overlapAtomics) = decimalDenom ∧
      compareGeq 1 3 overlapAtomics = true ∧
      compareGt 2 3 (decimalDenom - overlapAtomics) = true := by
  native_decide

/-- `Decimal::percent(k)` for `k ≤ 100` lands on a multiple of `10^16` and is at most one. -/
theorem percent_constructor (k : Nat) (hk : k ≤ 100) :
    k * (10 ^ 16) ≤ decimalDenom := by
  have : k * (10 ^ 16) ≤ 100 * (10 ^ 16) := Nat.mul_le_mul_right _ hk
  have h100 : 100 * (10 ^ 16) = decimalDenom := by native_decide
  omega

/-- V5. Adding a power preserves the sum when the sum still fits in `Uint256`. -/
def checkedAdd (a b : Nat) : Option Nat :=
  if a + b < 2 ^ 256 then some (a + b) else none

theorem checkedAdd_some_iff (a b : Nat) :
    (∃ v, checkedAdd a b = some v ∧ v = a + b) ↔ a + b < 2 ^ 256 := by
  unfold checkedAdd
  split <;> simp_all

theorem checkedAdd_none (a b : Nat) (h : ¬ a + b < 2 ^ 256) :
    checkedAdd a b = none := by
  simp [checkedAdd, h]

inductive Choice where
  | yes
  | no
  | abstain
  deriving DecidableEq

def addLimb (yes no abstain power : Nat) (c : Choice) : Nat × Nat × Nat :=
  match c with
  | .yes => (yes + power, no, abstain)
  | .no => (yes, no + power, abstain)
  | .abstain => (yes, no, abstain + power)

theorem add_limb_preserves_sum (yes no abstain power : Nat) (c : Choice) :
    let (y, n, a) := addLimb yes no abstain power c
    y + n + a = yes + no + abstain + power := by
  cases c <;> simp [addLimb] <;> omega

/--
`Votes::add_vote` uses panicking `+=`. It agrees with `checkedAdd` on the
updated limb when the sum fits, and the Rust operator panics when `checkedAdd`
is `none`. `MultipleChoiceVotes::add_vote` returns that `none` as `StdError`.
-/
theorem single_choice_add_fits (limb power : Nat) (h : limb + power < 2 ^ 256) :
    checkedAdd limb power = some (limb + power) := by
  simp [checkedAdd, h]

/-- V6. `checked_mul(supply, 10^9)` fails exactly past this bound. -/
def checkedMul (a b : Nat) : Option Nat :=
  if a * b < 2 ^ 256 then some (a * b) else none

theorem active_mul_iff (supply : Nat) :
    (∃ v, checkedMul supply precision = some v ∧ v = supply * precision) ↔
      supply * precision < 2 ^ 256 := by
  unfold checkedMul
  split <;> simp_all

/-- Largest supply whose `checked_mul` by `10^9` still returns `Ok`. -/
def maxActiveSupply : Nat := (2 ^ 256 - 1) / precision

theorem max_active_supply_mul_ok :
    checkedMul maxActiveSupply precision = some (maxActiveSupply * precision) := by
  have h : maxActiveSupply * precision < 2 ^ 256 := by native_decide
  simp [checkedMul, h]

/--
At that supply and a 100% threshold, `applied + 10^9 - 1` does not fit in
`Uint256`. The checked active-threshold query returns `none` there.
`Decimal256::one` makes `multiply_ratio` return the scaled supply unchanged.
-/
theorem active_rounding_add_overflows :
    maxActiveSupply * precision + precision - 1 ≥ 2 ^ 256 := by
  native_decide

/--
cw721 counts that fit in `2^64` keep the rounded threshold inside `Uint128`,
so the `try_into().unwrap()` on that copy is reached only after a value that fits.
`rounded ≤ supply` whenever the percent is at most one (`atomics ≤ 10^18`).
-/
theorem rounded_le_supply (supply atomics : Nat)
    (ha : atomics ≤ decimalDenom) :
    let scaled := supply * precision
    let applied := (scaled * atomics) / decimalDenom
    let rounded := (applied + precision - 1) / precision
    rounded ≤ supply := by
  intro scaled applied rounded
  have happlied_le : applied ≤ scaled := by
    have hdiv : scaled * atomics ≤ scaled * decimalDenom := Nat.mul_le_mul_left _ ha
    have hquot : (scaled * atomics) / decimalDenom ≤ (scaled * decimalDenom) / decimalDenom :=
      Nat.div_le_div_right hdiv
    have hcancel : (scaled * decimalDenom) / decimalDenom = scaled :=
      Nat.mul_div_cancel _ denom_pos
    rw [hcancel] at hquot
    simpa [applied, scaled] using hquot
  have hle : applied + precision - 1 ≤ scaled + precision - 1 :=
    Nat.sub_le_sub_right (Nat.add_le_add_right happlied_le precision) 1
  have hid : scaled + precision = (supply + 1) * precision := by
    simp [scaled, Nat.succ_mul]
  have hlt : scaled + precision - 1 < (supply + 1) * precision := by
    have hsub : scaled + precision - 1 < scaled + precision :=
      Nat.sub_lt (Nat.lt_of_lt_of_le precision_pos (Nat.le_add_left precision scaled)) (by decide : 0 < 1)
    simpa [hid] using hsub
  have : rounded < supply + 1 :=
    (Nat.div_lt_iff_lt_mul precision_pos).mpr (Nat.lt_of_le_of_lt hle hlt)
  omega

/-- V7. Delegation sums with `checked_add`. The result fits, or the call returns `Err`. -/
theorem delegation_checked_add (individual delegated : Nat) :
    checkedAdd individual delegated = some (individual + delegated) ∨
      checkedAdd individual delegated = none := by
  by_cases h : individual + delegated < 2 ^ 256
  · exact Or.inl (by simp [checkedAdd, h])
  · exact Or.inr (by simp [checkedAdd, h])

theorem delegation_no_wrap (individual delegated : Nat)
    (h : individual + delegated < 2 ^ 256) :
    checkedAdd individual delegated = some (individual + delegated) := by
  simp [checkedAdd, h]


/-- Subtraction in the proposal predicates. `none` is the panicking `-`. -/
def checkedSub (a b : Nat) : Option Nat :=
  if b ≤ a then some (a - b) else none

structure Tally where
  yes : Nat
  no : Nat
  abstain : Nat

/-- `Votes::total`. `none` is the panicking `+`. -/
def checkedTotal (t : Tally) : Option Nat :=
  match checkedAdd t.yes t.no with
  | none => none
  | some yn => checkedAdd yn t.abstain

/-- `Votes::remove_vote`. `none` is the panicking `-=`. -/
def removeLimb (yes no abstain power : Nat) (c : Choice) : Option (Nat × Nat × Nat) :=
  match c with
  | .yes =>
    match checkedSub yes power with
    | none => none
    | some y => some (y, no, abstain)
  | .no =>
    match checkedSub no power with
    | none => none
    | some n => some (yes, n, abstain)
  | .abstain =>
    match checkedSub abstain power with
    | none => none
    | some a => some (yes, no, a)

theorem remove_limb_preserves_sum (yes no abstain power : Nat) (c : Choice)
    (h : match c with
      | .yes => power ≤ yes
      | .no => power ≤ no
      | .abstain => power ≤ abstain) :
    match removeLimb yes no abstain power c with
    | some (y, n, a) => y + n + a + power = yes + no + abstain
    | none => False := by
  cases c <;> simp [removeLimb, checkedSub, h] <;> omega

theorem remove_limb_none (yes no abstain power : Nat) (c : Choice)
    (h : match c with
      | .yes => yes < power
      | .no => no < power
      | .abstain => abstain < power) :
    removeLimb yes no abstain power c = none := by
  cases c with
  | yes =>
    have hy : yes < power := by simpa using h
    simp [removeLimb, checkedSub, Nat.not_le_of_gt hy]
  | no =>
    have hn : no < power := by simpa using h
    simp [removeLimb, checkedSub, Nat.not_le_of_gt hn]
  | abstain =>
    have ha : abstain < power := by simpa using h
    simp [removeLimb, checkedSub, Nat.not_le_of_gt ha]

theorem votes_total_some (yes no abstain : Nat) (h : yes + no + abstain < 2 ^ 256) :
    checkedTotal ⟨yes, no, abstain⟩ = some (yes + no + abstain) := by
  have hyn : yes + no < 2 ^ 256 := by omega
  simp [checkedTotal, checkedAdd, hyn, h]

theorem votes_total_none (yes no abstain : Nat) (h : 2 ^ 256 ≤ yes + no + abstain) :
    checkedTotal ⟨yes, no, abstain⟩ = none := by
  by_cases hyn : yes + no < 2 ^ 256
  · have hrest : ¬ yes + no + abstain < 2 ^ 256 := by omega
    simp [checkedTotal, checkedAdd, hyn, hrest]
  · simp [checkedTotal, checkedAdd, hyn]

/--
`effective.saturating_sub(total.saturating_sub(delegated).min(effective))`.
This is the delegation-loss query. `effective ≤ total` is the cap invariant.
-/
def delegationLoss (total effective delegated : Nat) : Nat :=
  effective - min (total - delegated) effective

theorem delegation_loss_uncapped (total effective delegated : Nat)
    (he : effective ≤ total) (hd : delegated ≤ total) (hc : total ≤ effective) :
    delegationLoss total effective delegated = delegated := by
  have hte : total = effective := Nat.le_antisymm hc he
  rw [delegationLoss, hte]
  have hsub : effective - delegated ≤ effective := Nat.sub_le _ _
  rw [Nat.min_eq_left hsub]
  omega

theorem delegation_loss_partial (total effective delegated : Nat)
    (hd : delegated ≤ total) (hpart : total - delegated < effective) :
    delegationLoss total effective delegated = effective - (total - delegated) := by
  have _bound := hd
  simp [delegationLoss, Nat.min_eq_left (Nat.le_of_lt hpart)]

theorem delegation_loss_zero (total effective delegated : Nat)
    (hd : delegated ≤ total) (h : effective ≤ total - delegated) :
    delegationLoss total effective delegated = 0 := by
  have _bound := hd
  simp [delegationLoss, Nat.min_eq_right h, Nat.sub_self]

theorem delegation_loss_stale (total effective delegated : Nat)
    (hstale : total < delegated) :
    delegationLoss total effective delegated = effective := by
  have hzero : total - delegated = 0 := Nat.sub_eq_zero_of_le (Nat.le_of_lt hstale)
  simp [delegationLoss, hzero]

theorem delegation_loss_le_delegated (total effective delegated : Nat)
    (he : effective ≤ total) :
    delegationLoss total effective delegated ≤ delegated := by
  unfold delegationLoss
  by_cases hd : delegated ≤ total
  · by_cases hpart : total - delegated < effective
    · omega
    · have hge : effective ≤ total - delegated := Nat.le_of_not_gt hpart
      omega
  · have hzero : total - delegated = 0 := Nat.sub_eq_zero_of_le (Nat.le_of_not_ge hd)
    omega

/-- When the ballot recorded this same delegated power, the cap leaves individual power. -/
theorem ballot_keeps_individual (individual delegated loss : Nat)
    (h : loss ≤ delegated) :
    individual + delegated - min (individual + delegated) loss =
      individual + (delegated - loss) := by
  have hle : loss ≤ individual + delegated := Nat.le_trans h (Nat.le_add_left _ _)
  have hmin : min (individual + delegated) loss = loss := Nat.min_eq_right hle
  omega

theorem query_loss_keeps_recorded_individual
    (total effective delegated individual : Nat) (he : effective ≤ total) :
    let loss := delegationLoss total effective delegated
    individual + delegated - min (individual + delegated) loss ≥ individual := by
  intro loss
  have hloss : loss ≤ delegated := delegation_loss_le_delegated total effective delegated he
  have hkept := ballot_keeps_individual individual delegated loss hloss
  omega

/--
Arms of `SingleChoiceProposal`. `hundred` is
`PercentageThreshold::Percent(Decimal::percent(100))`. `is_rejected` takes
that arm before `does_vote_count_fail`. `is_passed` does not.
-/
inductive SingleThreshold where
  | percentage (majority : Bool) (atomics : Nat) (hundred : Bool)
  | quorum (thresholdMajority : Bool) (thresholdAtomics : Nat) (hundred : Bool)
      (quorumMajority : Bool) (quorumAtomics : Nat)
  | count (threshold : Nat)

def tZero : Tally := ⟨0, 0, 0⟩
def tYes2 : Tally := ⟨2, 0, 0⟩
def tNo2 : Tally := ⟨0, 2, 0⟩
def tYes3 : Tally := ⟨3, 0, 0⟩
def tAbstain2 : Tally := ⟨0, 0, 2⟩

/-- 100% rejection: no options, or at least one no. Kept because it matches `fails` at atomics 0. -/
def hundredPercentFail (no options : Nat) : Bool :=
  if options = 0 then true else decide (no ≥ 1)

theorem single_choice_hundred_percent_fail (no options : Nat) :
    hundredPercentFail no options = fails no options 0 false := by
  cases options with
  | zero =>
    simp [hundredPercentFail, fails]
  | succ m =>
    cases no with
    | zero =>
      simp [hundredPercentFail, fails, compareGt, floorThreshold, Nat.mul_zero, Nat.zero_div]
    | succ n =>
      have hhun : hundredPercentFail (Nat.succ n) (Nat.succ m) = true := by
        simp [hundredPercentFail]
      have hfail : fails (Nat.succ n) (Nat.succ m) 0 false = true := by
        simp [fails, compareGt, floorThreshold, Nat.mul_zero, Nat.zero_div]
        exact precision_pos
      rw [hhun, hfail]

/--
`SingleChoiceProposal::is_passed`. Expired proposals read the full tally.
Open proposals read `individual_votes`. `minBlocking` is a live minimum
voting period. Re-voting blocks the result until expiry.
-/
def singlePassed? (expired allowRevoting minBlocking : Bool)
    (individual full : Tally) (totalPower : Nat) (th : SingleThreshold) : Option Bool :=
  if allowRevoting && !expired then some false
  else if minBlocking then some false
  else
    let votes := if expired then full else individual
    match th with
    | .percentage majority atomics _ =>
      match checkedSub totalPower votes.abstain with
      | none => none
      | some options => some (passes votes.yes options atomics majority)
    | .quorum majority atomics _ quorumMajority quorumAtomics =>
      match checkedTotal votes with
      | none => none
      | some cast =>
        if passes cast totalPower quorumAtomics quorumMajority = false then some false
        else
          match checkedSub (if expired then cast else totalPower) votes.abstain with
          | none => none
          | some options => some (passes votes.yes options atomics majority)
    | .count threshold => some (decide (votes.yes ≥ threshold))

/--
`does_vote_count_fail` for a percent compares against `Decimal::one() - percent`.
The 100% arm is the early return in `is_rejected`. Majority ignores atomics.
-/
def rejectByThreshold (no options atomics : Nat) (majority hundred : Bool) : Option Bool :=
  if hundred then some (hundredPercentFail no options)
  else if majority then some (fails no options atomics true)
  else
    match complementAtomics? atomics with
    | none => none
    | some complement => some (fails no options complement false)

/-- `SingleChoiceProposal::is_rejected`. A live minimum voting period does not block this. -/
def singleRejected? (expired allowRevoting : Bool)
    (individual full : Tally) (totalPower : Nat) (th : SingleThreshold) : Option Bool :=
  if allowRevoting && !expired then some false
  else
    let votes := if expired then full else individual
    match th with
    | .percentage majority atomics hundred =>
      match checkedSub totalPower votes.abstain with
      | none => none
      | some options => rejectByThreshold votes.no options atomics majority hundred
    | .quorum majority atomics hundred quorumMajority quorumAtomics =>
      match checkedTotal votes with
      | none => none
      | some cast =>
        let quorumMet := passes cast totalPower quorumAtomics quorumMajority
        if quorumMet && expired then
          match checkedSub cast votes.abstain with
          | none => none
          | some options => rejectByThreshold votes.no options atomics majority hundred
        else if !expired then
          match checkedSub totalPower votes.abstain with
          | none => none
          | some options => rejectByThreshold votes.no options atomics majority hundred
        else some true
    | .count threshold =>
      match checkedTotal votes with
      | none => none
      | some cast =>
        match checkedSub totalPower cast with
        | none => none
        | some outstanding =>
          match checkedAdd votes.yes outstanding with
          | none => none
          | some summed => some (decide (summed < threshold))

/-- Absolute percentage: options are `total_power - abstain`, then `passes`. -/
theorem single_choice_percentage_options
    (yes abstain total atomics : Nat) (majority : Bool)
    (h : abstain ≤ total) :
    singlePassed? true false false ⟨yes, 0, abstain⟩ ⟨yes, 0, abstain⟩ total
        (.percentage majority atomics false) =
      some (passes yes (total - abstain) atomics majority) := by
  simp [singlePassed?, checkedSub, h]

/--
Expired quorum compares yes with votes cast minus abstain. An open quorum
compares yes with total power minus abstain. Here cast is 3, total is 10,
and the majority threshold accepts one and rejects the other.
-/
theorem single_choice_quorum_denominator :
    singlePassed? true false false tYes3 tYes3 10
        (.quorum true 0 false false 0) = some true ∧
      singlePassed? false false false tYes3 tYes3 10
        (.quorum true 0 false false 0) = some false := by
  decide

/--
Absolute count. Pass is `yes ≥ threshold`. Rejection is
`yes + (total_power - total()) < threshold`, with no and abstain inside `total()`.
Here yes 1, no 2, abstain 3, total power 10: outstanding is 4, and 1 + 4 = 5.
-/
theorem single_choice_absolute_count :
    singlePassed? true false false ⟨1, 2, 3⟩ ⟨1, 2, 3⟩ 10 (.count 6) = some false ∧
      singleRejected? true false ⟨1, 2, 3⟩ ⟨1, 2, 3⟩ 10 (.count 6) = some true ∧
      singleRejected? true false ⟨1, 2, 3⟩ ⟨1, 2, 3⟩ 10 (.count 5) = some false := by
  decide

/-- Re-voting hides both outcomes until expiry, including a tally that would otherwise pass or fail. -/
theorem single_choice_revoting_blocks :
    singlePassed? false true false tYes2 tYes2 2 (.percentage true 0 false) = some false ∧
      singleRejected? false true tNo2 tNo2 2 (.percentage true 0 false) = some false := by
  decide

/--
A live minimum voting period blocks a pass that the same majority tally would
otherwise produce. It does not block rejection.
-/
theorem single_choice_min_period_blocks_pass :
    singlePassed? true false false tYes2 tYes2 2 (.percentage true 0 false) = some true ∧
      singlePassed? true false true tYes2 tYes2 2 (.percentage true 0 false) = some false ∧
      singleRejected? true false tNo2 tNo2 2 (.percentage true 0 false) = some true := by
  decide

/--
60% rejects on the complement. Five no-votes out of 10 are above 40% and not above 60%.
`Decimal::percent(60)` has atomics `6 * 10^17`.
-/
def percent60 : Nat := 6 * 10 ^ 17

theorem rejection_uses_complement :
    singleRejected? true false ⟨0, 5, 0⟩ ⟨0, 5, 0⟩ 10 (.percentage false percent60 false) =
        some true ∧
      compareGt 5 10 percent60 = false ∧
      compareGt 5 10 (decimalDenom - percent60) = true := by
  native_decide

/-- The same scaled floor, evaluated: 6 of 10 meets 60%, and 5 of 10 does not. -/
theorem single_choice_percentage_evaluates :
    singlePassed? true false false ⟨6, 0, 0⟩ ⟨6, 0, 0⟩ 10 (.percentage false percent60 false) =
        some true ∧
      singlePassed? true false false ⟨5, 0, 0⟩ ⟨5, 0, 0⟩ 10 (.percentage false percent60 false) =
        some false := by
  native_decide

/-- The 100% arm, reached through `singleRejected?`, agrees with failure at complement 0. -/
theorem single_choice_hundred_percent_rejected :
    singleRejected? true false ⟨0, 1, 0⟩ ⟨0, 1, 0⟩ 10 (.percentage false decimalDenom true) =
        some true ∧
      singleRejected? true false ⟨0, 0, 0⟩ ⟨0, 0, 0⟩ 10 (.percentage false decimalDenom true) =
        some false ∧
      singleRejected? true false ⟨0, 0, 10⟩ ⟨0, 0, 10⟩ 10 (.percentage false decimalDenom true) =
        some true ∧
      singleRejected? true false ⟨0, 1, 0⟩ ⟨0, 1, 0⟩ 10 (.percentage false decimalDenom false) =
        some true := by
  native_decide

/--
Before expiry the individual tally is what passes. Delegated yes votes that
exist only on the full tally do not pass the proposal until it expires.
-/
theorem single_choice_delegation_after_expiry :
    singlePassed? false false false tZero tYes2 2 (.percentage true 0 false) = some false ∧
      singlePassed? true false false tZero tYes2 2 (.percentage true 0 false) = some true := by
  decide

/-- Quorum missed and voting closed: pass is false and rejection is true, with no threshold test. -/
theorem single_choice_expired_quorum_fail_rejects :
    singlePassed? true false false tZero tZero 10 (.quorum true 0 false true 0) = some false ∧
      singleRejected? true false tZero tZero 10 (.quorum true 0 false true 0) = some true := by
  decide

/-- `total_power - abstain` panics when abstain exceeds total power, so no boolean is produced. -/
theorem percentage_options_undefined :
    singlePassed? true false false tAbstain2 tAbstain2 1 (.percentage false 0 false) = none ∧
      singleRejected? true false tAbstain2 tAbstain2 1 (.percentage false 0 false) = none := by
  decide

inductive ProposalStatus where
  | passed
  | rejected
  | pending
  | vetoTimelock
  deriving DecidableEq

/--
`Proposal::current_status` on an open proposal. Pass is tested first, including
when the rejection predicate is also true. Expiry rejects an open proposal that
has not passed, even when `is_rejected` is false. A live veto timelock replaces
`passed`.
-/
def currentStatus (pass rejected expired vetoConfigured timelockExpired : Bool) : ProposalStatus :=
  if pass then
    if vetoConfigured then
      if timelockExpired then ProposalStatus.passed else ProposalStatus.vetoTimelock
    else
      ProposalStatus.passed
  else if expired || rejected then
    ProposalStatus.rejected
  else
    ProposalStatus.pending

theorem current_status_pass_first :
    currentStatus (compareGeq 1 3 overlapAtomics)
        (compareGt 2 3 (decimalDenom - overlapAtomics)) true false false =
      ProposalStatus.passed := by
  have h := percent_can_pass_and_fail
  rw [h.2.1, h.2.2]
  decide

theorem current_status_veto_timelock (rejected expired : Bool) :
    currentStatus true rejected expired true false = ProposalStatus.vetoTimelock ∧
      currentStatus true rejected expired true false ≠ ProposalStatus.passed := by
  cases rejected <;> cases expired <;> decide

/-- Expiry alone rejects an open proposal that did not pass. -/
theorem current_status_expired_rejects :
    currentStatus false false true false false = ProposalStatus.rejected ∧
      currentStatus false false false false false = ProposalStatus.pending := by
  decide

theorem percent_overlap_reports_passed :
    compareGeq 1 3 overlapAtomics = true ∧
      compareGt 2 3 (decimalDenom - overlapAtomics) = true ∧
      currentStatus (compareGeq 1 3 overlapAtomics)
        (compareGt 2 3 (decimalDenom - overlapAtomics)) false false false =
        ProposalStatus.passed := by
  have h := percent_can_pass_and_fail
  refine ⟨h.2.1, h.2.2, ?_⟩
  rw [h.2.1, h.2.2]
  decide

/--
Multiple-choice early pass. An expired standard winner passes without the
unbeatable test. An open standard winner passes only when that test is true.
`None` and a tie do not pass.
-/
def mcPassed (expired allowRevoting minBlocking quorumMet tied noneWinner unbeatable : Bool) : Bool :=
  if allowRevoting && !expired then false
  else if minBlocking then false
  else if !quorumMet then false
  else if tied || noneWinner then false
  else if expired then true
  else unbeatable

/--
Multiple-choice rejection. A tie rejects when voting is closed or no power
remains. `None` rejects after expiry when quorum passed, and before expiry
only when it is unbeatable. Missed quorum rejects after expiry.
-/
def mcRejected (expired allowRevoting quorumMet tied noneWinner unbeatable totalEqCast : Bool) : Bool :=
  if allowRevoting && !expired then false
  else if tied then expired || totalEqCast
  else if quorumMet && expired then noneWinner
  else if !expired && noneWinner then unbeatable
  else if !quorumMet && expired then true
  else false

inductive ChoiceKind where
  | standard
  | none
  deriving DecidableEq

/-- `is_choice_unbeatable`. `none` is `total_power - total()` when the cast exceeds total power. -/
def choiceUnbeatable (winning second total cast : Nat) (kind : ChoiceKind) : Option Bool :=
  match checkedSub total cast with
  | none => none
  | some remaining =>
    match kind with
    | .standard => some (decide (winning > second + remaining))
    | .none => some (decide (winning ≥ second + remaining))

theorem unbeatable_standard :
    choiceUnbeatable 5 2 5 3 .standard = some true ∧
      choiceUnbeatable 5 3 7 5 .standard = some false := by
  decide

theorem unbeatable_none :
    choiceUnbeatable 5 2 8 5 .none = some true ∧
      choiceUnbeatable 5 2 8 5 .standard = some false := by
  decide

theorem mc_expired_pass_ignores_unbeatable :
    mcPassed true false false true false false false = true ∧
      mcPassed false false false true false false false = false := by
  decide

theorem mc_open_pass_needs_unbeatable :
    choiceUnbeatable 5 2 5 3 ChoiceKind.standard = some true ∧
      mcPassed false false false true false false true = true ∧
      choiceUnbeatable 5 3 7 5 ChoiceKind.standard = some false ∧
      mcPassed false false false true false false false = false ∧
      choiceUnbeatable 5 2 8 5 ChoiceKind.none = some true ∧
      mcRejected false false true false true true false = true ∧
      mcRejected true false true false true false false = true := by
  decide

theorem multiple_choice_remaining_undefined :
    choiceUnbeatable 5 2 3 5 ChoiceKind.standard = none ∧
      choiceUnbeatable 5 2 3 5 ChoiceKind.none = none := by
  decide

/--
Checked active-threshold rounding: `checked_mul` by `10^9` (`supply * 10^9 < 2^256`),
floor division by `10^18`, then `checked_add` of `10^9`. The ceiling quotient is
returned only when that add fits.
-/
def checkedActiveRound (supply atomics : Nat) : Option Nat :=
  if supply * precision < 2 ^ 256 then
    let applied := (supply * precision * atomics) / decimalDenom
    if applied + precision < 2 ^ 256 then
      some ((applied + precision - 1) / precision)
    else
      none
  else
    none

theorem active_checked_round_overflows :
    checkedActiveRound maxActiveSupply decimalDenom = none := by
  native_decide

theorem active_checked_round_le_supply (supply atomics v : Nat)
    (hv : checkedActiveRound supply atomics = some v)
    (ha : atomics ≤ decimalDenom) :
    v ≤ supply := by
  have hceil := rounded_le_supply supply atomics ha
  unfold checkedActiveRound at hv
  by_cases hmul : supply * precision < 2 ^ 256
  · by_cases hadd : (supply * precision * atomics) / decimalDenom + precision < 2 ^ 256
    · rw [if_pos hmul, if_pos hadd] at hv
      cases hv
      simpa using hceil
    · rw [if_pos hmul, if_neg hadd] at hv
      cases hv
  · rw [if_neg hmul] at hv
    cases hv

theorem delegated_mul_floor_fits (vp atomics : Nat)
    (hv : vp < 2 ^ 256) (ha : atomics ≤ decimalDenom) :
    (vp * atomics) / decimalDenom < 2 ^ 256 := by
  have hle : (vp * atomics) / decimalDenom ≤ vp := by
    have hdiv : vp * atomics ≤ vp * decimalDenom := Nat.mul_le_mul_left _ ha
    have hquot : (vp * atomics) / decimalDenom ≤ (vp * decimalDenom) / decimalDenom :=
      Nat.div_le_div_right hdiv
    have hcancel : (vp * decimalDenom) / decimalDenom = vp :=
      Nat.mul_div_cancel _ denom_pos
    simpa [hcancel] using hquot
  omega

theorem delegated_reduction_capped (ballot reduction : Nat) :
    min ballot reduction ≤ ballot := Nat.min_le_left _ _

/--
When the queried reduction exceeds the ballot, the cap equals the whole ballot.
A ballot of individual 7 plus delegated 3, reduced by 11, loses both parts.
This is the arithmetic of the current `min`. It is not a contract edit.
-/
theorem delegated_reduction_includes_individual :
    min (7 + 3) 11 = 7 + 3 := by
  decide

/--
Condorcet `add_vote` before it writes the matrix. A power or an outstanding
value outside `Uint128`, or a power above outstanding, is
`PowerExceedsTallyRange`. Otherwise the outstanding power is the checked difference.
-/
inductive CondorcetStep where
  | rejected
  | accepted (remaining : Nat)
  deriving DecidableEq

def condorcetAdd (outstanding power : Nat) : CondorcetStep :=
  if ¬ power < 2 ^ 128 then CondorcetStep.rejected
  else if ¬ outstanding < 2 ^ 128 then CondorcetStep.rejected
  else if power > outstanding then CondorcetStep.rejected
  else CondorcetStep.accepted (outstanding - power)

theorem condorcet_power_fits_u128 :
    condorcetAdd 0 (2 ^ 128) = CondorcetStep.rejected ∧
      condorcetAdd 1 0 = CondorcetStep.accepted 1 := by
  native_decide

theorem condorcet_sub_fits :
    condorcetAdd 5 3 = CondorcetStep.accepted 2 ∧
      condorcetAdd 5 6 = CondorcetStep.rejected ∧
      condorcetAdd (2 ^ 128) 1 = CondorcetStep.rejected := by
  native_decide

end VotingPow
