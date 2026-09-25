"""Execute the current, unchanged Model implementation against a synthetic use graph.

Uses the existing release cpg-schema rlib for real IDs, codebooks and bounded BDD
conditions. Only the SQL input-row structs are narrowed to the fields Model reads.
No Cargo rebuild or production-tree edit. Outputs and binary go to a temp dir.
"""

from pathlib import Path
import hashlib
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[4]
source = (ROOT / "crates/cpg-core/src/flow_model.rs").read_text()
model = source[source.index("/// Where a value comes from:"):source.index("/// A sink: its module,")]
header = r'''
#![allow(dead_code)]
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;
use cpg_schema::id::Id;
use cpg_schema::codebook::{BindingKind, FlowSink, LexicalScopeKind};
use cpg_schema::condition_kernel::{BoundedCondition as ModelCondition, KernelBoundary};
#[derive(Clone)]
struct UseRow { module_node_id: Id, start_byte: i64, end_byte: i64, scope_kind: LexicalScopeKind }
#[derive(Clone)]
struct DefRow { module_node_id: Id, parameter_node_id: Option<Id>, value_start_byte: Option<i64>, value_end_byte: Option<i64>, kind: BindingKind }
type ValueSources = HashMap<(Id, FlowSink, i64, i64), Vec<(Id, Id, Transfer, Id)>>;
fn id(n: u8) -> Id { Id([n;16]) }
'''
tail = r'''
fn model(cyclic: bool) -> Model {
    let parameter = DefRow { module_node_id:id(0), parameter_node_id:Some(id(9)), value_start_byte:None, value_end_byte:None, kind:BindingKind::Parameter };
    let assignment = |start| DefRow { module_node_id:id(0), parameter_node_id:None, value_start_byte:Some(start), value_end_byte:Some(start+1), kind:BindingKind::Assignment };
    Model {
        uses: HashMap::new(),
        defs: [(id(10),parameter),(id(11),assignment(11)),(id(12),assignment(12))].into(),
        // A = parameter p OR call(B); B = A. The acyclic control replaces B = A with B = p.
        reaching: [(id(2),vec![(Some(id(10)),id(1),false),(Some(id(11)),id(1),true)]),
                   (id(3),vec![(Some(if cyclic {id(12)} else {id(10)}),id(1),false)])].into(),
        values: [((id(0),FlowSink::Definition,11,12),vec![(id(21),id(3),Transfer::Call,id(1))]),
                 ((id(0),FlowSink::Definition,12,13),vec![(id(22),id(2),Transfer::Identity,id(1))])].into(),
        conditions: [(id(1), ModelCondition::always())].into(),
        captured:HashMap::new(), receivers:HashSet::new(), fields:HashMap::new(), memo:HashMap::new(), keep_receivers:false,
    }
}
fn transfers(m:&mut Model,u:Id)->Vec<Transfer> {
    m.reach(u,&mut Vec::new()).0.keys().map(|(_,t)|*t).collect()
}
fn main() {
    let mut a_first = model(true);
    let a = transfers(&mut a_first,id(2));
    let _ = transfers(&mut a_first,id(3));
    let a_again = transfers(&mut a_first,id(2));
    let mut b_first = model(true);
    let _ = transfers(&mut b_first,id(3));
    let b = transfers(&mut b_first,id(2));
    let control = transfers(&mut model(false),id(2));
    println!("cyclic A queried first: {a:?}; cached after B: {a_again:?}");
    println!("cyclic B queried first, then A: {b:?}");
    println!("acyclic control A: {control:?}");
    assert_eq!(a,vec![Transfer::Identity]);
    assert_eq!(a_again,a);
    assert_eq!(b,vec![Transfer::Identity,Transfer::Call]);
    assert_eq!(control,b);
    println!("passed: query-order-dependent loss of a loop-carried transfer reproduced");
}
'''
rlib = max((ROOT / "target/release/deps").glob("libcpg_schema-*.rlib"), key=lambda p: p.stat().st_mtime)
print("flow_model.rs SHA256", hashlib.sha256(source.encode()).hexdigest(), flush=True)
print("cpg_schema rlib", rlib.name, flush=True)
with tempfile.TemporaryDirectory(prefix="lctx-reach-review-") as temp:
    rust = Path(temp) / "reach.rs"
    binary = Path(temp) / "reach"
    rust.write_text(header + model + tail)
    subprocess.run(["rustc", "--edition=2024", "-C", "linker=clang", "-C", "link-arg=-fuse-ld=mold", str(rust), "-L", f"dependency={ROOT / 'target/release/deps'}", "--extern", f"cpg_schema={rlib}", "-o", str(binary)], cwd=ROOT, check=True)
    subprocess.run([str(binary)], cwd=ROOT, check=True)
