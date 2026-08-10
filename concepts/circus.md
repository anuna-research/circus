# Circus

The local execution harness this repository implements. It gives one coding
attempt a [[concepts/git-worktree|Git worktree]] and branch, and hosts the
process in a named [[concepts/tmux|tmux]] pane. It receives explicit completion
through [[concepts/withdone|withdone]], records the attempt, and serialises an
accepted merge.

It is not a planner, scheduler, daemon, code reviewer, or
[[concepts/elephant|Elephant]] client — see
[[SPEC-001-circus-agent-harness#REQ-007]].
