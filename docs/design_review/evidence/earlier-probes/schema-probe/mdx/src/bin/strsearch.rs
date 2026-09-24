fn main(){ let t=std::fs::read_to_string("sample.mdx").unwrap(); for (a,z) in warnings(&t){ println!("{:?}", &t[a..z]); } }
pub fn warnings(passage: &str) -> Vec<(usize, usize)> {
    const OPEN: &str = "<Warning>";
    const CLOSE: &str = "</Warning>";
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(open) = passage[at..].find(OPEN).map(|i| at + i + OPEN.len()) {
        let Some(close) = passage[open..].find(CLOSE).map(|i| open + i) else {
            break;
        };
        let inner = &passage[open..close];
        let lead = inner.len() - inner.trim_start().len();
        let body = inner.trim();
        if !body.is_empty() {
            out.push((open + lead, open + lead + body.len()));
        }
        at = close + CLOSE.len();
    }
    out
}
