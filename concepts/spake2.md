# SPAKE2

A password-authenticated key exchange (RFC 9382). Two parties that share a short
low-entropy phrase derive a strong shared key, and the phrase never crosses the
wire. A wrong phrase fails opaquely and leaves no partial state.

[[concepts/elephant|Elephant]] uses it for `theory join`, and
[[concepts/cbcl-bus|cbcl-bus]] for agent pairing. In both, the ceremony blocks
on a human passing the phrase out of band. That is why an agent cannot enrol
another agent on its own — see [[SPEC-002-leaderless-dispatch-loop#ADR-006]].
