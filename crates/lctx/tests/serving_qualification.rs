//! The operator runner is also exercised against real compiled disposable PG18 data.
#[path = "fixtures/serving_support.rs"]
mod support;
use std::path::Path;
use support::{ServingFixture, write};

fn runner(fixture: &ServingFixture, profile: &str, receipts: &Path) -> std::process::Command {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut command = std::process::Command::new("uv");
    command
        .current_dir(&root)
        .args([
            "run",
            "--no-sync",
            "python",
            "scripts/qualify_serving.py",
            "--library",
            "demo",
            "--profile",
            profile,
            "--config",
        ])
        .arg(fixture.dir.path().join("postgres-serving.json"))
        .arg("--generation")
        .arg(fixture.generation.hex())
        .arg("--source-file")
        .arg(fixture.dir.path().join("source.py"))
        .arg("--anchors")
        .arg(fixture.dir.path().join("anchors.json"))
        .arg("--receipt-dir")
        .arg(receipts);
    command
}

#[tokio::test]
async fn runner_uses_actual_stdio_and_challenges_received_generation_evidence_channels() {
    // These bytes and signatures are independent of the runner and server helpers.
    let source = format!(
        "\"\"\"Public api consume method signature source evidence.\n{}\"\"\"\n{}",
        "🦀 café\n".repeat(5000),
        r#"__all__ = ['api', 'consume', 'Holder']
def api(flag: bool = False) -> bool:
    return flag
def consume(value: int) -> int:
    return value
class Holder:
    def method(self, token: str = 'ready') -> str:
        if not isinstance(token, str):
            raise TypeError('token must be str')
        return token
"#
    )
    .into_bytes();
    let anchors = serde_json::json!([
        {"path":"demo.api", "qualname":"api"},
        {"path":"demo.consume", "qualname":"consume"},
        {"path":"demo.Holder.method", "qualname":"Holder.method"}
    ]);
    for profile in ["catalog", "behavioral"] {
        let fixture = ServingFixture::start_profile(&source, profile).await;
        write(&fixture.dir.path().join("source.py"), &source);
        write(
            &fixture.dir.path().join("anchors.json"),
            anchors.to_string(),
        );
        let receipt_root = std::env::var_os("LCTX_SERVING_QUALIFICATION_RECEIPTS_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("lctx-serving-qualification"));
        let receipts = receipt_root.join(format!("{}-{profile}", fixture.generation.hex()));
        let output = runner(&fixture, profile, &receipts).output().unwrap();
        assert!(
            output.status.success(),
            "{profile}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let receipt: serde_json::Value =
            serde_json::from_slice(&std::fs::read(receipts.join("receipt.json")).unwrap()).unwrap();
        assert_eq!(receipt["outcome"], "passed");
        assert_eq!(receipt["generation"], fixture.generation.hex());
        assert_eq!(receipt["tools"].as_array().unwrap().len(), 10);
        assert_eq!(
            std::fs::read(receipts.join("original-source.bin")).unwrap(),
            source
        );
        // Read actual protocol replies, then challenge independent qualification assertions.
        // This proves the oracle detects bad values rather than trusting successful dispatch.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let fault = std::process::Command::new("uv").current_dir(root)
            .args(["run", "--no-sync", "python", "-c", r#"
import copy, importlib.util, json, pathlib, sys
spec = importlib.util.spec_from_file_location('qualify', 'scripts/qualify_serving.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)
folder = pathlib.Path(sys.argv[1])
receipt = json.loads((folder/'receipt.json').read_text())
dtos = [json.loads(p.read_bytes()).get('result', {}).get('structuredContent') for p in folder.glob('*.response.jsonl')]
dtos = [d for d in dtos if d]
generation = dtos[0]
q.check_generation(generation, list(bytes.fromhex(receipt['generation'])))
ranked = next(d for d in dtos if 'channels' in d)
q.check_channels(ranked)
original = (folder/'original-source.bin').read_bytes()
q.check_original(original, pathlib.Path(sys.argv[2]).read_bytes())
core = next(d['operation']['packet']['core'] for d in dtos if 'operation' in d and d['operation'].get('packet', {}).get('core', {}).get('name') == 'demo.api')
node = q.source_functions(original)['api']
schema = json.loads(q.wire_tool('get_operation'))['output_schema']
q.check_signature(core, node, schema)
wrong_default = copy.deepcopy(core)
default_id = next(p['default']['literal'] for s in wrong_default['signatures'] for p in s['parameters'] if p['name'] == 'flag')
next(l for l in wrong_default['literal_values'] if l['literal'] == default_id)['value'] = {'kind':'bool','value':True}
wrong_channel = copy.deepcopy(ranked)
wrong_channel['channels']['vector'] = {'status':'available'}
for check, arguments in [(q.check_generation,(generation,[255]*16)),(q.check_channels,(wrong_channel,)),(q.check_original,(original,original+b'\n# wrong source\n')),(q.check_signature,(wrong_default,node,schema))]:
    try:
        check(*arguments)
    except AssertionError:
        pass
    else:
        raise AssertionError('fault escaped independent check')
assert len([d for d in dtos if 'evidence' in d]) > 1, 'actual byte continuation required'
print('actual received generation/evidence/channel/default faults detected')
"#]).arg(&receipts).arg(fixture.dir.path().join("source.py"))
            .output().unwrap();
        assert!(
            fault.status.success(),
            "{}",
            String::from_utf8_lossy(&fault.stderr)
        );
        println!("{profile}: {}", String::from_utf8_lossy(&fault.stdout));
        fixture.finish().await;
    }
}
