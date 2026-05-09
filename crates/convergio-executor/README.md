# convergio-executor

Layer 4 reference dispatcher.

`Executor::tick` finds pending tasks whose wave is ready, spawns a local
worker through `convergio-lifecycle`, and moves each successfully spawned
task to `in_progress` with the spawned process id as `agent_id`.

A failure to dispatch one task does not abort the tick: the task stays
`pending` and the executor continues trying later tasks.
