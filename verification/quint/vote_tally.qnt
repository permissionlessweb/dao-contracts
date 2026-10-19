// Executable spec of one proposal tally and of the scaled-floor comparison.
// The Rust evaluator only accepts i64 literals, so this module uses a smaller
// instance of the same formula: PRECISION^2 = DENOM, and BOUND fits in i64.
// The 10^9 / 10^18 instance and the 2^256 ceiling are proved in VotingPow.lean.
module vote_tally {
  pure val PRECISION: int = 100
  pure val DENOM: int = 10000
  pure val BOUND: int = 1000000

  type Choice = Yes | No | Abstain

  var yes: int
  var no: int
  var abstain: int

  pure def inRange(x: int): bool = and(0 <= x, x <= BOUND)

  pure def tallyIsSum: bool =
    and(inRange(yes), inRange(no), inRange(abstain), inRange(yes + no + abstain))

  pure def floorThreshold(total: int, atomics: int): int =
    (total * PRECISION * atomics) / DENOM

  pure def compareGeq(votes: int, total: int, atomics: int): bool =
    votes * PRECISION >= floorThreshold(total, atomics)

  pure def compareGt(votes: int, total: int, atomics: int): bool =
    votes * PRECISION > floorThreshold(total, atomics)

  pure def majorityPass(y: int, options: int): bool = y * 2 > options

  pure def majorityFail(n: int, options: int): bool = n * 2 >= options

  pure def majorityDisjoint(y: int, n: int, options: int): bool =
    not(and(y + n <= options, majorityPass(y, options), majorityFail(n, options)))

  action init: bool = all {
    yes' = 0,
    no' = 0,
    abstain' = 0,
  }

  pure def added(choice: Choice, toward: Choice, power: int): int =
    if (choice == toward) power else 0

  action cast(power: int, choice: Choice): bool = all {
    power >= 0,
    yes + no + abstain + power <= BOUND,
    yes' = yes + added(choice, Yes, power),
    no' = no + added(choice, No, power),
    abstain' = abstain + added(choice, Abstain, power),
  }

  action step: bool = {
    nondet power = oneOf(0.to(4))
    nondet choice = oneOf(Set(Yes, No, Abstain))
    cast(power, choice)
  }

  action tallyHolds: bool = all {
    assert(tallyIsSum),
    assert(yes + no + abstain == 8),
    yes' = yes,
    no' = no,
    abstain' = abstain,
  }

  run castPreservesSumTest =
    init.then(cast(4, Yes)).then(cast(3, No)).then(cast(1, Abstain)).then(tallyHolds)

  run majorityDisjointTest = all {
    assert(majorityDisjoint(2, 1, 3)),
    assert(majorityDisjoint(1, 1, 3)),
    assert(majorityDisjoint(0, 0, 0)),
    assert(not(majorityPass(0, 0))),
    assert(majorityFail(0, 0)),
  }

  // Pass and fail are both true for this fraction. current_status tries
  // is_passed before is_rejected, so the state is reported passed.
  run percentOverlapTest = all {
    assert((DENOM + 2) / 3 + (DENOM - (DENOM + 2) / 3) == DENOM),
    assert(compareGeq(1, 3, (DENOM + 2) / 3)),
    assert(compareGt(2, 3, DENOM - (DENOM + 2) / 3)),
  }
}
