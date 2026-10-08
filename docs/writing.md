# Write ROM documentation

Use ASD-STE100 Issue 9 as a guide for technical prose. Preserve the meaning of the source before you change its words.

The [ASD-STE100 skill](https://github.com/nuelcyoung/asd-ste100/tree/3fe2ccb51b1a98fb8d40aa88ec448575b8439ddb) supports this work. It is an unofficial aid, not a certificate of compliance.
The [official standard](https://asd-ste100.org/) remains the authority for its rules and dictionary.

## Preserve the contract

- Keep code, commands, identifiers, paths, and link destinations unchanged during a prose edit.
- Keep requirement strength. Do not change `SHOULD` into `MUST` or remove a condition from `SHALL`.
- Keep OpenSpec requirement names, scenario names, and task states unchanged during a prose edit.
- Keep dates, measured values, test counts, and evidence paths unchanged.
- Distinguish proposed behavior from implemented behavior. Distinguish source review from executed tests.
- Preserve limits and uncertainty. A successful test does not prove behavior outside its test conditions.
- Do not rewrite license text, third-party notices, quoted source text, or raw test output.

## Write instructions

Give one action in each sentence. Put the condition before the action. Use an imperative when the reader must do the action.

Aim for a maximum of 20 words per instruction. Split a long instruction into steps. Do not remove a condition to shorten it.

For example: “Before you publish the archive, run the package test.” Keep the actual command in its code block.

## Explain behavior

Give one topic in each paragraph. Name the component that does the work when the source identifies it.

Aim for a maximum of 25 words per sentence and six sentences per paragraph. Split long sentences without removing facts.
Use a table to compare alternatives. Use a list when it makes conditions or steps easier to read.

Keep technical terms when a common word changes their meaning. A commit is not a response. An unknown outcome is not a confirmed failure.
Distinguish runtime validation from a test of that validation. Do not rewrite an authorization check as an executed experiment.

## Use consistent terms

These terms come from ROM's existing contracts. They are technical vocabulary, not claims of approval by the ASD dictionary.

| Term | Meaning in ROM documentation |
| --- | --- |
| Resource | The domain entity that ROM manages through a shared contract. |
| action | An operation on a Resource. Do not use this term for every internal function call. |
| mutation | A change to Resource state. An attempted mutation does not imply a successful commit. |
| commit | The atomic persistence operation. Distinguish it from acknowledgement to the caller. |
| event | A fact recorded for a committed change. |
| receipt | A stored result that supports durable idempotency. It is not a temporary cache entry. |
| reaction | Work that follows an event and can invoke another action. |
| compensation | An explicit domain action that addresses an earlier effect. It does not erase history. |
| adapter | An implementation of a ROM boundary contract. |
| descriptor | Metadata that describes a Resource or its fields. It is not the Resource's current value. |
| live query | An observed query result that changes as authorized data changes. |
| journal | The event history available through the journal contract. |
| cursor | A position used to continue a stream. Keep it distinct from a query anchor. |
| query anchor | The continuation value for the query contract. |
| codec | The code that encodes or decodes a value. |
| authentication | The process that establishes identity. |
| authorization | The decision about permitted operations or disclosure. |
| unknown outcome | A result that does not establish whether the operation committed. |

Keep Rust names and names from wire formats exactly as declared. Keep names of libraries and products unchanged.
Treat “data” as a mass noun in new explanatory prose. Preserve quoted material and identifiers.

## Review the result

Compare each changed passage with its source. Confirm that every fact, qualifier, and obligation remains present.
Review word meaning and part of speech together. Do not use a word-list match as proof of compliance.

For a prose-only change, compare code blocks, identifiers, link destinations, numeric evidence, headings, and task states with the baseline.
Run OpenSpec validation after edits to OpenSpec documents. Review differences that an automated comparison reports.

Preserve normative keywords and technical terms when a vocabulary substitution changes the contract. Record these exceptions in the review evidence.

The current rewrite uses the skill's fallback references. Vocabulary was checked against known rulings and high-risk patterns only, not against the official ASD-STE100 Part 2 dictionary. Full compliance requires verification against the official standard.
This process does not certify compliance. A human writer remains responsible for final approval of the prose.
