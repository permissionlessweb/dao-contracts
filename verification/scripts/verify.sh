#!/bin/sh
# Re-check the Uint256 voting-power artifacts. Run from anywhere.
# Tools: lake (elan), python3, quint, java (Apalache), cargo.
# Uses PATH first. Fills gaps from ELAN_HOME, CARGO_HOME, JAVA_HOME, and NVM_DIR.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
cd "$ROOT"

prepend_dir() {
  if [ -n "$1" ] && [ -d "$1" ]; then
    PATH="$1:${PATH:-/usr/bin:/bin}"
  fi
}

if ! command -v lake >/dev/null 2>&1; then
  prepend_dir "${ELAN_HOME:-$HOME/.elan}/bin"
fi
if ! command -v cargo >/dev/null 2>&1; then
  prepend_dir "${CARGO_HOME:-$HOME/.cargo}/bin"
fi
if ! command -v python3 >/dev/null 2>&1; then
  prepend_dir "$HOME/.local/bin"
fi
if ! command -v python3 >/dev/null 2>&1 && command -v brew >/dev/null 2>&1; then
  brew_prefix=$(brew --prefix 2>/dev/null || true)
  prepend_dir "$brew_prefix/bin"
fi
if ! command -v python3 >/dev/null 2>&1 && [ -d /Library/Frameworks/Python.framework/Versions ]; then
  py=$(find /Library/Frameworks/Python.framework/Versions -maxdepth 2 -type f -name python3 2>/dev/null | sort | tail -n 1)
  if [ -n "$py" ]; then
    prepend_dir "$(dirname "$py")"
  fi
fi
if ! command -v java >/dev/null 2>&1 && [ -n "${JAVA_HOME:-}" ]; then
  prepend_dir "$JAVA_HOME/bin"
fi
if ! command -v java >/dev/null 2>&1 && [ -x /usr/libexec/java_home ]; then
  jh=$(/usr/libexec/java_home -v 21 2>/dev/null || true)
  prepend_dir "$jh/bin"
fi
if ! command -v java >/dev/null 2>&1 && command -v brew >/dev/null 2>&1; then
  jdk_prefix=$(brew --prefix openjdk@21 2>/dev/null || true)
  prepend_dir "$jdk_prefix/bin"
fi
if ! command -v quint >/dev/null 2>&1; then
  nvm_root="${NVM_DIR:-$HOME/.nvm}/versions/node"
  if [ -d "$nvm_root" ]; then
    quint_bin=$(
      find "$nvm_root" -maxdepth 3 -name quint ! -type d 2>/dev/null |
        while IFS= read -r bin; do
          ver=$(basename "$(dirname "$bin")")
          ver=${ver#v}
          printf '%s\t%s\n' "$ver" "$bin"
        done |
        sort -t. -k1,1n -k2,2n -k3,3n |
        tail -n 1 |
        cut -f2-
    )
    if [ -n "$quint_bin" ]; then
      prepend_dir "$(dirname "$quint_bin")"
    fi
  fi
fi
export PATH

missing=""
for cmd in lake python3 quint java cargo; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    missing="$missing $cmd"
  fi
done
if [ -n "$missing" ]; then
  echo "missing tools:${missing}" >&2
  echo "Install them on PATH, or set JAVA_HOME, ELAN_HOME, CARGO_HOME, or NVM_DIR." >&2
  exit 1
fi

LOCK="$ROOT/Cargo.lock"
LOCK_BACKUP=$(mktemp)
cp "$LOCK" "$LOCK_BACKUP"

restore_lock() {
  code=$?
  if [ -f "$LOCK_BACKUP" ] && ! cmp -s "$LOCK" "$LOCK_BACKUP"; then
    cp "$LOCK_BACKUP" "$LOCK"
    echo "restored Cargo.lock" >&2
  fi
  rm -f "$LOCK_BACKUP"
  rm -rf "$ROOT/_apalache-out"
  if [ "$code" -ne 0 ]; then
    printf '%s\n' "uint256 voting-power verification failed" > "$ROOT/verification/LAST_RUN.txt"
  fi
}
trap restore_lock EXIT

echo "== lean =="
( cd "$ROOT/verification/lean" && lake build )
if grep -R -n --include='*.lean' '\bsorry\b' "$ROOT/verification/lean" >/dev/null; then
  echo "sorry remains in verification/lean" >&2
  exit 1
fi

LEAN="$ROOT/verification/lean/VotingPow.lean"
for name in \
  compare_geq_is_scaled_floor \
  compare_gt_is_scaled_floor \
  votes_full_mul_fits \
  compare_wide_mul_fits \
  compare_divisor_nonzero \
  majority_double_fits \
  majority_disjoint \
  abstain_only \
  percent_can_pass_and_fail \
  percent_constructor \
  checkedAdd_some_iff \
  checkedAdd_none \
  add_limb_preserves_sum \
  single_choice_add_fits \
  remove_limb_preserves_sum \
  remove_limb_none \
  votes_total_some \
  votes_total_none \
  delegation_loss_uncapped \
  delegation_loss_partial \
  delegation_loss_zero \
  delegation_loss_stale \
  delegation_loss_le_delegated \
  ballot_keeps_individual \
  query_loss_keeps_recorded_individual \
  active_mul_iff \
  max_active_supply_mul_ok \
  active_rounding_add_overflows \
  rounded_le_supply \
  delegation_checked_add \
  delegation_no_wrap \
  single_choice_percentage_options \
  single_choice_percentage_evaluates \
  rejection_uses_complement \
  single_choice_hundred_percent_rejected \
  single_choice_quorum_denominator \
  single_choice_absolute_count \
  single_choice_hundred_percent_fail \
  single_choice_revoting_blocks \
  single_choice_min_period_blocks_pass \
  single_choice_delegation_after_expiry \
  single_choice_expired_quorum_fail_rejects \
  percentage_options_undefined \
  current_status_pass_first \
  current_status_veto_timelock \
  current_status_expired_rejects \
  percent_overlap_reports_passed \
  unbeatable_standard \
  unbeatable_none \
  mc_expired_pass_ignores_unbeatable \
  mc_open_pass_needs_unbeatable \
  multiple_choice_remaining_undefined \
  active_checked_round_overflows \
  active_checked_round_le_supply \
  delegated_mul_floor_fits \
  delegated_reduction_capped \
  delegated_reduction_includes_individual \
  condorcet_power_fits_u128 \
  condorcet_sub_fits
do
  if ! grep -q "^theorem ${name}" "$LEAN"; then
    echo "missing theorem ${name}" >&2
    exit 1
  fi
done

echo "== scope =="
python3 "$ROOT/verification/scripts/scope_voting_power.py"

echo "== quint test =="
quint test "$ROOT/verification/quint/vote_tally.qnt"

echo "== quint verify =="
# Apalache rejects this invariant as an inductive predicate because it
# constrains yes/no/abstain by inequalities. The bounded check from init
# is the gate. The unbounded sum is add_limb_preserves_sum in Lean.
qout=$(quint verify --invariant=tallyIsSum --max-steps=4 --main=vote_tally "$ROOT/verification/quint/vote_tally.qnt" 2>&1) || {
  printf '%s\n' "$qout" >&2
  exit 1
}
printf '%s\n' "$qout"
printf '%s\n' "$qout" | grep -q "No violation found"

echo "== rust oracle =="
cargo test -p dao-voting --offline --lib uint256_voting_power_soundness -- --nocapture

echo "uint256 voting-power verification passed"
printf '%s\n' "uint256 voting-power verification passed" > "$ROOT/verification/LAST_RUN.txt"
