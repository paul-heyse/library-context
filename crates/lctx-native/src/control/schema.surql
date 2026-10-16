DEFINE TABLE native_installation SCHEMAFULL;
DEFINE FIELD generation ON native_installation TYPE string;
DEFINE FIELD schema ON native_installation TYPE string;
DEFINE FIELD schema_version ON native_installation TYPE int;
DEFINE FIELD admission_open ON native_installation TYPE bool;
DEFINE FIELD control_revision ON native_installation TYPE int DEFAULT 0;
DEFINE FIELD era ON native_installation TYPE int DEFAULT 0;
DEFINE FIELD closed_through ON native_installation TYPE int DEFAULT 0;
DEFINE FIELD history_inventory ON native_installation TYPE option<string>;
DEFINE TABLE native_attempt SCHEMAFULL;
DEFINE FIELD id ON native_attempt TYPE string;
DEFINE FIELD generation ON native_attempt TYPE string;
DEFINE FIELD era ON native_attempt TYPE int;
DEFINE FIELD state ON native_attempt TYPE string;
DEFINE FIELD revision ON native_attempt TYPE int;
DEFINE FIELD admitted ON native_attempt TYPE bool;
DEFINE FIELD admitted_view ON native_attempt TYPE option<string>;
DEFINE FIELD epoch ON native_attempt TYPE int;
DEFINE INDEX live_attempts ON native_attempt FIELDS state,id;
DEFINE TABLE native_attempt_stripe SCHEMAFULL;
DEFINE FIELD format ON native_attempt_stripe TYPE int;
DEFINE FIELD attempt ON native_attempt_stripe TYPE record<native_attempt>;
DEFINE FIELD generation ON native_attempt_stripe TYPE string;
DEFINE FIELD era ON native_attempt_stripe TYPE int;
DEFINE FIELD epoch ON native_attempt_stripe TYPE int;
DEFINE FIELD slot ON native_attempt_stripe TYPE int;
DEFINE FIELD revision ON native_attempt_stripe TYPE int;
DEFINE INDEX attempt_stripes ON native_attempt_stripe FIELDS attempt,slot UNIQUE;
DEFINE TABLE native_effect SCHEMAFULL;
DEFINE FIELD id ON native_effect TYPE string;
DEFINE FIELD attempt ON native_effect TYPE option<record<native_attempt>>;
DEFINE FIELD owner ON native_effect TYPE option<record>;
DEFINE FIELD owner_kind ON native_effect TYPE string;
DEFINE FIELD generation ON native_effect TYPE string;
DEFINE FIELD era ON native_effect TYPE int;
DEFINE FIELD request ON native_effect TYPE string;
DEFINE FIELD committed ON native_effect TYPE bool;
DEFINE FIELD resolved ON native_effect TYPE bool;
DEFINE FIELD revision ON native_effect TYPE int;
DEFINE FIELD epoch ON native_effect TYPE int;
DEFINE FIELD attempt_slot ON native_effect TYPE option<int>;
DEFINE FIELD original_completion_outcome ON native_effect TYPE option<string> ASSERT $value=NONE OR $value IN ['applied','premise_changed'];
DEFINE INDEX live_effects ON native_effect FIELDS resolved,id;
DEFINE INDEX attempt_effects ON native_effect FIELDS attempt,id;
DEFINE INDEX owner_effects ON native_effect FIELDS owner,resolved,id;
DEFINE TABLE native_guard SCHEMAFULL;
DEFINE FIELD revision ON native_guard TYPE int;
DEFINE FIELD retired ON native_guard TYPE bool;
DEFINE FIELD phase ON native_guard TYPE string DEFAULT 'active';
DEFINE FIELD incarnation ON native_guard TYPE int DEFAULT 1;
DEFINE FIELD retiring_item ON native_guard TYPE option<record<native_retirement_item>>;
DEFINE TABLE native_hold SCHEMAFULL;
DEFINE FIELD owner ON native_hold TYPE record;
DEFINE FIELD object ON native_hold TYPE record;
DEFINE INDEX object_holds ON native_hold FIELDS object,owner UNIQUE;
DEFINE INDEX owner_holds ON native_hold FIELDS owner,object;
DEFINE TABLE native_pin SCHEMAFULL;
DEFINE FIELD id ON native_pin TYPE string;
DEFINE FIELD views ON native_pin TYPE array<record<compiler_view>>;
DEFINE FIELD released ON native_pin TYPE bool;
DEFINE FIELD epoch ON native_pin TYPE int;
DEFINE FIELD generation ON native_pin TYPE string;
DEFINE FIELD era ON native_pin TYPE int;
DEFINE INDEX live_pins ON native_pin FIELDS released,id;
DEFINE TABLE native_product SCHEMAFULL;
DEFINE FIELD request ON native_product TYPE string;
DEFINE FIELD contribution ON native_product TYPE record<compiler_contribution>;
DEFINE FIELD released ON native_product TYPE bool DEFAULT false;
DEFINE FIELD release_job ON native_product TYPE option<record<native_retirement>>;
DEFINE INDEX request_product ON native_product FIELDS request;
DEFINE INDEX contribution_products ON native_product FIELDS contribution;
DEFINE INDEX product_release_job ON native_product FIELDS release_job;
DEFINE TABLE native_retirement SCHEMAFULL;
DEFINE FIELD id ON native_retirement TYPE string;
DEFINE FIELD examined ON native_retirement TYPE int;
DEFINE FIELD retired ON native_retirement TYPE int;
DEFINE FIELD revision ON native_retirement TYPE int;
DEFINE FIELD generation ON native_retirement TYPE string;
DEFINE FIELD era ON native_retirement TYPE int;
DEFINE FIELD epoch ON native_retirement TYPE int;
DEFINE FIELD root_digest ON native_retirement TYPE string;
DEFINE FIELD state ON native_retirement TYPE string;
DEFINE FIELD successor ON native_retirement TYPE option<record<native_retirement>>;
DEFINE FIELD lineage ON native_retirement TYPE option<record<native_retirement>>;
DEFINE FIELD root_count ON native_retirement TYPE int DEFAULT 0;
DEFINE FIELD roots_registered ON native_retirement TYPE int DEFAULT 0;
DEFINE FIELD predecessor ON native_retirement TYPE option<record<native_retirement>>;
DEFINE FIELD recovery_source ON native_retirement TYPE option<record<native_retirement>>;
DEFINE INDEX live_retirements ON native_retirement FIELDS state,id;
DEFINE INDEX lineage_retirements ON native_retirement FIELDS lineage,state,id;
DEFINE TABLE native_retirement_item SCHEMAFULL;
DEFINE FIELD id ON native_retirement_item TYPE string;
DEFINE FIELD job ON native_retirement_item TYPE record<native_retirement>;
DEFINE FIELD object ON native_retirement_item TYPE record;
DEFINE FIELD state ON native_retirement_item TYPE string;
DEFINE FIELD incarnation ON native_retirement_item TYPE int;
DEFINE FIELD cutoff ON native_retirement_item TYPE option<int>;
DEFINE FIELD successor ON native_retirement_item TYPE option<record<native_retirement_item>>;
DEFINE INDEX retirement_queue_v4 ON native_retirement_item FIELDS job,state,id;
DEFINE INDEX object_retirements ON native_retirement_item FIELDS object,state,id;
DEFINE TABLE native_backup_hold SCHEMAFULL;
DEFINE FIELD id ON native_backup_hold TYPE string;
DEFINE FIELD active ON native_backup_hold TYPE bool;
DEFINE FIELD generation ON native_backup_hold TYPE string;
DEFINE FIELD era ON native_backup_hold TYPE int;
DEFINE FIELD epoch ON native_backup_hold TYPE int;
DEFINE INDEX live_backups ON native_backup_hold FIELDS active,id;
DEFINE TABLE native_outcome_ref SCHEMAFULL;
DEFINE FIELD effect ON native_outcome_ref TYPE option<record<native_effect>>;
DEFINE FIELD object ON native_outcome_ref TYPE option<record>;
DEFINE FIELD owner ON native_outcome_ref TYPE string;
DEFINE FIELD kind ON native_outcome_ref TYPE string;
DEFINE INDEX effect_outcome_refs ON native_outcome_ref FIELDS effect,owner,kind;
DEFINE INDEX object_outcome_refs ON native_outcome_ref FIELDS object,owner,kind;
DEFINE TABLE native_cleanup SCHEMAFULL;
DEFINE FIELD id ON native_cleanup TYPE string;
DEFINE FIELD attempt ON native_cleanup TYPE record<native_attempt>;
DEFINE FIELD generation ON native_cleanup TYPE string;
DEFINE FIELD era ON native_cleanup TYPE int;
DEFINE FIELD epoch ON native_cleanup TYPE int;
DEFINE FIELD owners ON native_cleanup TYPE array<record>;
DEFINE FIELD unadmitted ON native_cleanup TYPE bool;
DEFINE FIELD state ON native_cleanup TYPE string;
DEFINE FIELD successor ON native_cleanup TYPE option<record<native_cleanup>>;
DEFINE INDEX live_cleanup ON native_cleanup FIELDS state,id;
DEFINE INDEX attempt_cleanup ON native_cleanup FIELDS attempt,state,id;
DEFINE TABLE native_history_checkpoint SCHEMAFULL;
DEFINE FIELD generation ON native_history_checkpoint TYPE string;
DEFINE FIELD era ON native_history_checkpoint TYPE int;
DEFINE FIELD epoch ON native_history_checkpoint TYPE int;
DEFINE FIELD closed_through ON native_history_checkpoint TYPE int;
DEFINE FIELD revision ON native_history_checkpoint TYPE int;
DEFINE FIELD removed_effects ON native_history_checkpoint TYPE int;
DEFINE FIELD removed_records ON native_history_checkpoint TYPE int DEFAULT 0;
DEFINE FIELD phase ON native_history_checkpoint TYPE string DEFAULT 'effect';
DEFINE FIELD state ON native_history_checkpoint TYPE string;
DEFINE FIELD after ON native_history_checkpoint TYPE option<record>;
DEFINE FIELD last_page ON native_history_checkpoint TYPE option<object> FLEXIBLE;

