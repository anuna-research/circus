# Rendezvous Hashing

Also called highest-random-weight hashing. Each participant scores a key by
hashing the key together with its own identifier, and the participants sort by
that score. Every participant computes the same order from data it already
holds, so agreement needs no message.

Removing a participant changes the order only below the removed entry, so work
redistributes without reshuffling. Different keys give different participants the
top position, so load spreads with no balancer.

[[SPEC-002-leaderless-dispatch-loop#REQ-002]] uses it to decide which peer claims
a task first, and [[SPEC-002-leaderless-dispatch-loop#ADR-005]] records why it
was chosen over random backoff.
