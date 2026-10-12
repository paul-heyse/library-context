"""Focused controls for the explicitly authorized design campaign disposition."""
from __future__ import annotations
import contextlib
import json
from pathlib import Path
from types import SimpleNamespace
import pytest
import surrealdb_campaign as campaign
import surrealdb_service as service
import surrealdb_fixture as fixture
import surrealdb_pd02_recovery as recovery
import storage_service
import storage_lifecycle
SQL=service.sql


@pytest.fixture
def ready(tmp_path,monkeypatch):
    root=tmp_path/'library-context';root.mkdir()
    old=tmp_path/'library-context-wt/efficiency-source5'
    run=old/'build/runs'/campaign.RUN;run.mkdir(parents=True)
    monkeypatch.setattr(service,'ROOT',root)
    directory=tmp_path/'installation';directory.mkdir()
    (directory/'attachments'/campaign.ATTACHMENT/'native-drain').mkdir(parents=True)
    old_generation=list(bytes([4])*32)
    record={'installation_id':campaign.CAMPAIGN,'kind':'campaign','phase':'ready',
        'service_generation':old_generation,'endpoint':campaign.ENDPOINT,
        'grpc_endpoint':campaign.ENDPOINT.replace('http','grpc'),'namespace':service.NAMESPACE,
        'databases':list(service.DATABASES),'binary':{'path':str(tmp_path/'server')},
        'installer':str(tmp_path/'old-installer'),'native_schema_version':5}
    record['unit']=service.installation_unit(record)
    installation=service.Installation(directory,record)
    service.private_json(directory/'installation.json',record)
    for db in service.DATABASES:
        cfg={'database':db,'cache_database':db,'endpoint':record['grpc_endpoint'],'namespace':service.NAMESPACE,
            'service_generation':old_generation,'authentication':'database','username':db+'_writer','password':'old',
            'viewer_username':db+'_viewer','viewer_password':'old-viewer','selection':str(directory/f'{db}-selected.json'),
            'reuse':{'database':db,'capacity_bytes':service.PRODUCT_CAPACITY_BYTES,'lease_directory':str(directory/'product-leases'/db)}}
        (directory/'product-leases'/db).mkdir(parents=True)
        service.private_json(installation.runtime_path(db),cfg)
        service.private_json(installation.runtime_path(db,installer=True),{**cfg,'authentication':'root','username':'root','password':'root'})
        (directory/f'{db}-selected.json').write_text('old selected handle')
    original_owner={'pid':123,'boot_id':'dead','start_ticks':1,'pid_namespace':1}
    current_owner={'pid':321,'boot_id':'current','start_ticks':2,'pid_namespace':1}
    executable=tmp_path/'old-executable';executable.write_text('original executable');executable.chmod(0o700)
    for name in ('efficiency_source5_upper.rs','efficiency_source5_seed.rs'):
        path=old/'crates/lctx-publisher/tests'/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text(name)
    producer={'executable':str(executable),'executable_sha256':service.file_sha256(executable),
        'source_sha256':service.file_sha256(old/'crates/lctx-publisher/tests/efficiency_source5_upper.rs'),
        'seed_source_sha256':service.file_sha256(old/'crates/lctx-publisher/tests/efficiency_source5_seed.rs')}
    (old/'scripts').mkdir();(old/'scripts/efficiency_source5_upper.py').write_text('owned original wrapper')
    (old/'.campaign/source5').mkdir(parents=True)
    service.private_json(old/'.campaign/source5/upper-producer.json',producer)
    original={'id':campaign.ATTACHMENT,'checkout':str(root),'maintenance':True,'released':False,'owner':original_owner,
        'command':{'argv':[str(executable),'--ignored','--exact','populate_genuine_source5_upper_artifact','--nocapture']},
        'command_cleanup':{'status':'unknown'},'interrupted':True,'native_drain':{'attachment':campaign.ATTACHMENT,
            'directory':str(directory/'attachments'/campaign.ATTACHMENT/'native-drain'),'generation':bytes(old_generation).hex()}}
    service.private_json(directory/'attachments'/campaign.ATTACHMENT/'record.json',original)
    service.private_json(directory/'maintenance.json',{'operation':'maintenance','owner':original_owner,'token':'original'})
    (run/'output.log').write_text('original interrupted output')
    service.private_json(run/'record.json',{'id':campaign.RUN,'cwd':str(old),'phase':'ended','termination':'cancelled',
        'argv':['env','LCTX_SURREAL_SERVICE_CONFIG='+str(directory/'installation.json'),'uv','run','--no-project','--offline','--no-python-downloads','--python','3.14.7','scripts/efficiency_source5_upper.py','--produce'],
        'exit':{'code':143},'cleanup':{'status':'confirmed','survivors':[]},'logs':{'output':str(run/'output.log')}})
    monkeypatch.setattr(campaign.ProcessIdentity,'from_json',lambda value:SimpleNamespace(alive=lambda:False,foreign=lambda:False))
    monkeypatch.setattr(campaign.ProcessIdentity,'of',lambda pid=None:SimpleNamespace(to_json=lambda:current_owner, pid=321,alive=lambda:True))
    monkeypatch.setattr(fixture,'_command_observation',lambda row:{'status':'gone','members':[]})
    monkeypatch.setattr(storage_lifecycle,'admission',lambda *a,**kw:contextlib.nullcontext())
    candidate=tmp_path/'current-lctx';candidate.write_text('current executable');candidate.chmod(0o700)
    monkeypatch.setattr(storage_service,'prepare',lambda directory,source:{'path':str(source),'sha256':service.file_sha256(source)})
    monkeypatch.setattr(recovery,'validate_native_server',lambda inst:None)
    monkeypatch.setattr(recovery,'native_daemon_snapshot',lambda *a:{'actual':'checked'})
    monkeypatch.setattr(service.Installation,'check_daemon',lambda inst:None)
    monkeypatch.setattr(service,'unit_properties',lambda *a:{'InvocationID':'a'*32,'MainPID':'321','ActiveState':'active'})
    actions=[];databases=set(service.DATABASES);markers={};fail_init=[False];lost_remove=[False]
    def joined(inst,plan,**kw):
        actions.append('joined')
        daemon=plan['daemon'];entry={'MESSAGE':'SIGTERM received. Waiting for graceful shutdown;',
            '_PID':str(daemon['process']['pid']),'_SYSTEMD_INVOCATION_ID':daemon['invocation'],'_SYSTEMD_USER_UNIT':inst.unit}
        text=json.dumps(entry)+'\n'
        return {'state':{'InvocationID':daemon['invocation'],'MainPID':'0','ActiveState':'inactive','Result':'success','ExecMainCode':'1','ExecMainStatus':'0'},
            'journal':{'text':text,'sha256':recovery.digest_bytes(text.encode()),'entries':[entry]},'source_proof':recovery.SOURCE_JOIN}
    monkeypatch.setattr(recovery,'joined_exit',joined)
    monkeypatch.setattr(service,'_owned_daemon_start',lambda inst:actions.append('start'))
    def sql(inst,statement,*,cfg):
        if statement=='INFO FOR NS;':return [{'result':{'databases':{db:{} for db in databases}}}]
        if statement.startswith('REMOVE DATABASE '):
            db=statement.split()[2].rstrip(';');actions.append('remove-'+db);databases.discard(db)
            if lost_remove[0]:lost_remove[0]=False;raise service.FixtureFailed('sql','reply lost')
            return [{'result':None}]
        if statement.startswith('DEFINE DATABASE '):
            db=statement.split()[5];databases.add(db);actions.append('create-'+db);return [{'result':None}]
        if statement.startswith('SELECT generation,'):return [{'result':[markers[cfg['database']]]}]
        pytest.fail(statement)
    monkeypatch.setattr(service,'sql',sql)
    target={'schema_version':6,'schema':'b'*64}
    def cli(inst,candidate,db,action,**kw):
        actions.append(action+'-'+db)
        if action=='schema':return target
        if action=='init':
            assert kw=={'keep_closed':True}
            if fail_init[0]:fail_init[0]=False;raise service.FixtureFailed('init','interrupted')
            markers[db]={'generation':bytes(inst.record['service_generation']).hex(),'schema':target['schema'],'schema_version':6,'admission_open':False}
        return {'ready':action=='check'}
    monkeypatch.setattr(service,'_installer_cli',cli)
    monkeypatch.setattr(service,'_run_installer',lambda inst,exe,action='init':actions.append(action))
    monkeypatch.setattr(service.Installation,'check',lambda inst,**kw:{'ready':True})
    return SimpleNamespace(installation=installation,run=run,candidate=candidate,actions=actions,
        original=original,fail_init=fail_init,lost_remove=lost_remove,databases=databases,markers=markers)


