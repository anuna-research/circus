# Leaderless Swarm

A set of agents working one specification where no member allocates work to
another. Readiness and ownership are conclusions read from a shared theory, not
transitions announced by a coordinator. A member's death removes a worker and
nothing else: there is no lease to reap, no queue to drain, and no supervisor to
restart.

Contrast [[concepts/hence|hence]], which coordinated through a supervisor that
forked workers under leases. See [[SPEC-002-leaderless-dispatch-loop]].