DEFINE TABLE native_admission_stripe SCHEMAFULL;
DEFINE FIELD format ON native_admission_stripe TYPE int;
DEFINE FIELD generation ON native_admission_stripe TYPE string;
DEFINE FIELD era ON native_admission_stripe TYPE int;
DEFINE FIELD closed_through ON native_admission_stripe TYPE int;
DEFINE FIELD admission_open ON native_admission_stripe TYPE bool;
DEFINE FIELD revision ON native_admission_stripe TYPE int;
DEFINE TABLE native_backup_gate SCHEMAFULL;
DEFINE FIELD revision ON native_backup_gate TYPE int;
DEFINE FIELD active_count ON native_backup_gate TYPE int;
DEFINE TABLE native_operation_owner SCHEMAFULL;
DEFINE FIELD generation ON native_operation_owner TYPE string;
DEFINE FIELD era ON native_operation_owner TYPE int;
DEFINE FIELD epoch ON native_operation_owner TYPE int;
DEFINE FIELD operation ON native_operation_owner TYPE string;
DEFINE FIELD state ON native_operation_owner TYPE string;
DEFINE TABLE native_predicate SCHEMAFULL;
DEFINE FIELD revision ON native_predicate TYPE int;
DEFINE FIELD tombstone_revision ON native_guard TYPE int DEFAULT 0;
DEFINE FIELD predecessor ON native_cleanup TYPE option<record<native_cleanup>>;
DEFINE FIELD lineage ON native_cleanup TYPE option<record<native_cleanup>>;
DEFINE INDEX retirement_predecessor ON native_retirement FIELDS predecessor;
DEFINE INDEX retirement_successor ON native_retirement FIELDS successor;
DEFINE INDEX retirement_recovery_source ON native_retirement FIELDS recovery_source;
DEFINE INDEX cleanup_predecessor ON native_cleanup FIELDS predecessor;
DEFINE INDEX cleanup_successor ON native_cleanup FIELDS successor;
// Exact native IDs are array keys: no string encoding or content-hash correspondence.
DEFINE FUNCTION fn::native_touch($record: option<record>) {
    IF $record != NONE { IF record::table($record)='native_attempt' { LET $owner=SELECT * FROM ONLY $record; IF $owner=NONE { THROW 'native attempt incidence owner missing'; }; fn::native_attempt_cut($record,$owner.generation,$owner.era,$owner.epoch,'incidence'); }; LET $guard=type::record('native_predicate',[$record]); IF (SELECT VALUE revision FROM ONLY $guard)>=9223372036854775807 { THROW 'native predicate revision exhausted'; }; UPSERT $guard SET revision=(revision ?? 0)+1 RETURN NONE; };
    RETURN NONE;
};
// Every incidence writer/remover and collector calls this on the same exact target.
DEFINE FUNCTION fn::native_incidence_targets($row: object) {
    RETURN array::distinct(array::flatten([
        [$row.id],
        IF record::table($row.id) IN ['compiler_contribution','compiler_binding','native_effect','native_cleanup','native_pending_original','native_publication_staging'] { [$row.attempt] } ELSE { [] },
        IF record::table($row.id) IN ['native_effect','native_hold'] { [$row.owner] } ELSE { [] },
        IF record::table($row.id) IN ['native_hold','native_outcome_ref'] { [$row.object] } ELSE { [] },
        IF record::table($row.id)='native_outcome_ref' { [$row.effect] } ELSE { [] },
        IF record::table($row.id)='compiler_membership' { [$row.contribution] } ELSE { [] },
        IF record::table($row.id) IN ['native_completed_view','native_completed_contributor'] { [$row.completion] } ELSE { [] },
        IF record::table($row.id)='native_retirement_item' { [$row.job] } ELSE { [] },
        IF record::table($row.id)='native_product' { [$row.contribution,$row.release_job] } ELSE { [] },
        IF record::table($row.id) IN ['native_retirement','native_cleanup'] { [$row.lineage,$row.predecessor,$row.successor,$row.recovery_source] } ELSE { [] }
    ])).filter(|$record| $record!=NONE);
};
DEFINE FUNCTION fn::native_incidence($row: object) {
    FOR $record IN fn::native_incidence_targets($row) { fn::native_touch($record); };
    RETURN NONE;
};

