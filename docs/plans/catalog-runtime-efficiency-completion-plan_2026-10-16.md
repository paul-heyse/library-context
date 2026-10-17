# Catalog runtime efficiency: publication and functional completion

**Accepted target / implemented source, integrated acceptance pending, 2026-10-16.** Companion to the
[coordinator](catalog-runtime-efficiency-plan_2026-10-16.md), whose §6 alone owns current finding
disposition. This document specifies runtime F06/F07, preserves exact inherited evidence and
defines RE80/RE90/RE99. Retained receipt observations below do not establish fresh executor state.

### Implementation checkpoint

RP00/RP01 are integrated on `main`: selected backup shares exact descriptors inside its one
snapshot, then reaches checked source EOF and transaction/session/pin finality before independent
local carrier construction. [ADR-0178](../adr/0178-atomic-bounded-native-effects.md) governs native
protection/recovery; [ADR-0179](../adr/0179-joined-owned-producer-execution.md) and
[ADR-0180](../adr/0180-shared-exact-base-preparation.md) govern the joined compiler and shared Base
contracts with which publication composes. The
[integrated review](../design_review/reviews/design_review_catalog-runtime-efficiency-integrated_2026-10-16.md)
accepts the inspected composition at its declared source boundary. RE80 default regeneration
passed as detailed in §5. RE90 selected-complete acceptance and RE99 final handback remain pending;
their outcomes and sole finding disposition belong to the coordinator.

## 1. Publication responsibility and suitable foundations