def execute(ready):
    return campaign.regenerate(ready.installation,ready.candidate,campaign.ATTACHMENT,ready.run)


def test_join_precedes_all_discard_and_unknown_original_outcome_is_preserved(ready):
    result=execute(ready)
    assert result['original_native_outcome']=='unknown'
    assert ready.actions.index('joined')<ready.actions.index('remove-main')<ready.actions.index('init-main')<ready.actions.index('open-admission')
    directory=ready.installation.directory/'attachments'/campaign.ATTACHMENT
    actual=json.loads((directory/'record.json').read_text())
    assert actual['released'] and actual['native_drain']==ready.original['native_drain']
    assert fixture.native_completion(directory,actual)=={'status':'not_observed','owners':[]}
    assert campaign.discarded(directory,actual)
    assert not fixture.native_protected(directory,actual,{'status':'unknown','owners':[]})
    assert not (ready.installation.directory/'maintenance.json').exists()
    assert all(not (ready.installation.directory/f'{db}-selected.json').exists() for db in service.DATABASES)


def test_interrupted_current_init_resumes_without_discarding_regenerated_database(ready):
    ready.fail_init[0]=True
    with pytest.raises(service.FixtureFailed,match='interrupted'):execute(ready)
    assert (ready.installation.directory/'maintenance.json').exists()
    execute(ready)
    assert ready.actions.count('joined')==1
    assert ready.actions.count('remove-main')==1 and ready.actions.count('remove-validation')==1


