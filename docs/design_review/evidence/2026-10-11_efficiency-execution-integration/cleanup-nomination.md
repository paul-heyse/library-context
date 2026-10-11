# N09 terminal hold-page nomination

Prepared 2026-10-11, OFF-TREE source packet only. Main checkout untouched; no Cargo,
service/fixture actions, quality tools, formatting, commits or plan/STATUS edits.

## Behavior and ownership

`patches/n09-cleanup-nomination.patch` changes cleanup.rs and retirement.rs only.
CleanupHolds::step prepares the ACTUAL ordered native_hold window for its first persisted
owner. It reads fixed actual native_guard rows for every target and refuses missing,
nonpositive, retired/retiring, duplicate or foreign physical evidence; it never creates
an incarnation1 fallback. The original held roots may have no product or no completed
content: root nomination does not require reuse eligibility or a content-row scan.

The same existing EffectOwner::Cleanup native payload rechecks the exact cleanup row,
full original terminal attempt row/generation/epoch, successor original era<=current cleanup
era, persisted owner scope and actual current owner incarnation/phase. It reselects and
compares the COMPLETE prepared hold page. Then, in that ONE transaction, it creates a
fresh native_retirement job and EVERY exact-incarnation native_retirement_item before
any compared hold is deleted. Job root_digest hashes the actual complete canonical root
set with the existing native-retirement/v4 formula; root_count=roots_registered=actual
unique page roots, state=open, examined=retired=0. No partial registering job is exposed.
Each job is globally discoverable through existing live_retirements state/id index and
uses existing explicit resume_retirement/recover_retirement routes. There is no automatic
retirement advancement or background collector added. Original effect retains unknown
acknowledgement/reconciliation ownership; unresolved cleanup effects still block next step.

An empty page creates no job and advances exactly its persisted owner. Actual origin checks
apply equally to admitted and unadmitted roots and to current-era maintenance Cleanup
successors; there is no TerminalAttempt effect in the new page implementation. Existing
backup gate is not acquired for harmless registration; actual destructive retirement later
uses native_backup_clear and existing hold/phase/incarnation guards. Peer/warm/view/publication/
reader holds therefore remain retirement blockers. No capacities, deadlines or concurrency
settings change; rows and complete SQL/bindings use existing native window/transfer preflight.

The existing unadmitted product-removal branch is preserved. This packet closes only the
attempt/cleanup outgoing-incidence nomination gap. Product retention/release, terminal reuse,
recovery-owner expiry/release and binding compaction remain separate N09 obligations, along
with source5 aggregate migration and actual native qualification. It does not authorize an
operator cleanup/sweep or claim assembled N09 closure.

## Controls

Pure controls added (ALL NOT_RUN):
- retirement::cleanup_nomination_controls authentic complete canonical root digest/count,
  exact item incarnation IDs, unchanged actual hold envelopes, duplicate/foreign hold refusal,
  missing/zero/nonactive guard refusal, and empty-page/no-job distinction;
- cleanup::cleanup_origin_controls current-era successor with older original terminal era
  succeeds with exact epoch/generation; changed epoch/generation/future era/nonterminal state
  or unadmitted/admitted mismatch refuses; admitted cleanup remains supported.

Required revealing actual native controls for root integration (conceptual, NOT_RUN):
1. Failed/no-product and incomplete-output attempts with more than one native window of
   holds. Stop after one CleanupHolds page: exactly that page's full job/items exist, all
   its holds are gone, remaining holds stay; explicit resume discovers/retires jobs later.
2. Admitted attempt has shared completed/publication/reader/warm-held targets: nomination
   and attempt hold release succeed; explicit retirement retains those targets/peers.
3. Prepare a page, then replace a hold at the same count or change one root incarnation:
   cleanup snapshot/holds/guards and all job/items remain unchanged after refusal. Do not
   refresh prepared observations inside a retry of the same native request.
4. Lose native acknowledgement after payload commit: reconcile the original NativeRequest;
   exact job/items and hold deletion exist once, next cleanup step cannot delete unseen work.
   Lose acknowledgement before commit: no job/items or hold deletion; unresolved owner blocks
   advancement until explicit actual reconciliation.
5. Cut era only after lawful owner closure; recover a remaining cleanup under its distinct
   current-era Cleanup successor and original terminal attempt epoch/generation. Nomination
   works without granting original TerminalAttempt a new era; stale predecessor pages refuse.
6. Active backup hold: nomination/release page succeeds without destructive advancement;
   explicit retirement refuses at existing backup gate until actual backup release.
7. Empty-owner and empty-hold pages create no empty retirement jobs; exact owner progression
   rejects concurrent scope/terminal-origin changes and changed owner incarnation.

No mocked provider result or source inspection is a native-control pass. Root must compile
and run focused affected controls after accepting the packet and acquiring execution ownership.
