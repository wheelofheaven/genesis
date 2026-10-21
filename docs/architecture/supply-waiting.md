# Household supply waiting

21 October 2026, following resident camera tracking.

## Playable change

A failed supply search no longer sends a resident on a social visit when their
home lacks enough food or water for the next consumption cycle.
At an existing home-departure boundary, the resident tries household jobs using
the existing food/water priority, then optional crop tending.
If none is available and delivered stock does not cover the next use, they enter
**Waiting for household supplies** at their home threshold.
They retry every two logical seconds.

A resumed well, newly mature crop, released reservation or restored route can
allow an ordinary collection task at a retry.
Covered homes keep their social routine, even when below refill thresholds.
The rule counts actual residents of the home: one food portion and half a water
unit each. Cargo, source stock and another home's surplus do not cover consumption.
Crop-tending work is still allowed, so waiting does not itself prevent production.

The resident inspector shows the retry countdown and existing last-attempt issues.
Water and Food household rows show the same committed countdown.
Pause stops it and single stepping advances it with the simulation.
There is no new health, hunger, fatigue or mortality system.
Residents already traveling, gathering, carrying or tending finish their existing
commitments. Existing rest periods are retained; the rule changes the decision
at departure, and does not recall people from social visits already underway.

## Simulation contract

`people/supply_wait.rs` owns home-departure eligibility, shortage waiting and its
validation. It uses the existing `remaining_ms` timer and adds the
`Activity::WaitingForSupplies` variant.
Waiting residents have no task, cargo or route and stand at their home threshold.
Unsafe ground keeps the existing blocked behavior; source searches resume only
when the resident can stand safely.

Task selection remains ordered by persistent resident identity and uses existing
bounded source/route searches. Waiting does not reserve anything independently.
Repeated failed retries do not emit routine-change events unless activity or
blocked status changes. Existing issue-change events remain intact.

## Save compatibility

New captures use schema **11**, rules **`genesis:rules/9`**.
V1–V10 migration remains staged. V10 migration preserves all materialized domain
state, clocks, pending intent, world identity and expedition role; new behavior
begins when the world advances under current rules.
Old envelopes containing the new waiting activity are rejected.
New waiting saves reject impossible positions, cargo, routes and timers above
2000 ms. Successful round trips preserve the precise retry phase.
The pre-change `storage/fixtures/v10-conserving.json` protects migration of active
pumping, cultivation and resident commitments.

## Acceptance

Five simulation tests cover exact retries and event silence, well recovery,
available food and stable ordering, actual shared-home demand, and real crop growth.
Existing task/conservation tests still cover active food, water and irrigation
commitments. Social-route tests now start with covered homes and retain their
movement and obstacle assertions.
Two storage tests cover V10 preservation, pending intent, waiting continuation
and invalid saves. A UI test checks both supply panels and the resident countdown
without advancing paused state or queuing actions.

CPU reproduction:

```sh
target/debug/genesis-headless --scenario water --ticks 900 --commands crates/sim_core/fixtures/supply-recovery.commands.json
```

The fixture stops pumping at tick 1 and resumes at tick 121.
All six residents are waiting at tick 65; every home has consumed water by tick
900. Saving at tick 65 and continuing with the resume command matches an
uninterrupted run exactly in materialized state and logical tick.

Native reproduction:

```sh
target/debug/genesis-app --supply-wait-test /private/tmp/genesis-supply-wait.png
```

This authored six-home fixture checks waiting, recovery after a well resumes,
and exact restoration, then captures the waiting inspector.
It uses a temporary save beside the capture, separate from the player's slot.