def test_remove_lost_acknowledgement_uses_actual_namespace_absence(ready):
    ready.lost_remove[0]=True
    execute(ready)
    assert ready.actions.count('remove-main')==1


@pytest.mark.parametrize('change',['scope','unnamed','source','command','native'])
def test_preflight_refuses_before_daemon_or_database_effects(ready,monkeypatch,change):
    if change=='scope':ready.installation.record['kind']='stable'
    elif change=='unnamed':monkeypatch.setattr(service,'borrowers',lambda inst:[{'id':'other'}])
    elif change=='source':(ready.run/'record.json').write_text('{}')
    elif change=='command':monkeypatch.setattr(fixture,'_command_observation',lambda row:{'status':'unknown'})
    elif change=='native':(ready.installation.directory/'attachments'/campaign.ATTACHMENT/'native-drain'/'unexpected').write_text('unknown')
    with pytest.raises(service.FixtureBlocked):execute(ready)
    assert not any(action in ready.actions for action in ['joined','start','remove-main','remove-validation'])


def test_join_failure_never_discards_or_releases_unknown_attachment(ready,monkeypatch):
    monkeypatch.setattr(recovery,'valid_join_evidence',lambda *a,**kw:False)
    with pytest.raises(service.FixtureBlocked,match='registered execution join'):execute(ready)
    assert 'remove-main' not in ready.actions and 'open-admission' not in ready.actions
    path=ready.installation.directory/'attachments'/campaign.ATTACHMENT/'record.json'
    assert json.loads(path.read_text())==ready.original


def test_changed_immutable_disposition_never_releases_native_protection(ready):
    execute(ready)
    directory=ready.installation.directory/'attachments'/campaign.ATTACHMENT
    row=json.loads((directory/'record.json').read_text())
    path=Path(row['campaign_disposition']['path']);path.chmod(0o600)
    value=json.loads(path.read_text());value['original_native_outcome']='confirmed';path.write_text(json.dumps(value))
    assert not campaign.discarded(directory,row)
    assert fixture.native_protected(directory,row,{'status':'unknown','owners':[]})


