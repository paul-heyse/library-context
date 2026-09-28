"""PR2 disposable schema009→010 mixed-generation recovery rehearsal.

Requires the protected pre-cutover baseline and both current fixture generations. Never connects
to the operator database. Run with the repository's editable Python environment from its root.
"""
import json
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path.cwd()
sys.path[:0] = [str(ROOT / "scripts")]
import postgres_test_support as support
from postgres_expand import write_secret

BASELINE = ROOT / "build/pr2-baseline-8334c86/source"
OUTPUT = ROOT / (sys.argv[1] if len(sys.argv) > 1 else "build/pr2-mixed-recovery")


def main():
    OUTPUT.mkdir(exist_ok=False)
    support.ROOT = BASELINE
    with tempfile.TemporaryDirectory(prefix="lctx-pr2-rehearsal-") as tmp:
        config = Path(tmp)
        with support.database(config) as (_, sql, call, role, _):
            container = sql[3]
            call(["docker", "cp", str(BASELINE / "pre-cutover.dump"), container + ":/tmp/baseline.dump"])
            call(["docker", "exec", container, "pg_restore", "-U", "postgres", "--dbname=lctx", "--clean", "--if-exists", "--exit-on-error", "/tmp/baseline.dump"])
            cli = str(ROOT / "target/release/lctx")
            call([cli, "--database-config", str(config / "postgres.json"), "db", "migrate"])
            call([cli, "--database-config", str(config / "postgres.json"), "db", "check"])
            importer = config / "postgres-importer.json"
            write_secret(importer, {**role,"role":"importer","url":role["url"].replace("lctx_serving:","lctx_importer:"),"statement_timeout_seconds":30})
            serving = [cli,"serving","--importer-config",str(importer)]
            legacy = json.loads((BASELINE / "pre-cutover.dump.json").read_text())["inventory"]["generations"]
            call(sql,input="UPDATE lctx_serving.artifact_locations SET location='/unavailable/pr2/'||md5(location)||'/'||name;")
            for generation in legacy:
                call([*serving,"relocate-artifacts","--generation",generation["generation"],"--artifacts",str(BASELINE / "pre-cutover.dump.artifacts")])
            current = []
            for fixture in ["pr1-catalog-fixture","py-fixture"]:
                folder = ROOT / "build" / fixture
                generation = folder / (folder / "CURRENT").read_text().strip()
                manifest = json.loads((generation / "MANIFEST.json").read_text())
                call([*serving,"import-bundle","--bundle",str(generation),"--artifacts",str(OUTPUT / "artifacts")])
                call([*serving,"select","--library",manifest["library"],"--generation",manifest["projection_generation"]])
                current.append(manifest["projection_generation"])
            backup = [sys.executable,str(ROOT / "scripts/postgres_backup.py")]
            denied = OUTPUT / "legacy-selected.dump"
            call([*backup,"backup",str(denied),"--config",str(config / "postgres.json")])
            refusal = subprocess.run([*backup,"restore-drill",str(denied)],capture_output=True,text=True)
            assert refusal.returncode != 0 and "selected generation needs retained runtime" in refusal.stderr, refusal.stderr
            # Explicit test-only removal: prove current selections are usable; production never
            # silently deletes or substitutes the incompatible selected generation.
            call(sql,input="DELETE FROM lctx_serving.selections WHERE library='fastmcp';")
            archive = OUTPUT / "mixed.dump"
            call([*backup,"backup",str(archive),"--config",str(config / "postgres.json")])
            result = json.loads(call([*backup,"restore-drill",str(archive)]).stdout)
            receipt = json.loads(archive.with_suffix(".dump.json").read_text())
            assert result["all_generations_preserved"]
            served = result["serving"]["generations"]
            assert len(served) == len(legacy) + 2
            assert {r["generation"] for r in served if r.get("preservation") == "passed"} == {r["generation"] for r in legacy}
            assert {r["native_loaded"] for r in served if r["generation"] in current} == {False,True}
            item = next(g for g in receipt["inventory"]["generations"] if g["runtime_admission"] == "legacy_runtime_required")["artifacts"][0]
            path = archive.with_suffix(".dump.artifacts") / item["sha256"] / item["name"]
            original = path.read_bytes()
            try:
                path.write_bytes(b"corrupt")
                refusal = subprocess.run([*backup,"restore-drill",str(archive)],capture_output=True,text=True)
                assert refusal.returncode != 0 and "artifact" in refusal.stderr, refusal.stderr
            finally:
                path.write_bytes(original)
            result["legacy_selected_refused"] = True
            result["legacy_artifact_corruption_refused"] = True
            (OUTPUT / "receipt.json").write_text(json.dumps(result,indent=2)+"\n")
            print(json.dumps(result))


if __name__ == "__main__":
    main()
