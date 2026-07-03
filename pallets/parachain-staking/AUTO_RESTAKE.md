# Delegator auto-restake (inline)

Delegator staking rewards are **restaked automatically** at payout time: instead of
landing in the delegator's free (spendable) balance, each reward is added to the
delegator's bonded stake and to the collator they delegate. All delegators
auto-compound; there is no keeper transaction and no per-delegator opt-in.

Collator rewards are unchanged — they are still paid as liquid balance.

## Behaviour change

| Before | After |
|---|---|
| Delegator reward → free (spendable) balance | Delegator reward → locked stake (restaked onto the same collator) |
| Delegator's stake constant between manual `delegator_stake_more` calls | Delegator's stake grows every round their collator is paid |

A reward earned for round `R` is paid during round `R+1` and, because the payout runs
in `on_finalize` (after that block's `on_initialize` snapshot), the restaked stake is
first captured by the round `R+2` snapshot — so it begins earning compounding rewards
from round `R+2`.

## Withdrawing rewards (users)

Rewards are no longer liquid by default. To turn restaked rewards back into spendable
tokens, unstake with `delegator_stake_less(collator, amount)`. The unstaked amount is
subject to the normal unbonding delay (`StakeDuration`: **14 days** on peaq, **7 days**
on krest) and then `unlock_unstaked`. Note `MaxUnstakeRequests` (10, of which 9 are
usable manually) caps how many pending unstakes you can have at once.

## Events (indexers / wallets / tax tooling)

Note the reward is still **transferred** from the pot into the delegator's account
(free balance goes up, and a `Balances` transfer still occurs), but it is immediately
**locked** — so the delegator's *spendable* balance does not change. Indexers that
track rewards by spendable/usable balance, or by the staking events, must adapt.
Three staking-pallet events now distinguish the cases:

| Event | Meaning |
|---|---|
| `DelegatorRewardRestaked(delegator, collator, amount)` | Delegator reward was restaked (auto-compounded). |
| `DelegatorRewardPaidNotRestaked(delegator, collator, amount)` | Delegator reward was paid as liquid balance because it could not be restaked (delegator no longer delegates the collator, candidate leaving, or inconsistent state). |
| `Rewarded(account, amount)` | **Collators only** now. Delegators no longer emit `Rewarded`. |

**Indexer migration:** any pipeline that tracked delegator rewards via `Rewarded` or via
delegator free-balance increases must switch to `DelegatorRewardRestaked` /
`DelegatorRewardPaidNotRestaked`. A delegator's free balance still increases (the
reward is transferred in), but it is locked, so spendable balance does not change.

## Invariants / safety

- Restaking a reward raises the `STAKING_ID` lock by exactly the reward and does **not**
  consume any pending `Unstaking` (a reward must never cancel a queued unstake).
- `try_state` (test / try-runtime) checks that every candidate's `total` equals its
  self-stake plus the sum of its delegators' stakes, catching any partial-write bug.
