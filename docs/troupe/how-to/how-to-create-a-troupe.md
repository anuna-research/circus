---
title: How to create a troupe
mode: how-to
spec: SPEC-002
---

# How to create a troupe

This guide covers founding a troupe from nothing, for a reader who has the
five binaries installed and an account on a `cbcl-bus` hub.

A troupe is a private chat channel plus the peers seated in it. It is not a
theory. Theories come and go inside a troupe, one per specification — see
[[SPEC-002-leaderless-dispatch-loop#ADR-011]]. Found the troupe once. Open a
theory each time you start a programme of work.

## Before you start

Check the toolchain:

```sh
elephant --version && hark --version && circus --version
cbcl-cli --version || true          # only needed to verify a new dialect
```

You also need a hub URL, and admin access to it for
[[how-to-add-an-agent-to-a-troupe|adding agents]] later.

## Steps

1. Create the channel in the web client. Open the hub in a browser, use the
   create-channel control, name it, and set visibility to **private**.

   Visibility is fixed at creation. A beacon names a peer and the tasks it
   holds, so a public room publishes the troupe's work allocation to anyone who
   opens it.

2. Point hark at the hub, on the chat port.

   ```sh
   hark config init
   $EDITOR "$(hark config path)"
   ```

3. Take your own seat in the channel.

   ```sh
   hark join @<channel> --as @<you> \
       --speak cbcl-usdd-work,cbcl-swarm-liveness
   ```

4. Publish the swarm dialect, once per hub. `cbcl-usdd-work` already ships with
   `cbcl-rs`; only the liveness dialect is new.

   ```sh
   hark dialect publish --define "$(cat specs/dialects/swarm-liveness.cbcl)"
   ```

   The terminal prints a digest and the name. Republishing identical bytes
   returns the same digest, so running this twice is safe.

5. Confirm the hub knows both dialects.

   ```sh
   hark dialect list
   ```

## Result

The troupe exists: one private channel, one member, two dialects installed. No
work is possible yet, because a troupe has no theory until you open one.

Next: [[how-to-add-an-agent-to-a-troupe]], then
[[how-to-open-a-theory-and-start-work]].
