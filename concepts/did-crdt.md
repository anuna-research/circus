# did:crdt

The identity scheme [[concepts/elephant|Elephant]] uses. One Ed25519 key pair per
agent, resolved through a conflict-free replicated document rather than a
registry, with no networking in its core.

A DID is the unit of trust in a theory. It is why attribution is cryptographic
rather than declared. A member cannot assert as another member, so adversarial
review is checkable from the corpus instead of taken on trust.
