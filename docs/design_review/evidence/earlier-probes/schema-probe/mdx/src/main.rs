use markdown::mdast::{AttributeContent, AttributeValue, Node};
fn walk(n: &Node, text: &str, depth: usize, ancestors: &mut Vec<String>) {
    let (name, attrs, kind) = match n {
        Node::MdxJsxFlowElement(e) => (e.name.clone(), &e.attributes, "flow"),
        Node::MdxJsxTextElement(e) => (e.name.clone(), &e.attributes, "text"),
        _ => {
            for c in n.children().into_iter().flatten() { walk(c, text, depth, ancestors); }
            return;
        }
    };
    let p = n.position().unwrap();
    let kids = n.children().unwrap();
    let inner = kids.first().zip(kids.last()).map(|(a, z)| {
        let (s, e) = (a.position().unwrap().start.offset, z.position().unwrap().end.offset);
        text[s..e].to_owned()
    });
    let a: Vec<String> = attrs.iter().map(|a| match a {
        AttributeContent::Property(p) => format!("{}={:?}", p.name, p.value.as_ref().map(|v| match v { AttributeValue::Literal(s) => s.clone(), AttributeValue::Expression(e) => format!("{{{}}}", e.value) })),
        AttributeContent::Expression(e) => format!("{{...{}}}", e.value),
    }).collect();
    println!("{:indent$}{kind} {:?} [{}..{}] attrs={a:?} within={ancestors:?} inner={inner:?}", "", name, p.start.offset, p.end.offset, indent = depth * 2);
    ancestors.push(name.unwrap_or_default());
    for c in kids { walk(c, text, depth + 1, ancestors); }
    ancestors.pop();
}
fn main() {
    let text = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let mut o = markdown::ParseOptions::mdx();
    o.constructs.frontmatter = true;
    let tree = markdown::to_mdast(&text, &o).unwrap();
    walk(&tree, &text, 0, &mut Vec::new());
}
