# CBCL

Common Business Communication Language: an agent communication language of typed
performatives in S-expression syntax. It is restricted to the deterministic
context-free language class, so message validity stays decidable while agents
define new dialects at run time.

Five invariants make self-extension safe: R1 no recursion, R2 resource bounds, R3
core preservation, R4 integrity, and R5 contract well-formedness. A dialect
definition is itself a valid CBCL message, read by the same parser that reads
ordinary traffic.

Implemented by [[concepts/cbcl-rs|cbcl-rs]], carried by
[[concepts/cbcl-bus|cbcl-bus]], and spoken by [[concepts/hark|hark]].
