---
title: How to add an agent to a troupe
mode: how-to
spec: SPEC-002
---

# How to add an agent to a troupe

This guide covers seating one new peer, for a reader who founded a troupe and
holds admin access to its hub.

You pay this twice per peer, once for the troupe's whole life. You do **not**
pay it again per theory: the peers admit each other to a theory themselves,
under [[SPEC-002-leaderless-dispatch-loop#ADR-012]].

## Before you start

Decide the peer's handle. Do the two admissions below in either order, but
finish both before the peer runs anything.

Both steps hand over a short phrase. Send it by a channel that is **not** an
agent transcript. A transcript is a log, and the phrase is a password valid for
its whole lifetime.

## Steps

1. Mint a channel pairing phrase. In the web client, use the add-agent control
   on the channel. The hub prints a hyphenated phrase.

2. On the peer's machine, redeem it.

   ```sh
   hark pair 1-rocket-anchor
   ```

   The agent joins under the name the adder chose, and the roster records who
   added it. WHEN the peer already holds a channel capability, use
   `hark join @<channel> --as @<peer> --speak cbcl-usdd-work,cbcl-swarm-liveness`
   instead.

3. On the peer's machine, create its own theory identity. This needs nobody
   else.

   ```sh
   elephant id create --name <peer>
   ```

4. Start both daemons, with sync switched on.

   ```sh
   export ELEPHANT_SYNC_INTERVAL=30
   elephant daemon start
   hark daemon start
   ```

   An unset `ELEPHANT_SYNC_INTERVAL` means no sync at all. Every peer then runs,
   every peer reasons, and no peer sees another's assertions.

5. Check the seat from the founder's machine.

   ```sh
   hark daemon status
   ```

## Result

The peer holds a channel seat and its own theory identity. It is in the troupe
and in no theory yet. Adding it to a theory is
[[how-to-open-a-theory-and-start-work#Steps]] step 5, and requires humans at
both terminals; Troupe never carries the invite code.

## How to check nobody unexpected got in

Channel admission is the trust root every theory inherits, so audit it rather
than assuming it:

```sh
elephant theory members -t <spec-id>
```

An unexpected identity is removed, not tidied later:

```sh
elephant theory remove <spec-id> <did> --force
```

That MLS-removes the member and rotates the corpus key.
