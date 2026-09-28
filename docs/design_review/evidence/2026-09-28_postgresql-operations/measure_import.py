"""Explicit isolated-cluster import/index costs; cluster-wide WAL includes concurrent work."""
import argparse,json,subprocess,time,sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[4]/'scripts'))
from postgres_backup import connection_env
p=argparse.ArgumentParser();p.add_argument('bundle',type=Path);p.add_argument('config',type=Path);p.add_argument('artifacts',type=Path);p.add_argument('out',type=Path);a=p.parse_args()
config=json.loads(a.config.with_name('postgres-admin.json').read_text())
env=connection_env(config['migration_url'])
def state():
 sql="SELECT json_build_object('wal_bytes',(SELECT wal_bytes FROM pg_stat_wal),'database_bytes',pg_database_size(current_database()),'connections',(SELECT count(*) FROM pg_stat_activity WHERE datname=current_database()),'serving_index_bytes',(SELECT coalesce(sum(pg_indexes_size(c.oid)),0) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='lctx_serving' AND c.relkind='r'));"
 return json.loads(subprocess.run(['psql','-X','-qAt','-v','ON_ERROR_STOP=1'],input=sql,env=env,text=True,capture_output=True,check=True).stdout)
manifest=json.loads((a.bundle/'MANIFEST.json').read_text());before=state();start=time.monotonic()
base=['target/release/lctx','serving','--importer-config',str(a.config)]
receipt=json.loads(subprocess.run([*base,'import-bundle','--bundle',str(a.bundle),'--artifacts',str(a.artifacts)],text=True,capture_output=True,check=True).stdout)
imported=time.monotonic()-start;middle=state();start=time.monotonic()
subprocess.run([*base,'build-hnsw','--generation',manifest['projection_generation']],text=True,capture_output=True,check=True)
index=time.monotonic()-start;after=state()
value=dict(outcome='passed',generation=manifest['projection_generation'],import_seconds=imported,index_seconds=index,before=before,after_import=middle,after_index=after,wal_scope='cluster-wide; includes concurrent work',artifact_bytes=sum(p.stat().st_size for p in a.artifacts.rglob('*') if p.is_file()),receipt=receipt)
a.out.write_text(json.dumps(value,indent=2)+'\n');print(json.dumps(value))