DEFINE FUNCTION fn::native_backup_clear() {
    LET $gate=SELECT * FROM ONLY native_backup_gate:current FOR UPDATE;
    IF $gate=NONE OR type::is_int($gate.active_count)=false OR $gate.active_count!=0 OR type::is_int($gate.revision)=false OR $gate.revision<0 { THROW 'native retirement backup hold'; };
    RETURN NONE;
};

DEFINE FIELD owner_incarnations ON native_cleanup TYPE array<object> FLEXIBLE DEFAULT [];

DEFINE FUNCTION fn::native_incidence_insert($row: object) {
    fn::native_incidence($row);
    IF record::table($row.id) IN ['compiler_contribution','compiler_binding','native_cleanup','native_pending_original','native_publication_staging'] { IF $row.attempt=NONE OR (SELECT VALUE id FROM ONLY $row.attempt)=NONE { THROW 'native incidence attempt missing'; }; };
    IF record::table($row.id)='compiler_membership' { fn::native_contribution_incidence($row.contribution,$row); };
    IF record::table($row.id) IN ['native_completed_view','native_completed_contributor'] { fn::native_completed_incidence($row.completion,$row); };
    IF record::table($row.id)='native_hold' AND record::table($row.owner)='native_completed_state' { fn::native_completed_incidence($row.owner,$row); };
    IF record::table($row.id)='native_retirement_item' { IF (SELECT VALUE id FROM ONLY $row.job)=NONE { THROW 'native incidence parent missing'; }; };
    RETURN NONE;
};