@pytest.mark.parametrize('args',[
 ['--regenerate-campaign'],['--installer','/candidate'],
 ['--regenerate-campaign','--installer','/candidate','--discard-attachment',campaign.ATTACHMENT],
 ['--regenerate-campaign','--installer','/candidate','--discard-attachment',campaign.ATTACHMENT,'--discard-run','/run','--','true'],
 ['--regenerate-campaign','--recover']])
def test_explicit_regeneration_arguments_cannot_be_partial_or_mixed(args):
    with pytest.raises(SystemExit):service.main(['maintenance',*args])


@pytest.mark.parametrize("change",["snapshot","recipe"])
def test_pending_join_resume_rechecks_actual_original_executable_and_source_recipe(ready,monkeypatch,change):
    joined=recovery.joined_exit
    monkeypatch.setattr(recovery,"joined_exit",lambda *a,**kw:(_ for _ in ()).throw(service.FixtureFailed("join","interrupted before signal")))
    with pytest.raises(service.FixtureFailed):execute(ready)
    monkeypatch.setattr(recovery,"joined_exit",joined)
    if change=="snapshot":monkeypatch.setattr(recovery,"native_daemon_snapshot",lambda *a:{"changed":True})
    else:monkeypatch.setattr(recovery,"validate_native_server",lambda *a:(_ for _ in ()).throw(service.FixtureBlocked("source","recipe changed")))
    ready.actions.clear()
    with pytest.raises(service.FixtureBlocked):execute(ready)
    assert "joined" not in ready.actions and "remove-main" not in ready.actions


def test_successful_exit_before_join_receipt_publication_resumes_from_exact_inactive_invocation(ready,monkeypatch):
    immutable=campaign.immutable
    def crash(path,value):
        if path.name=="joined.json":raise OSError("after daemon exit before receipt")
        return immutable(path,value)
    monkeypatch.setattr(campaign,"immutable",crash)
    with pytest.raises(OSError,match="after daemon exit"):execute(ready)
    monkeypatch.setattr(campaign,"immutable",immutable)
    monkeypatch.setattr(service,"unit_properties",lambda *a:{"InvocationID":"a"*32,"MainPID":"0","ActiveState":"inactive"})
    monkeypatch.setattr(recovery,"native_daemon_snapshot",lambda *a:pytest.fail("exited process has no readable /proc identity"))
    execute(ready)
    assert ready.actions.count("remove-main")==1


def test_pending_disposition_protects_original_evidence_and_candidate_storage(ready):
    ready.fail_init[0]=True
    with pytest.raises(service.FixtureFailed):execute(ready)
    dependencies=storage_service.dependencies(ready.installation)["dependencies"]
    paths={row["path"] for row in dependencies}
    assert str(ready.run/"record.json") in paths and str(ready.candidate) in paths
    assert any(row["role"]=="pending-campaign-disposition-evidence" for row in dependencies)


def test_generic_recover_refuses_pending_campaign_disposition(ready,monkeypatch,capsys):
    ready.fail_init[0]=True
    with pytest.raises(service.FixtureFailed):execute(ready)
    monkeypatch.setattr(service.Installation,"load",lambda **kw:ready.installation)
    assert service.main(["maintenance","--recover"])==75
    assert "--regenerate-campaign" in capsys.readouterr().err
    assert "open-admission" not in ready.actions


def test_removed_scope_receipt_must_match_exact_checked_join_before_resume(ready):
    ready.fail_init[0]=True
    with pytest.raises(service.FixtureFailed):execute(ready)
    path=ready.installation.directory/"campaign-regeneration"/campaign.ATTACHMENT/"main-removed.json"
    path.chmod(0o600);value=json.loads(path.read_text());value["database"]="foreign";path.write_text(json.dumps(value));path.chmod(0o400)
    ready.actions.clear()
    with pytest.raises(service.FixtureBlocked,match="database receipt differs"):execute(ready)
    assert "remove-main" not in ready.actions and "init-main" not in ready.actions



