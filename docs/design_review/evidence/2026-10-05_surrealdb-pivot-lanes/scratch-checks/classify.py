import re,sys
files=sys.argv[1:]
cats=["hop","plumb","budget","check","assembly"]
tot={c:0 for c in cats}
for f in files:
    src=open(f).read().split('\n')
    end=len(src)
    for i,l in enumerate(src):
        if l.strip().startswith('#[cfg(') and 'test' in l: end=i;break
    text='\n'.join(src[:end])
    # split on ; and { } boundaries keeping line counts
    stmts=re.split(r'(?<=[;{}])',text)
    c={k:0 for k in cats}
    for s in stmts:
        n=s.count('\n')
        if n==0: n=0
        if re.search(r'\.(read_for|read_ids|visit_for|visit_verified|visit_named)\b|sqlx::query|visit_physical',s): k="hop"
        elif re.search(r'\.query\(|Box::pin|PacketLease::new|shares_guard|check_execution|\.confirm\(\)|Error::State',s): k="plumb"
        elif re.search(r'reserve\(|retain\(|try_resize|charge|StateCharge|saturating_mul\(size_of',s): k="budget"
        elif re.search(r'Err\(Error::Contract\)|required\(|need\(|ok_or\(Error::Contract',s) and len(s)<400: k="check"
        else: k="assembly"
        c[k]+=n
    print(f, end, c)
    for k in cats: tot[k]+=c[k]
print("TOTAL",tot,sum(tot.values()))
