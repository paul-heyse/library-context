# Source5 committed replay and fixed journal pointer

Prepared 2026-10-11, OFF-TREE only. Depends on the complete migration foundation in
`/tmp/lctx-migration6-foundation`, including its private retained-source5 outcome helper.
This patch is independently apply-checkable against main, but cannot compile before that
foundation is integrated. No Cargo, service, plan/STATUS or main writes were performed.

`patches/replay-pointer.patch` changes only control/authorization.rs and control/schema.surql.
The committed public replay match calls a narrow async owner validator after the exact
payload digest and native receipt ID/generation/era/request digest are authenticated.
Ordinary format6 owners retain their exact existing validation. Only committed+resolved
installation-owner/attempt NONE receipt shape reaches the foundation's fixed-pointer
source5 proof; shape alone still refuses. The proof is outcome-only and cannot reach
intent or payload execution. Pending/new requests and every other owner retain their
existing refusal/fences. The error retains NativeRequest identity.

The target base control schema now declares the optional fixed native_installation
source5_upgrade record<native_upgrade_v6> pointer. It changes the target base schema
identity and therefore root-owned target binding descriptors/snapshot expectations.
A fresh target installation does not need a journal pointer; only the aggregate adjacent
activation can create a published source5 pointer. The foundation supplies retained journal
and terminal-frozen evidence declarations. Root owns their storage retention/backup closure.

Pure candidate-shape controls were added but NOT_RUN. They show that normal owner validation
still refuses NONE, and non-installation, pending, unresolved, non-NONE owner or attempt
shapes never qualify for this specific proof. Actual retained-journal native controls remain
required once the aggregate is runnable. Do not interpret source controls as migration tests.
