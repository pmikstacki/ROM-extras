# Import transport implementation plan

> **For agentic workers:** Use superpowers:executing-plans task-by-task.

**Goal:** Acquire immutable approved import input and qualify exact recovery through public ROM.

**Architecture:** Default file admission and optional HTTPS transport feed the existing pure import core. Hosts retain source activation, actors and recovery records.

**Tech Stack:** Rust1.99, published ROM d7ef529, existing reqwest0.13.5/tokio1.53.1/url2.5.8/zeroize1.9.1. Node/OpenSSL controlled TLS and native Nginx.

**Spec:** ../specs/2026-10-09-import-transport.md

## Global constraints

Preserve public API paths, exact identities and original retry epoch. No implicit deletion, retry, credential discovery or browser grant. Preserve historical fixtures and logs.

## Review focus

- File byte bounds do not guarantee syscall deadlines.
- HTTP retained header bounds do not guarantee every parser allocation.
- Conditional server validators supplement the mandatory expected digest.
- Denied replay after Unknown must not be interpreted as rollback.
- Recovery uses frozen original bytes and host scope, without refetch.

### Task1: File and HTTP acquisition

Files: crates/rom-import-transport/src/{lib,error,file,config,etag,context,http,response}.rs; tests/admission.rs; crates/rom-import/src/{action,document}.rs getter; tests/import-transport-consumer; scripts/check-import-transport; package preparation and controlled TLS driver.

Interfaces: prepare_file(&ActionPlan, File); HttpConfig::new(endpoint,agent,interval,concurrency); HttpSource::new(config); StrongEtag::new(&str); RequestContext::new(Duration,watch::Receiver<bool>); HttpSource::fetch(&ActionPlan,Option<&StrongEtag>,&RequestContext).

- [x] Record failing public boundary tests before implementation.
- [x] Implement bounded file admission and explicit conditional HTTPS fetch into PreparedAction.
- [x] Execute actual TLS protocol failures and independent normalized archive consumer.

### Task2: Native HTTP and actual unknown recovery

- [x] Create an owned isolated persistent Nginx fixture with explicit TLS/auth configuration.
- [x] Execute native static representation/validator/auth/missing/restart checks.
- [x] Use published storage fault hooks and actual child-process exit; retain trusted original host request before submission.
- [x] Reopen SQLite/redb and prove exact replay, one effect, durable provenance and zero refetches.

### Task3: Integration gate

- [x] Review final increment once, audit dependencies, run affected checks and full local verifier on frozen source.
- [x] Record precise evidence and limitations; update guide/support; commit and publish.

This plan fulfills sourced-import Task2 only when its native and recovery checks pass. Full extras Task9 remains incomplete until SQL maintenance and blob qualification also pass.