`lctx-publisher` consumes privately admitted exact native selections and owns coherent unselected
publication, selected backup and independent restore. `lctx-native` retains protected copies,
incarnations, pins, transactions and session finality. The
[storage/publication owner](../design/sections/storage-and-publication.md#section-6-1), ADR-0145's
carried decisions and ADR-0178 govern the implemented boundary. Model transport declares complete
logical/physical correspondence; a local archive cannot create receiving validity.

The existing selected backup already retains all root-qualified physical copies and uses one
snapshot with owned spools. Correct repeated dependency descriptor queries and the snapshot's
unnecessarily long lifetime; do not replace backup with a server dump that loses selected closure,
an old-generation copy or a transaction spanning local carrier work.

<a id="2-inherited-checkpoint-and-protected-obligations"></a>

## 2. Historical inherited checkpoint and obligation provenance

This section preserves dated evidence and the identities behind original outcomes. It is not a
list of still-open default-target cleanup actions: the RE80 regeneration in §5 separately settled
the inherited protected attachment obligations through authenticated shutdown/discard. Original
failed/Unknown observations remain unchanged, and historical passes do not qualify the new source.

### Target and dated acceptance

The previous efficiency CE80 default regeneration passed in
`20261016T154634.590Z-fe4647`. Installation `58abe9ea-2088-4109-95ab-8440a3749c51` then used generation
`880b51cb809f3b0697ff56a45a4f6efb22d758b5eab076bcd88b2c6a4ff80fcf`, required patched peer
`9fbab612b96f62d2e88070ffd2cb7f1d8ec7b7a3a5cf304592c1c041ed3c761e` and regeneration journal
`fd0f108d2ed2e72c2a70b18026f77f00344a7429e17cebdc5f9a5e463d055bea`. These identify the inherited
baseline superseded by §5, not instructions to reinstall or recreate E1. Reobserve actual
prerequisites before authorized execution; generated format8/physical completion v2 and current lexical definitions
remain subject to their existing runtime checks.

| Retained run | Outcome and evidence limit |
|---|---|
| `20261016T191550.601Z-86bc68` | Full workspace/all-target release check passed; current-source metadata, not native behavior |
| `20261016T193501.402Z-c13b5c` | Full strict keep-going Clippy passed with `-D warnings`; host cleanup confirmed |
| `20261016T193501.752Z-e692b8` | 335 model library controls passed / 0 skipped; integration/doc/compile-fail and native boundaries remain separate |
| `20261016T173929.279Z-6056dd` | Python: 1,428 passed / 1 skipped at its dated source scope; no subsequent Python production edit in inherited checkpoint |
| `20261016T185531.584Z-647adb` | Six demand-catalog controls passed with enclosing completion |
| `20261016T185824.660Z-c8359a` | Five borrowed-owner controls passed with enclosing completion |
| `20261016T184944.985Z-2d7145` | Six provider bodies passed, including >60s outliers; enclosing ingress finalizer failed; no enclosing pass |
| `20261016T190337.280Z-029373` | BM25 passed; paired layout, Base, SourceCall, ordinary/recursive Summary and selected backup timed out after 900 seconds; binaries predate last finality repair |
| `20261016T191228.173Z-9223fb` | Original 2 passed/1 failed model-control result retained; subsequent corrected model-lib pass is separate evidence |

Last finality implementation separates ordered operation errors from actual driver joins and
finalizers, retaining typed JoinError and nested Unknown. The corrected large-binding native
control now earns an actual InputRevision contribution through its factory/pool; its execution
was not_run at that inherited checkpoint. Process-only compiler protocol controls construct
their signal owner once;
actual constructors still earn receipt-backed ownership. These are source-accepted foundations
with the exact remaining functional controls carried into RE90.

### Default-target attachments protected at the inherited checkpoint

The following sixteen attachments were named by the previous coordinator and were unreleased
in retained attachment records inspected at the inherited checkpoint on 2026-10-16. They bound
the delivered default generation above. Complete/Started counts are stored record observations, not proof of current live work or
actual executor drainage. Fourteen records explicitly retain native Unknown and confirmed command
cleanup; the two broad interrupted records omit aggregate native/host fields and require their
managed run and individual owner/drain receipts. No successful cleanup is inferred from omission.

| Attachment ID | Managed run suffix / retained observation |
|---|---|
| `8a037d17-6cf2-487d-a511-e616c2bb797b` | `154844.226Z-8b8832`; broad interrupted acceptance, aggregate native/host fields absent |
| `b323b570-a542-42fa-a01d-1f059c17b983` | `161916.748Z-3d119e`; Unknown,7 Complete/1 Started |
| `46f7963b-c310-4b75-9251-38ebf3fd575a` | `163056.893Z-d12e09`; Unknown,2 Complete/4 Started |
| `af1e7850-0574-4b49-bd0a-14a4c8cc8b51` | `165017.959Z-d15f80`; Unknown,2 Complete/1 Started |
| `adcb090e-4ad9-4dd5-a0f5-1664bcd8a420` | same `d15f80`; Unknown,2 Complete/4 Started |
| `9acf7fa8-7321-40e8-b346-abc0150d9316` | same `d15f80`; Unknown,0 Complete/6 Started |
| `917d4f86-8d3e-4373-a9a2-46eb03c5701c` | `171004.131Z-7ea2d9`; Unknown,4 Complete/8 Started |
| `26cd9420-134d-4c69-958c-ed3719c8b298` | same `7ea2d9`; Unknown,0 Complete/1 Started |
| `678cfee7-f332-4b22-a203-a5ff4ac1db1f` | `173259.745Z-b28768`; Unknown,0 Complete/6 Started |
| `3f83a807-caec-44b1-879e-0ea0e3e35a9d` | `175338.931Z-b83a1e`; Unknown,0 Complete/8 Started |
| `b824ac0b-ac80-4a95-ad8d-e1ded90dd673` | same `b83a1e`; Unknown,2 Complete/0 Started; Resource-negative diagnostic failure |
| `3bc97416-0581-4f01-953a-ab1364e57d7f` | `180032.094Z-21bab5`; stale interrupted compiler acceptance, aggregate native/host fields absent |
| `34b6ac6e-b532-4276-bc71-75354522fc68` | `181032.595Z-69ebfa`; Unknown,14 Complete/2 Started |
| `d586ccfe-b8ec-4170-83e1-f2860c05d7d8` | `184944.754Z-c37fab`; Unknown,2 Complete/1 Started |
| `1964f2e8-94cf-4911-a5da-3b7effbd520d` | `184944.985Z-2d7145`; Unknown,16 Complete/0 Started; enclosing finalizer failure |
| `f67a78df-8cc9-45e3-95d3-70b2761074f9` | `190337.280Z-029373`; Unknown,12 Complete/8 Started |

All abbreviated runs in this table have prefix `20261016T`. Exact records remain under
`/home/paul/.local/state/library-context/surrealdb/attachments/<ID>/record.json`, with their native
owners/pending actions in the same existing receipt scope. RE80 used the actual owner routes and
separate discard acknowledgements to settle protected obligations; it did not turn their original
native observations into successful logical cleanup. A stale record neither proves a daemon is
live nor permits disposal. This historical list covers named inherited default obligations; it
is not a complete installation inventory or authorization to inspect/reset operator content.

### Disposed targets and predecessor acknowledgements

E1 installation `e1c54ff4-396c-425a-8b36-f49a753055b9` disposal
`20261016T154532.732Z-740258` records **drain failed** and **executor shutdown, datastore disposal,
attachment release, installer/unit/storage/credential removal passed**. Preserve both observations.
The disposed attachment set includes `04cdcb69-531e-41e2-b9c6-377a8734afae`,
`845ebce2-c778-45a7-b8ba-5b29729b4d17`, `06b80791-e4f2-425f-b19e-c29759e01424`,
`c6dad243-0329-450b-bf07-e875b342f476`, `24dbc918-6d43-4f01-bd20-addcf94f139d`,
`e001cd4c-05ef-49f7-8fc4-42c5d7457263`, `8d934f28-d306-4a2e-9bac-e5eb34f52475`, and
disposal-only IDs `276c8ae7-ba76-4017-b233-a29763cd5823` and
`de3a0a29-55aa-4b98-b2e8-55cc705d76eb`. Its retained storage log/receipt IDs are
`101c7ec5-5ea4-4787-b23a-582e182f09ce` / `a5f05e0c-88d4-4afd-8abf-fc423b959c34`.

Earlier C attachment `3899a715-92c5-4740-9d6b-2541f03c567f` belongs to failed acknowledgement
recovery `20261016T014438.376Z-239543` and disposal 82425; D attachment
`cf96c19b-d0e7-4346-9b49-e77c6483b372` belongs to grouped failure
`20261016T020656.965Z-d1060a` and disposal 77145. A/B installation identities
`77a4bd62-3f8b-4cd6-b34e-06f87aca0cdb` / `2274fec2-5124-4292-99c7-9546a6ffdf7b` and C
`16d9f9a9-cf58-45de-be54-bf67689c7994` are not attachment IDs. Their authenticated disposal
receipts remain historical; do not recreate those campaigns or reopen discarded target work.

Four predecessor default attachments have durable `installation-discard.json` acknowledgements:
`01aabe56-6239-4ce2-8579-be3e1a88ddd4`, `76b96326-dad4-4a07-9400-bc6639a66db2`,
`931e1988-60d3-4346-ad16-6efe7d9889a0` and `be1dcfff-2b76-4bfd-8675-a07cc2ee5bef`.
They bind predecessor generation `cfe8cd1ee735f4c65eae9f03b1d04bb390a561cb058933bbd655f9eb2b55ac30`
and the CE80 journal above. Whole-target discard settles those ownership obligations without
rewriting original failed/Unknown outcomes. CE80's retained log/receipt objects are
`173cac4f-3b05-433c-aa54-74365ad75601` / `952b90b6-c924-4ee9-a865-c997ea2770c6`.

### Evidence retention and historical document consumer

Keep the managed captures under existing `citation:catalog-efficiency-current-acceptance` holds
and the runtime review's `citation:catalog-runtime-efficiency-review` holds. The runtime slow
inventory covers 34 of 149 existing full managed run IDs cited by the previous coordinator;
115 cited runs, including construction/disposal, pure acceptance and hygiene, lie outside it.
It cannot replace their receipt context. No storage consumer was released by plan authorship.

The [superseded coordinator's dated records](catalog-efficiency-plan_2026-10-15.md#6-sole-disposition-and-current-checkpoint)
and companions retain historical consumers, including immutable ADR-0174/0175/0177 citations
and their receipt context. They are reference-only, with no live execution ownership. This is the reason for
retaining full content rather than leaving stubs or editing accepted ADR bodies. Current obligations
are available here without recovering old instructions. Remove historical documents only when
their actual remaining consumers end; Git provides recovery.

Original review runs `20261015T181356.562Z-8a3347` and `20261015T181515.206Z-7c15e3` retain
their independent `citation:catalog-completion-efficiency` consumer and paused older-track scope.
Their original Unknowns are not imported as new execution work by this supersession.

## 3. RP00 — Snapshot-local exact descriptor acquisition

Selected backup consumes the exact transport envelope and protected publication in one snapshot.
Use a charged map of exact view descriptors across the entire traversal, including dependencies
shared by multiple contributors/views. Preserve every root-qualified copy label and logical/physical
correspondence. A dependency previously observed within this snapshot can be reused without
another exact query. Batch independent metadata demands only where the pinned executor actually
supports the required read semantics; an envelope around scalar calls is not the benefit itself.

Keep complete closure, full unequal-copy refusal, missing-dependency failure and exact selected
inventory. No map is authoritative across transactions, clients, contexts/epochs or independent
receivers. Bound admission and retain map charges through its last snapshot consumer, then release
them; do not create a publication-wide descriptor cache.

Migrate `selected_backup.rs` root/descriptor traversal and all its in-snapshot dependency reads.
Source inspection plus a diamond/shared-dependency fixture establishes once-per-snapshot acquisition
and retention of all physical copies. Unequal descriptors, cycles handled by the existing traversal,
missing dependencies and partial read/finality failure remain strict. Actual backup acceptance must
reach this path; the historical compile-frontier timeout supplies no backup diagnosis.

## 4. RP01 — Finalize source before carrier construction

`backup_owned` retains its external transaction through local metadata/carrier hashing/copying.
Reorder the existing owners: export the complete selected content to owned spools under the exact
snapshot, reach checked application/physical EOF for every source stream, then cancel and close
the exact transaction/session before independent carrier work. Preserve source pins through their
last required use; release them through their actual owner rather than assume transaction close
settles every hold.

Only successful source finalization permits carrier construction, receipt validation and final
output publication. Failed cancellation/close retains the exact Unknown owner and leaves staged
output unpublished. A local carrier failure after successful native close does not reopen native
uncertainty. Serialization required to consume source rows stays inside export; the removed
lifetime is work that needs only owned local spools.

Migrate success, cancellation and error paths in backup/export and their carrier call sites.
Maintain structured primary/related/finalizer observations; do not discard an acknowledged effect
when later cleanup fails. RP00 and RP01 share publisher files but neither requires RN01's protocol
replacement. Verify source-finalization ordering using explicit local gates/failures, then actual
selected backup/independent restore with exact originals, aliases, ownership and cold semantics.

## 5. RE80 — Conditional final target-state regeneration

**Delivered RE80 default replacement, passed 2026-10-16:** managed run
`20261016T232319.399Z-739293` completed exact default installation
`58abe9ea-2088-4109-95ab-8440a3749c51` regeneration of `library_context/{main,validation}` through
the existing service/run/storage owners. Its generation is
`1b7ae070a26755c60d87888e1b2291b75b120d6a89941bae7c87dc001c79db4c`, regeneration journal
`21343c88a458c8df763770a0c998950921522037461e2e5aba8b99ccaccc7e14`, with unchanged required peer
`9fbab612b96f62d2e88070ffd2cb7f1d8ec7b7a3a5cf304592c1c041ed3c761e`. Authenticated host/native
executor drainage and exact daemon shutdown preceded discard/reinitialization; both scopes
passed their checks and admission reopened. Host cleanup was confirmed. All 23 previously
protected default attachments received separate validated discard acknowledgements. Those settle
ownership obligations without rewriting their original failed/Unknown outcomes. The one owned
candidate's controls, regeneration and final disposal remain separately recorded by the coordinator.
This establishes target replacement, not RE90 functional acceptance or operator adoption.

Prior CE80 is delivered. Determine from selected new code/schema/definition/identity changes whether
fresh project reconstruction is required; do not redo an old campaign automatically. Fresh logical
attempts may suffice where physical schema/definition authority is unchanged. If replacement is
required, use existing installation/service/run/storage ownership and exact required-peer route.

First establish authenticated worker/reader/native drainage and settle or explicitly retain exact
obligations. Unknown drainage blocks destructive replacement. Regenerate project stores/projections
from pinned inputs, migrate all consumers and remove obsolete generations, compatibility-only
readers and runtime copies without a current consumer. No old-format preservation or historical
fixture/benchmark prerequisite is required. Preserve warm caches, cited receipts and original
outcomes; broad purge, shared reset and operator adoption remain unauthorized.

Run revealing pre-cutover controls, perform any required state replacement, and only then RE90's
final acceptance. A later target-state mutation invalidates the affected acceptance boundary.

The later stale-binary acceptance run was stopped after material source repairs. Approved exact
default replacement `20261017T012711.701Z-53365f` **passed** on 2026-10-17, with authenticated
shutdown/discard and host cleanup confirmed. Its generation was
`c3ff2d7df87dadc02167479b95fd5fcbd4ac6e9590a2a3f619fd87e77251aa4b`; journal
`f49200d7cfcfd9194f4ef583c3416b82938c4bf1c96df6e21a906f684873e2d7` retains the operation and
separate discard acknowledgements for `2faec210-e023-4b36-b2e0-b76f1f2911d4` and
`4f3b2fcf-9059-42a1-89c5-ef23484bce10`. Original failure/Unknown observations remain unchanged.
Main/validation checks and admission reopening passed. A subsequent approved exact default replacement `20261017T020954.067Z-ed921b` **passed** after
all workers drained. Its generation was
`ea82ade5fe60bd3078a6e8ac46439a12cda496d260f73feac94fdc0e5f9ae2e4`, journal
`f48b8e325a5b677dcfec6f90618555262f6d7a05406af4e5d0c3ba6d360f7c7d`, same exact installation
and pinned peer. Separate discard acknowledgements settle the later CallScope timeout/probe/byte-fixture
attachments `f339481b-bb73-44a9-adc4-922c69b01b3c`, `de405073-4543-4c72-8fc6-bb12bf36c789`
and `25cd9e0a-d4c5-4285-a4bb-d90bfb8b7277`; original failed/Unknown outcomes remain unchanged.
Main/validation check/open and host cleanup passed. Native refresh `20261017T021104.009Z-d38095`
passed after formatting. RE90's outstanding selected-complete rerun is `20261017T021707.714Z-3e0358`;
predecessor passes remain dated at their own scopes. No target mutation may follow accepted final-state
evidence without reopening the affected boundary.

Rerun `3e0358` was subsequently stopped after native concurrency failures selected another bounded
source correction. Exact authorized replacement `20261017T023906.406Z-682a66` **passed** after
host/executor drainage, with main/validation check/open and host cleanup confirmed. Current generation
is `ce1ca3917631274968a68eb7b028b1130463ac83f50c95abbbdd991985d66ff3`, journal
`fb4fa3e1f6e88ff92b9b9479cb8c87b04715dea2f7ae2ed6c3fb25e84f03e10d`, same installation/peer.
Its exact separate discard acknowledgement settles attachment
`12604101-fef7-4585-8858-a40960af4897`; original failures/cancellation and native observations
remain unchanged. Subsequent acceptance must bind this successor, with no credit for workload
success inferred from replacement. No new campaign or operator adoption occurred.


## 6. RE90 — Integrated functional acceptance

First review the integrated implementation and accumulated findings against the target and its
growth/failure scenarios. Assess interactions of demand acquisition, independent tasks, protected
premises, native cuts and early source release. Local passes do not establish their composition.

Carry the remaining functional boundaries forward explicitly:

| Boundary | Required evidence |
|---|---|
| Model | Full current selected model boundary, including integration/doc/compile-fail; 335 library passes alone is insufficient |
| Providers | Enclosing Resource-negative/finality path after the last operation/finalizer repair; ordinary/native provider and flow-oracle controls |
| Compiler | Corrected actual large-binding/owner controls, producer/CLI, private composition/rebase/cancel, old-port stability, independent reference admission |
| Native | Repeated/equal/conflicting windows, complete cuts, delayed issuance/reconciliation, retirement/backup races, admitted wide/parallel ingress and actual session/EOF finality |
| Analysis | Paired facts layout, Base channels, SourceCall equivalence and ordinary/recursive Summary reach actual assertions; finite analytic/partial-result distinctions remain |
| Publication | Real selected backup/export/independent cold restore reaches the consumer; complete originals/copies/evidence and fresh receiving ownership |
| Lexical/serving | Inherited Rust serving/publication controls, actual selected-corpus scoring/access, corruption/cold reconstruction; BM25's historical single pass is narrower |
| Development | Selected tooling cancellation/discovery/source-observation/channel-loss controls; keep complete safety inventory distinct from bounded discovery |

Resolve historical timeouts from actual reached phases and retained obligations. Do not assume the
last event or test name identifies the initiating cause. The slow inventory routes all captured
keys without requiring a separate diagnosis or rerun for every historical occurrence. Rebuild
affected current binaries and rerun changed/failed/not-run boundaries; keep original evidence.

Use the existing verification selections and selected-complete coverage contract to preserve the
inherited functional scope. At completion, perform applicable shared model/receipt/trust/transport
acceptance, full compile check, strict keep-going Clippy including warnings and applicable Python
Ruff/Pyrefly/non-functional leaves once; repair owned failures and rerun affected scope. Do not
launch assembled campaigns after each package. Preserve explicit embedding and submitted MCP
runtime exclusions, report selected versus global coverage and leave wider qualification incomplete
where it would require excluded work. Full real-library/operator acceptance is separate.

First-principles reasoning and minimal revealing cases establish design value. Timeouts may bound
observation but do not define a 60-second SLA or justify weaker assertions/caps. No fixed comparative
benchmark, historical volume or machine-specific provenance campaign is required. Measurements
are needed for quantitative claims or a concrete uncertainty capable of reversing a choice.

## 7. RE99 — Documentation handback and older-track boundary

After RE90, update current architectural owners, ADR lifecycle metadata/index, runbooks, STATUS
and this coordinator's dispositions to the actual implementation/evidence strength. Record remaining
blocked/not_run items with their reason and exact owner, without claiming whole-track completion
while mandatory exits remain. Carry unresolved evidence and storage consumers forward explicitly.

Identify which paused Catalog-completion/SA9/persisted-efficiency assumptions and exits need later
operator-authorized reconciliation. Do not execute or close those tracks as part of handback.

The later handback must account for these changed assumptions in their existing owners:

- **Catalog completion:** original 900-second reconstruction timeouts do not qualify the new
  demand/joined/capture/Summary route. Reconcile its remaining acceptance against RE90's actual
  reached assertions and exclusions, rather than schedule duplicate historical runs automatically.
- **SA9/native lifecycle:** ordinary bounded effects are atomic under ADR-0178; split-intent
  expectations must move to original-request reconciliation and complete cuts. Distinguish
  independently acknowledged target discard from successful logical cleanup and retain actual
  receiving ownership, source finality and adoption boundaries.
- **Persisted efficiency:** shared Base and descriptor preparation, changed incidence and protected
  preserving reads are implemented contracts. Reconcile old eager acquisition, singleton-only
  preparation, per-window assurance and prolonged backup-source assumptions against ADR-0178–0180
  and current owners. Historical memory attribution and quantitative speedup remain unestablished.

Remove superseded runtime paths and no-longer-consumed documents according to actual ownership;
retain immutable historical citations while they have a consumer. RE99 changes documentation only;
no final regeneration or product mutation follows accepted target-state evidence.
