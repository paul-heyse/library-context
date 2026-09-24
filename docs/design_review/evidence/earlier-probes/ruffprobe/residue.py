import collections, sys
exec(open("parity.py").read().split("missing = ")[0])
missing = [k for k in ours if k not in ty]
site = "/home/paul/library-context/build/envs/fastmcp/lib/python3.14/site-packages/"
for k in missing[:36]:
    src = open(site + k[0], "rb").read()
    ls = src.rfind(b"\n", 0, k[1]) + 1; le = src.find(b"\n", k[1])
    print(k[0], k[1], ours[k]["name"], "|", src[ls:le].decode().strip()[:100])
