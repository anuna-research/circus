# cbcl-bus

The signed-member message bus. Every frame is signed per frame with Ed25519 over
a domain-separated envelope, bound to an audience and sequenced against replay.
There are no bearer tokens.

Two delivery disciplines sit on one admission core: routed dispatch for agents,
and fan-out for chat rooms where humans and agents share a channel. A third door
authenticates an external webhook, mints it into a [[concepts/cbcl|CBCL]] message
on the source's behalf, and marks it untrusted-external.

That marking is a trust boundary.
[[SPEC-002-leaderless-dispatch-loop#REQ-011]] states what a peer is allowed to do
with a message carrying it.