DEFINE FUNCTION fn::native_contribution_incidence($contribution: record<compiler_contribution>, $row: object) {
    fn::native_touch($contribution);
    LET $parent=SELECT * FROM ONLY $contribution FOR UPDATE;
    IF $parent=NONE OR type::is_bool($parent.completed)=false { THROW 'native contribution incidence parent missing'; };
    IF $parent.completed AND (SELECT * FROM ONLY $row.id)!=$row { THROW 'native completed contribution refuses changed incidence'; };
    RETURN NONE;
};

// Compound boundary child inventory cannot change after finality.
DEFINE FUNCTION fn::native_completed_incidence($completion: record<native_completed_state>, $row: object) {
    fn::native_touch($completion);
    LET $parent=SELECT * FROM ONLY $completion FOR UPDATE;
    IF $parent=NONE OR type::is_bool($parent.completed)=false { THROW 'native completed boundary parent missing'; };
    IF $parent.completed AND (SELECT * FROM ONLY $row.id)!=$row { THROW 'native completed boundary refuses changed inventory'; };
    RETURN NONE;
};

// One bounded immutable effect prepares exact existing values and distinct parent
// predicates once. Holds share incidence admission but retain their owner/target
// existence checks after new rows are installed in the same transaction.
DEFINE FUNCTION fn::native_immutable_install($rows: array<object>, $holds: array<object>, $context: option<object>, $deferred: array<object>, $originals: array<object>) {
    LET $all=array::concat($rows,$holds);
    LET $blobs=array::distinct($rows.filter(|$row| record::table($row.id)='original_chunk').map(|$row| $row.blob));
    LET $prepared=$all.map(|$row| { row: $row, existing: (SELECT * FROM ONLY $row.id) });
    LET $original_prepared=$originals.map(|$row| { row: $row, existing: (SELECT * FROM ONLY $row.id) });
    FOR $entry IN $original_prepared { LET $prior=$entry.existing; LET $row=$entry.row; IF $prior!=NONE AND ($prior.id!=$row.id OR $prior.content!=$row.content OR $prior.byte_len!=$row.byte_len OR $prior.format!=$row.format OR type::is_bool($prior.completed)=false OR array::len(object::keys($prior))!=5) { THROW 'original blob declaration collision'; }; };
    // Equal rows and already-owned equal holds do not change incidence. A new
    // owner's hold still changes both endpoints and excludes retirement.
    LET $changed=array::concat($prepared,$original_prepared).filter(|$entry| $entry.existing=NONE).map(|$entry| $entry.row);
    LET $changed_blobs=array::distinct($changed.filter(|$row| record::table($row.id)='original_chunk').map(|$row| $row.blob));
    LET $targets=array::distinct(array::concat(array::flatten($changed.map(|$row| fn::native_incidence_targets($row))),$changed_blobs));
    FOR $target IN $targets {
        fn::native_touch_admitted($target,$context);
        // First creation already advanced its physical incarnation above. New
        // incidence on an existing immutable endpoint advances the guard once;
        // existing equal holds/rows are absent from this changed target set.
        IF array::len($changed.filter(|$row| $row.id=$target))=0 AND ($context=NONE OR $target!=$context.attempt) {
            LET $entries=$deferred.filter(|$entry| $entry.record=$target);
            IF array::len($entries)>0 { LET $guard_id=$entries[0].guard; LET $guard=SELECT * FROM ONLY $guard_id FOR UPDATE; IF $guard=NONE OR $guard.phase!='active' OR $guard.revision>=9223372036854775807 { THROW 'native changed incidence guard invalid'; }; UPDATE $guard_id SET revision+=1 RETURN NONE; };
        };
    };
    LET $attempts=array::distinct($rows.filter(|$row| record::table($row.id) IN ['compiler_contribution','compiler_binding','native_cleanup','native_pending_original','native_publication_staging']).map(|$row| $row.attempt));
    FOR $attempt IN $attempts { IF $attempt=NONE OR (SELECT VALUE id FROM ONLY $attempt)=NONE { THROW 'native incidence attempt missing'; }; };
    LET $contributions=array::distinct(array::concat($rows.filter(|$row| record::table($row.id)='compiler_membership').map(|$row| $row.contribution),$all.filter(|$row| record::table($row.id)='native_hold' AND record::table($row.owner)='compiler_contribution').map(|$row| $row.owner)));
    FOR $contribution IN $contributions {
        LET $stored=SELECT * FROM ONLY $contribution FOR UPDATE;
        LET $parent=IF $stored=NONE { $rows.filter(|$row| $row.id=$contribution)[0] ?? NONE } ELSE { $stored };
        IF $parent=NONE OR type::is_bool($parent.completed)=false { THROW 'native contribution incidence parent missing'; };
        IF $parent.completed AND array::len($prepared.filter(|$entry| ($entry.row.contribution=$contribution OR $entry.row.owner=$contribution) AND $entry.existing!=$entry.row))>0 { THROW 'native completed contribution refuses changed incidence'; };
    };
    LET $completions=array::distinct(array::concat($rows.filter(|$row| record::table($row.id) IN ['native_completed_view','native_completed_contributor']).map(|$row| $row.completion),$all.filter(|$row| record::table($row.id)='native_hold' AND record::table($row.owner)='native_completed_state').map(|$row| $row.owner)));
    FOR $completion IN $completions {
        LET $stored=SELECT * FROM ONLY $completion FOR UPDATE;
        LET $parent=IF $stored=NONE { $rows.filter(|$row| $row.id=$completion)[0] ?? NONE } ELSE { $stored };
        IF $parent=NONE OR type::is_bool($parent.completed)=false { THROW 'native completed boundary parent missing'; };
        IF $parent.completed AND array::len($prepared.filter(|$entry| ($entry.row.completion=$completion OR $entry.row.owner=$completion) AND $entry.existing!=$entry.row))>0 { THROW 'native completed boundary refuses changed inventory'; };
    };
    LET $jobs=array::distinct($rows.filter(|$row| record::table($row.id)='native_retirement_item').map(|$row| $row.job));
    FOR $job IN $jobs { IF (SELECT VALUE id FROM ONLY $job)=NONE { THROW 'native incidence parent missing'; }; };
    FOR $blob IN $blobs {
        LET $stored=SELECT * FROM ONLY $blob FOR UPDATE;
        LET $parent=IF $stored=NONE { array::concat($rows,$originals).filter(|$row| $row.id=$blob)[0] ?? NONE } ELSE { $stored };
        IF $parent=NONE OR type::is_bool($parent.completed)=false { THROW 'native original blob declaration missing'; };
        IF $parent.completed AND array::len($prepared.filter(|$entry| $entry.row.blob=$blob AND $entry.existing!=$entry.row))>0 { THROW 'completed original blob refuses missing or changed piece'; };
    };
    FOR $entry IN $prepared { IF $entry.existing!=NONE AND $entry.existing!=$entry.row { THROW 'native immutable address collision'; }; };
    LET $new=$prepared.filter(|$entry| $entry.existing=NONE AND record::table($entry.row.id)!='native_hold').map(|$entry| $entry.row);
    LET $ordinary=$new.filter(|$row| record::table($row.id) NOT IN ['participant','reference','lex_occurs','vec_occurs']);
    LET $relations=$new.filter(|$row| record::table($row.id) IN ['participant','reference','lex_occurs','vec_occurs']);
    LET $new_originals=$original_prepared.filter(|$entry| $entry.existing=NONE).map(|$entry| $entry.row);
    IF array::len($new_originals)>0 { INSERT $new_originals RETURN NONE; };
    IF array::len($ordinary)>0 { INSERT $ordinary RETURN NONE; };
    IF array::len($relations)>0 { INSERT RELATION $relations RETURN NONE; };
    LET $owners=array::distinct($holds.map(|$row| $row.owner));
    FOR $owner IN $owners { IF (SELECT VALUE id FROM ONLY $owner)=NONE { THROW 'native attachment owner missing'; }; };
    LET $objects=array::distinct($holds.map(|$row| $row.object));
    FOR $object IN $objects { IF (SELECT VALUE id FROM ONLY $object)=NONE { THROW 'native attachment target missing'; }; };
    LET $new_holds=$prepared.filter(|$entry| $entry.existing=NONE AND record::table($entry.row.id)='native_hold').map(|$entry| $entry.row);
    IF array::len($new_holds)>0 { INSERT $new_holds RETURN NONE; };
    RETURN NONE;
};

