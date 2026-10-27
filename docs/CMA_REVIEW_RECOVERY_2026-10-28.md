# Private review execution recovery, 28 October 2026

Three real private reviews reached terminal integration failures: audit responses
substituted a different report schema, advice used incompatible field types, and
one earlier response explicitly failed with `agent_command_queue_timeout` and
empty output. A `review_complete` submission can also have a failed letter:
that status records the applied private audit, not letter delivery.

New CMA runs pin `delivery_contract: 1` in their saved private transport state.
The final chunk repeats a bounded copy of audit/advice/letter instructions and
asks for their exact fields and types. Every original input byte is retained;
legacy saved runs keep contract 0, their original framing and request identities.
Formalization's prefix is not repeated because it has additional file-delivery
instructions after its data. All response parsing and domain checks remain in
force: this framing cannot guarantee a model will comply and never coerces a
missing mathematical verdict or fabricates evidence.

For contract 1 only, an exact terminal `agent_command_queue_timeout` with
explicitly empty output records its response id, chunk and observation time
before any successor request. That same chunk may be sent once more on the same
agent, with the durable `:queue-retry:1` key. Ambiguous transport replies reuse
that key and body. The original deadline bounds recovery; no agent is replaced
and queued/running commands, unknown failures and nonempty output never qualify.

The existing staff restart endpoint additionally permits a settled failed current
review of a `review_only` submission in `review_complete`. A fresh round and its
operational re-review trigger are saved together. Its source/version/revision,
prior rounds, letters, visibility and absence of a publication record persist.
Live rounds and fully successful private reviews refuse this path. Authors cannot
invoke the staff endpoint, and no artificial author response is created.

Regression coverage includes byte preservation, legacy framing, durable retry
identity across lost replies/worker restarts, refusal to duplicate live work,
deadline/retry bounds, and complete private recovery through letter delivery
without changing the original historical round or creating a public record.

A further observed failure committed one valid 54,762-character audit across two
assistant messages, with the boundary inside a string. Reading only the last
message lost the verdict and earlier claims. The response adapter now checks
bounded ordered message suffixes (at most 128 messages and 2,000,000 bytes) for
one complete JSON object under the existing escape rules. It inserts no text,
ignores tool output, preserves a valid last standalone answer, and never combines
separate responses. Invalid syntax remains a failure. Regression tests exercise
an internal string boundary, interleaved tool activity, prior commentary,
unchanged replay, conflicting standalone objects and the assembly size bound.
Historical saved answers and failed rounds remain unchanged.
