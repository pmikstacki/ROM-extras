# Sourced action import

This increment implements Task 9 of the approved extras design. The whole maintenance family remains incomplete.

## Boundary

Use published ROM `d7ef529040eec60dc869034c2d33130219db85fe`. A transport-free `rom-import` validates JSON and constructs ordinary sourced action requests. The host owns source controls, service authentication, authorization, Resource selection and execution. No new receipt or Work ledger is introduced.

Two alternatives were considered: extending private ReloadTicket state, or repeating its controller. Both would duplicate published behavior or depend on private fields. Use public SourcePermit and Invocation instead. ReloadTicket remains the existing option for configuration Create/Replace.

Before fetching, the host prepares a managed generation and approves an expected SHA-256. A trusted ActionPlan captures exact target, action, target revision, idempotency, retry epoch, dependency revision, expiry and complete output attribution. Source version is `sha256:<hex>` of the expected bytes. SHA-256 detects a different representation; it does not authenticate the producer.

The plan verifies bounded bytes against that digest and parses through public `rom::parse_json`. Duplicate keys and trailing data fail. JSON roots may be objects, arrays or scalars as required by the selected action. The complete parsed value becomes its input without flattening or numeric conversion.

Admission defaults: 65,536 bytes, depth 32, 4,096 value nodes and 16,384 bytes per string or object key. Configurable limits remain at or below those hard ceilings. Bytes are checked before parsing. Structural checks occur after parsing; they are not streaming allocation limits. ROM retains its own smaller command admission limits.

A PreparedAction holds immutable Invocation and SourcePermit. Its request method clones that same pair for host execution and recovery. It does not choose an actor, renew grants, refetch, retry or allocate a new identity. Debug output contains no request data, keys, paths or grant labels. Finite admission errors contain no parser details.

The host attests output origins: action inputs cannot establish causality for computed output fields. Runtime validates complete output field coverage and current authority. Dependency or expiry changes can block replay of an already committed request. Such rejection is not proof of rollback. The host must resolve Unknown without silently replacing the original request.

## Evidence required

Admission tests cover exact bytes, depth, nodes, strings, duplicate keys, trailing input, integer precision, absent/null/false/zero/empty distinctions and redacted diagnostics. Independent consumer tests execute a domain action on real SQLite and redb through only public API. They inspect durable provenance, exact replay, target/control conflicts and denied actors.

This increment does not qualify file/HTTP transport, interrupted acknowledgement, process recovery, SQL archives, external blobs or migrations. Those remain later tasks. No production service is provisioned.
