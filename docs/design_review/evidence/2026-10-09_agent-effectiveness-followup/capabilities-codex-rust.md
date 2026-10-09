# Codex and Rust capability evidence

Date: 2026-10-09. Bounded library-research contribution to the agent-effectiveness follow-up. Recommendations below are **Proposed**; inspection and metadata receipts establish only their stated scopes. No productivity, model quality or performance improvement was measured.

Consumed by the [principal review](../../reviews/design_review_agent-effectiveness-followup_2026-10-09.md); this document supplies capability evidence, not an independent overall verdict or disposition ledger.

## Boundary and evidence levels

The brief identified HEAD `d2341cc4`; during inspection HEAD became `6c9755226a2c6960e4680a9db644e52f2abb3808`. The working tree held substantial concurrent product changes. Source pointers describe the current mixed tree inspected on this date, rather than a clean checkout of either commit. Nothing was reverted. Only this document was written. No builds/tests, installation, synchronization, service starts, fresh agent sessions, MCP registration, operator inspection, configuration or shared-skill edits were performed.

Levels used here: **documented** means a fetched primary contract; **installed** means executable/version or files observed; **configured** means declarations inspected; **exposed** means present in this worker's actual tool catalog; **exercised** means used here with a bounded result. Installed CLI versions do not identify the executable hosting the already-running desktop/session server.

The shared worker and library-research contracts were read. Applied skills: `openai-docs`, `context7-mcp`, and `rust-code-model`. Context7 was the first documentation lookup for nextest, rust-analyzer, sccache, Cargo and rustdoc. Installed help and local versioned sources refined those results. The official Codex documentation was fetched after official documentation search. A quick memory pass suggested ignored-config/version drift as an investigation lead; present claims were rechecked against current files/help.

## Current strengths and useful opportunities

| Consumer task | Capability and current evidence | Useful next use and cost |
|---|---|---|
| Keep a long compile/test observable while continuing independent work | Native `exec_command` returns a process `session_id`; `write_stdin` resumes it. Code-mode `functions.exec` has a separate running **cell_id** resumed by `functions.wait`. Both are exposed; short native shell calls and code-mode composition were exercised here. | Use native continuation for work within the current session; retain the correct handle type. Polling is observation, not a workload duration limit. No extra launcher needed. |
| Resume verification after compaction/session boundaries | `scripts/runs.py:10` owns durable `record.json`, `output.log`, progress and cancellation; `:24` explicitly gives runtime facilities priority. `scripts/verify.py:1551` owns verification `summary.json`; `:1514` consumes it for rerun. Configured repository facilities, source inspected; no run started here. | Reuse `just runs status RUN --json`, logs, and `just verify --rerun RUN`. Durable ownership answers a real cross-session need; adding a second task registry would duplicate that authority. Existing launcher adds a process/file lifecycle only when durability is useful. |
| Explain configuration discrepancies | Installed `codex-cli 0.162.0` has `--strict-config`, `codex doctor --json` (redacted), `mcp list/get --json`, features inspection and `debug prompt-input`. Help inspected; these diagnostics were not invoked beyond help. | Use the native diagnostic appropriate to the discrepancy. `mcp list` reports **configured** servers, not session readiness. Avoid a new config validator/wrapper. Doctor may inspect runtime/auth connectivity, so decide its scope separately. |
| Find Rust definitions and perform bounded refactoring | `rust-analyzer-mcp 0.4.0` installed/configured/exposed; tools include symbols, definitions, references, hover, completion, diagnostics, code actions and rename. Source inspected; semantic requests intentionally not exercised here. | Symbols/definition/hover add exact local semantics; rename returns edits for review/application and waits for workspace load in the inspected source. First semantic use can start rust-analyzer, load Cargo/sysroot/macros and trigger checks; choose it for a real consumer, not an automatic startup ritual. |
| Locate package/test ownership | `cargo metadata --locked --offline --no-deps --format-version 1` **passed** here: 14 packages, 14 members, `resolve: null`, local target directory and named serving test targets. | Use metadata to stop guessing file/target layout. This command builds nothing and does not fetch dependencies. A complete resolved dependency graph requires another invocation/contract; do not infer one from `--no-deps`. |
| Interpret an authorized compiler failure precisely | Installed Cargo/nextest expose diagnostic JSON. Cargo messages distinguish compiler diagnostics, artifacts and build completion. | Prefer JSON from the already-authorized affected compile to log regex. Preserve package, target, spans and diagnostic codes. Compiler-artifact `fresh` distinguishes reuse from a rustc invocation. Build-finished is not test success. Output from other tools can still be non-JSON. |
| Select exactly the controls intended | Installed nextest 0.9.146 supports package/target selection and filtersets; list JSON is documented and installed. Repository verification already derives test discovery from nextest (`scripts/verify.py:707`). | For uncertain filters, inspect the plan with `just verify --print --json`; when compilation is authorized, discover through nextest JSON. `list` builds test binaries and queries them; it is not a cheap read-only source listing. |
| Attribute slow compilation | `compile_profile.py` already composes Cargo build analysis, timings, selected rustc self-profile and sampling with `runs.py`. Exact measureme 12.0.3 readers and an install receipt are present. | Read existing capture status/report before recording another workload. Use existing focus/run ownership rather than another profiler wrapper. Report/view can write derived reports or launch a local viewer; recording perturbs compilation and owns capture state. No new recording or report was produced here. |