// Complete attempt cuts are conservative for explicit recovery/cleanup/history
// incidence. They grant no production eligibility; each caller retains its own
// owner/state/incarnation checks. Ordinary windows use admitted context instead.
DEFINE FUNCTION fn::native_attempt_initialize($attempt: record<native_attempt>, $generation: string, $era: int, $epoch: int) {
    IF $era<=0 OR $epoch<=0 { THROW 'native attempt stripe initial identity'; };
    FOR $slot IN 0..64 { LET $id=type::record('native_attempt_stripe',[$attempt,$slot]); CREATE $id SET format=1,attempt=$attempt,generation=$generation,era=$era,epoch=$epoch,slot=$slot,revision=0 RETURN NONE; };
    RETURN NONE;
};
DEFINE FUNCTION fn::native_attempt_validate($attempt: record<native_attempt>, $generation: string, $era: int, $epoch: int) {
    LET $owner=SELECT * FROM ONLY $attempt FOR UPDATE;
    IF $owner=NONE OR $owner.generation!=$generation OR $owner.era!=$era OR $owner.epoch!=$epoch OR $era<=0 OR $epoch<=0 { THROW 'native attempt cut owner identity'; };
    IF array::len(SELECT VALUE id FROM native_attempt_stripe WITH INDEX attempt_stripes WHERE attempt=$attempt LIMIT 65)!=64 { THROW 'native attempt stripe set width'; };
    LET $ids=array::range(0,64).map(|$slot| type::record('native_attempt_stripe',[$attempt,$slot]));
    LET $rows=$ids.map(|$id| (SELECT * FROM ONLY $id FOR UPDATE));
    FOR $slot IN 0..64 { LET $row=$rows[$slot]; IF $row=NONE OR $row.id!=$ids[$slot] OR $row.format!=1 OR $row.attempt!=$attempt OR $row.generation!=$generation OR $row.era!=$era OR $row.epoch!=$epoch OR $row.slot!=$slot OR type::is_int($row.revision)=false OR $row.revision<0 OR $row.revision>9223372036854775807 { THROW 'native attempt stripe set authority'; }; };
    RETURN $rows;
};
DEFINE FUNCTION fn::native_attempt_cut($attempt: record<native_attempt>, $generation: string, $era: int, $epoch: int, $purpose: string) {
    IF $purpose NOT IN ['freeze','close','admission','completion','maintenance','incidence','history'] { THROW 'native attempt cut purpose'; };
    LET $rows=fn::native_attempt_validate($attempt,$generation,$era,$epoch);
    FOR $row IN $rows { IF $row.revision>=9223372036854775807 { THROW 'native attempt stripe revision exhausted'; }; UPDATE $row.id SET revision+=1 RETURN NONE; };
    RETURN NONE;
};
DEFINE FUNCTION fn::native_attempt_participate($context: object) {
    // FOR UPDATE rejects field idioms before evaluating their record-id value.
    // Bind that exact witness to a parameter; the lock and all checks stay intact.
    LET $effect_id=$context.effect;
    LET $effect=SELECT * FROM ONLY $effect_id FOR UPDATE;
    LET $owner=SELECT * FROM ONLY $context.attempt;
    IF $effect=NONE OR $effect.resolved OR $effect.attempt!=$context.attempt OR $effect.owner!=$context.attempt OR $effect.owner_kind NOT IN ['attempt','frozen_attempt','terminal_attempt'] OR $effect.generation!=$context.generation OR $effect.era!=$context.era OR $effect.epoch!=$context.epoch OR $effect.request!=$context.request OR $effect.attempt_slot!=$context.slot OR $context.slot<0 OR $context.slot>=64 OR $owner=NONE OR $owner.generation!=$context.generation OR $owner.era!=$context.era OR $owner.epoch!=$context.epoch OR $context.epoch<=0 { THROW 'native attempt participant correspondence'; };
    LET $id=type::record('native_attempt_stripe',[$context.attempt,$context.slot]);
    LET $row=SELECT * FROM ONLY $id FOR UPDATE;
    IF $row=NONE OR $row.format!=1 OR $row.attempt!=$context.attempt OR $row.generation!=$context.generation OR $row.era!=$context.era OR $row.epoch!=$context.epoch OR $row.slot!=$context.slot OR type::is_int($row.revision)=false OR $row.revision<0 OR $row.revision>=9223372036854775807 { THROW 'native attempt participant stripe'; };
    UPDATE $id SET revision+=1 RETURN NONE;
    RETURN NONE;
};
DEFINE FUNCTION fn::native_touch_admitted($record: option<record>, $context: option<object>) {
    IF $record!=NONE { IF $context!=NONE AND record::table($record)='native_attempt' { IF $record!=$context.attempt { THROW 'native unrelated attempt incidence'; }; } ELSE { fn::native_touch($record); }; };
    RETURN NONE;
};
DEFINE FUNCTION fn::native_incidence_admitted($row: object, $context: option<object>) {
    FOR $record IN fn::native_incidence_targets($row) { fn::native_touch_admitted($record,$context); };
    RETURN NONE;
};
DEFINE FUNCTION fn::native_incidence_insert_admitted($row: object, $context: option<object>) {
    fn::native_incidence_admitted($row,$context);
    IF record::table($row.id) IN ['compiler_contribution','compiler_binding','native_cleanup','native_pending_original','native_publication_staging'] { IF $row.attempt=NONE OR (SELECT VALUE id FROM ONLY $row.attempt)=NONE { THROW 'native incidence attempt missing'; }; };
    IF record::table($row.id)='compiler_membership' { fn::native_contribution_incidence($row.contribution,$row); };
    IF record::table($row.id) IN ['native_completed_view','native_completed_contributor'] { fn::native_completed_incidence($row.completion,$row); };
    IF record::table($row.id)='native_hold' AND record::table($row.owner)='native_completed_state' { fn::native_completed_incidence($row.owner,$row); };
    IF record::table($row.id)='native_retirement_item' { IF (SELECT VALUE id FROM ONLY $row.job)=NONE { THROW 'native incidence parent missing'; }; };
    RETURN NONE;
};