def test_current_installer_can_acquire_same_product_lifecycle_after_old_scope_removal(ready,monkeypatch):
    cli=service._installer_cli
    def checked(inst,candidate,db,action,**kw):
        if action=="init":
            path=inst.directory/"product-leases"/db/"lifecycle.lock"
            descriptor=campaign.try_hold_lock(path)
            assert descriptor is not None,"current actual product installer must not deadlock on our old lease"
            campaign.os.close(descriptor)
        return cli(inst,candidate,db,action,**kw)
    monkeypatch.setattr(service,"_installer_cli",checked)
    execute(ready)


def test_live_product_lifecycle_refuses_before_daemon_signal_or_scope_discard(ready):
    path=ready.installation.directory/"product-leases/main/lifecycle.lock"
    descriptor=campaign.try_hold_lock(path)
    try:
        with pytest.raises(service.FixtureBlocked,match="product lifecycle lease"):execute(ready)
    finally:campaign.os.close(descriptor)
    assert "joined" not in ready.actions and "remove-main" not in ready.actions


def test_runtime_selection_escape_refuses_before_discard(ready):
    path=ready.installation.runtime_path("main")
    cfg=json.loads(path.read_text());cfg["selection"]=str(ready.run/"foreign-selected.json")
    service.private_json(path,cfg)
    with pytest.raises(service.FixtureBlocked,match="escaped"):execute(ready)
    assert "joined" not in ready.actions and "remove-main" not in ready.actions



def test_namespace_discard_administration_omits_nonexistent_database_header(ready,monkeypatch):
    import io
    requests=[]
    monkeypatch.setattr(service,"local_urlopen",lambda request,**kw:
        requests.append(request) or contextlib.nullcontext(io.BytesIO(b'[{"status":"OK","result":{}}]')))
    root=ready.installation.runtime("main",installer=True)
    SQL(ready.installation,"INFO FOR NS;",cfg={**root,"database":None})
    assert requests[0].get_header("Surreal-db") is None
    assert requests[0].get_header("Surreal-ns")==service.NAMESPACE
    with pytest.raises(service.FixtureBlocked,match="root authority"):
        SQL(ready.installation,"INFO FOR NS;",cfg={**root,"database":None,"authentication":"database"})
    assert len(requests)==1



def test_process_death_after_successful_reopen_resumes_by_checked_current_close_before_init(ready,monkeypatch):
    original_installer=service._run_installer
    def installer(inst,exe,action="init"):
        original_installer(inst,exe,action)
        if action in {"open-admission","close-admission"}:
            for marker in ready.markers.values():marker["admission_open"] = action=="open-admission"
    monkeypatch.setattr(service,"_run_installer",installer)
    publish=campaign.immutable
    def crash(path,value):
        if path.name=="complete.json":raise OSError("process died after reopening")
        return publish(path,value)
    monkeypatch.setattr(campaign,"immutable",crash)
    with pytest.raises(OSError,match="after reopening"):execute(ready)
    assert all(marker["admission_open"] for marker in ready.markers.values())
    monkeypatch.setattr(campaign,"immutable",publish)
    prior=len(ready.actions)
    execute(ready)
    resumed=ready.actions[prior:]
    assert resumed.index("schema-main") < resumed.index("close-admission") < resumed.index("init-main")
    assert ready.actions.count("remove-main")==1 and ready.actions.count("remove-validation")==1


def test_checked_resume_refuses_changed_current_generation_before_close(ready,monkeypatch):
    publish=campaign.immutable
    def crash(path,value):
        if path.name=="complete.json":raise OSError("before complete")
        return publish(path,value)
    monkeypatch.setattr(campaign,"immutable",crash)
    with pytest.raises(OSError):execute(ready)
    ready.markers["validation"]["generation"]="f"*64
    ready.actions.clear()
    with pytest.raises(service.FixtureBlocked,match="exact closed target"):execute(ready)
    assert "close-admission" not in ready.actions and "remove-main" not in ready.actions