These options fit heuristics H4/H5/H6/H8 in `docs/design_review/design_principles/core/efficient-architecture-heuristics.md:22`: delegate complete native capabilities, ask for the smallest sufficient scope, and preserve intent. They increase useful abilities as well as reduce coordination friction.

## Codex configuration and runtime are different observations

Current project `.codex/config.toml:1` sets Astra/high coordinator defaults, Sol/high default subagents, 65,536 project-document bytes, and `UV_NO_SYNC=1`. User `/home/paul/.codex/config.toml:1` sets never/full-access, Sol/xhigh, and allows login shells; `:56` declares the rust-analyzer command, with Context7 immediately below. The actual worker catalog exposes rust-analyzer, Context7 and official OpenAI docs; Context7 and official-doc calls succeeded here. This establishes more than declarations for the latter two, but no fresh semantic result for rust-analyzer.

Official [configuration basics](https://learn.chatgpt.com/docs/config-file/config-basic) describes CLI overrides above trusted project config, selected profile, user, cloud/system defaults and built-ins. A live explicit user/runtime choice therefore takes precedence over a repository default. [Configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference) documents unified exec, shell snapshots and feature configuration; the supplied session tool schemas are the strongest evidence of the mechanisms available **here**. These docs and CLI help do not prove a configuration edit changed an existing session.

`.agents/roles/README.md:24` already states that role model/effort routing is policy, not measured quality/cost, and preserves explicit user choices. No model calibration, fresh-session comparison or expanded Claude workflow is justified by this evidence. Current shell `codex --version` reports the binary at `/home/paul/.local/share/jobsearch-os-toolchain/codex-bin/codex`; a daemon/session can have a separate launch lifetime. Installed Codex implementation source was not located in the scoped installed package paths, so CLI help plus official docs support contracts, rather than an exact internal-source audit.

**Proposed:** for a concrete settings mismatch, first compare current session tool/settings evidence with native configured diagnostics. Fix an unsupported setting only when the discrepancy is established. Do not turn every task into a configuration inventory or require a fresh agent startup.

## Rust semantic capabilities and readiness gap

Installed `rust-analyzer --version` reports `1.101.0-nightly (c1070d6 2026-09-28)`; pinned `rust-toolchain.toml:4` selects nightly-2026-09-29. The version is not an `ra_ap_*` crate version. Installed adapter version is 0.4.0. Local registry source is under `/home/paul/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rust-analyzer-mcp-0.4.0/`; its provenance-to-binary equivalence was not rebuilt or hash-qualified here.

Decisive local pointers:

- `src/mcp/server.rs:50`: semantic tool calls lazily ensure/start a client and recover a gone client.
- `src/mcp/handlers.rs:115`: references opens the document and forwards immediately; `src/lsp/handlers.rs:69` issues `textDocument/references` with the declaration included. Neither path calls `wait_until_loaded`.
- `src/mcp/handlers.rs:221`: rename refreshes open documents, waits for loading, prepares rename, and returns the edit. `src/lsp/handlers.rs:277` observes the quiescent watch.
- `src/mcp/tools.rs:98`: rename's exposed contract says no files are changed and loading can make edits incomplete. The inspected exposed surface does not include workspace-symbol, call hierarchy or a dedicated status tool; this is a bounded tool-surface claim, not absence from rust-analyzer itself.
- `src/lsp/client.rs:332`: progress/serverStatus are negotiated. `src/position.rs:3`: positions are zero-based UTF-16 columns, not `rg` byte offsets.

Official [rust-analyzer LSP extensions](https://github.com/rust-lang/rust-analyzer/blob/master/docs/book/src/contributing/lsp-extensions.md) distinguish health (`ok`, warning, error) from quiescence. A warm/quiescent server can still have missing-dependency warnings. AGENTS.md records prior early empty reference/workspace-symbol answers; no fresh request was made here to reproduce them.

**Proposed:** use existing semantics for symbol/type/method questions and reviewable rename edits. Treat cold empty reference results as inconclusive; use source search to continue the task. If this repeatedly obstructs reference consumers, improve the existing adapter's readiness/status contract rather than inventing a parallel index. A useful gap is returning whether a result was obtained after workspace loading and with what health; it needs focused qualification before adoption. Do not use workspace diagnostics merely to force indexing: that can initiate/wait for compile work, and diagnostics cleanliness is a different contract.

## Cargo, nextest and cache contracts

Installed nextest is `0.9.146 (8af696ddc, 2026-09-21)`. [List JSON](https://nexte.st/docs/machine-readable/list) is distinct from [experimental libtest run JSON](https://nexte.st/docs/machine-readable/libtest-json). Installed run help supports `libtest-json`/`libtest-json-plus` and a format-version option; fetched docs require the experimental environment opt-in. No JSON test stream was exercised. Prefer the repository summary for durable verification outcomes; consume nextest's native stream only for a concrete richer event consumer. Doctests remain a separate Cargo control; list membership is not execution evidence.

Cargo's [external-tools contract](https://doc.rust-lang.org/cargo/reference/external-tools.html) and installed help support compiler-message/artifact/build-finished composition. [Metadata](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html) supports a versioned schema. The successful bounded metadata invocation here demonstrated that manifest ownership/target discovery is immediately useful without a new library or service.

Installed sccache is 0.17.0; `.cargo/config.toml:2` makes it the wrapper. `Cargo.toml:140` and `:165` enable workspace incremental compilation and disable it for imported dependencies (`:145`, `:170`). [sccache Rust documentation](https://github.com/mozilla/sccache/blob/main/docs/Rust.md) says incremental rustc work cannot be cached and linker-invoking crate kinds are not cacheable. Therefore low cache hits for incremental workspace edits would not by themselves diagnose a broken wrapper. Do not propose disabling incremental solely to improve a hit counter.

Installed help exposes `sccache --show-stats --stats-format json`. **Proposed:** when an existing server is known live and cache attribution is needed, compare its statistics around an authorized representative build, including uncacheable reasons and Cargo fresh artifacts. No stats command was run here: the assignment prohibited service starts, and the scoped installed sccache source search did not establish that a stats query never auto-starts a server. Shared cumulative counters do not attribute another agent's activity to this task. No cache-reset, stop/start or cache-effectiveness measurement is warranted.

## Introspection and existing profiling support

The shared `rust-code-model` skill pins ra_ap 0.0.352, rustdoc-types 0.61.0, cargo_metadata 0.23.1 and nightly-2026-09-13. Its `content/topics/00-map.md` separates public API, syntax, HIR, MIR/dataflow and package context. This is useful discovery guidance but differs from the checkout nightly. Do not transfer its executed compiler probes as current qualification.

**Proposed:** use rustdoc JSON for public API/signatures/docs/trait implementation questions, HIR for name/type resolution, and compiler MIR/expansion only when a concrete macro/control-flow question needs it. Rustdoc JSON has no function bodies, and omission can depend on visibility/cfg. Its format version must be inspected in the artifact; `rustdoc-types`' version alone is not an artifact guarantee. Official [rustdoc unstable features](https://github.com/rust-lang/rust/blob/main/src/doc/rustdoc/src/unstable-features.md) documents JSON's unstable gate. Local `rustc -Z help` confirmed self-profile, MIR dumps and unpretty switches without compiling. These extractions can compile/load macro dependencies and produce artifacts; they were not executed here.

`scripts/compile_profile.py:64` composes only recognized direct Cargo invocations and separates Cargo diagnostic flags from nextest result flags. `:971` guards measureme format 9; `:994` checks exact readers before deriving reports. `scripts/compile_profile_tools.py:14` pins version, revision, nightly and format; `:44` checks receipt and executable hashes. The present `/home/paul/.local/share/compile-profile-tools/measureme-12.0.3/install.json:2` matches these declarations and binaries exist, but the hash-check command was not run. `compile_profile.py:942` reports active units, lifecycle, completed chunks and read-only recent hotspots. This is a useful existing consumer; another compilation telemetry registry is unnecessary. Performance conclusions still require authorized representative measurements.

## Command-construction lessons for the retrospective

1. **Keep handle namespaces separate.** Native PTY process `session_id` goes to `write_stdin`; code-mode `cell_id` goes to `functions.wait` only after the running-cell result. Output budgets and poll windows govern observation, not CPU/test concurrency or workload duration.
2. **Preserve argument ownership.** Nextest `--cargo-message-format` controls compiler output; `--message-format` controls nextest list/results. Run JSON values differ from list JSON values. `--` introduces emulated libtest arguments, not arbitrary Cargo flags.
3. **Avoid accidental serial execution.** Installed nextest help explicitly says `--no-capture`/`--nocapture` runs tests serially. Do not add it merely to obtain visible output. No resource/thread caps are proposed.
4. **Discovery can build.** Installed nextest list help says it builds/query test binaries; `scripts/verify.py:1411` distinguishes static `--print` from compiling `--list`.
5. **Scoped selections are positional.** `scripts/verify.py:1357` shlex-splits each tool-argument string and attaches it to the preceding selection (before the first: shared). Shell quote the filter as one intended value; use `--print --json` to inspect expansion before an authorized compile.
6. **Run launchers take argv.** `scripts/runs.py:435` removes its separator and executes argv directly. Shell operators, redirections and `NAME=value` are not standalone executables/arguments with shell semantics there. Use `env NAME=value command` for an explicit environment or a deliberately quoted shell only when shell composition is needed. `subprocess` argv and `shlex.join` are already used by the owning scripts.
7. **Configuration quoting is another language boundary.** Installed Codex help says `-c key=value` parses TOML then falls back to a literal string. Preserve quotes through the shell; do not infer that a string-valued override became an array. Native `--strict-config` addresses unknown keys without a homegrown schema checker.
8. **Do not reinterpret output absence as success.** Nextest help defaults no-tests to failure (code 4 unless overridden); preserve that useful behavior. Metadata `resolve: null`, cold empty references, stale diagnostics and artifact-only compiler completion each have narrower meanings than the broader result one might want.

## Check receipts and remaining uncertainty

**Passed (2026-10-09):** `codex --version`, `cargo nextest --version`, `sccache --version`, `rustc --version`, `rust-analyzer-mcp --version`, `rust-analyzer --version`; inspected help for Codex, Cargo, nextest and sccache; `cargo -Z help`, `rustc -Z help`; bounded Cargo metadata command above; Context7 and official-doc retrieval. These are capability/version inspections, not product or tool-performance acceptance.

**Not_run:** semantic MCP requests, fresh-session config application, doctor runtime checks, nextest discovery/execution JSON, doctests, sccache stats/cache workloads, full reader hash validation, profiling/report/view, rustdoc/MIR extraction, product tests/builds and turn-end. No blocked prerequisite was repaired or bypassed. Remaining material uncertainty is live host/session settings, exact installed adapter source provenance, readiness/result behavior under this current mixed tree, future compiler format compatibility, and whether richer structured-result consumers justify any integration beyond current native composition.


## Implementation qualification, 2026-10-09

The follow-up implementation exercised the installed semantic MCP against the disposable
`build/agent-followup-semantic` two-file Cargo package. References returned declaration and call;
function rename returned two reviewable edits, module rename included a file operation, and a
non-symbol position was refused. No edits were applied; source bytes were unchanged and workspace
selection was restored. This extends the authoring evidence only for these scratch requests.
[Current task routes and limits](../../../plans/agent-effectiveness-followup-plan_2026-10-09.md#10-task-owned-command-examples)
own the guidance; no full-workspace indexing or performance claim follows.
