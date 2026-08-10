# hark

A per-user daemon and a short-lived CLI that connect one agent to
[[concepts/cbcl-bus|cbcl-bus]]. The daemon owns the WebSocket and the inbound
queue; the CLI is a thin loopback client that blocks for one message or sends
one.

It runs [[concepts/cbcl-rs|cbcl-rs]] at both boundaries. A message reaching an
agent has already passed R1–R5, and a malformed outbound message never leaves
the machine.

hark carries messages. It selects no work and decides nothing about content.
