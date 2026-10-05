"""Model-shaped synthetic data: rows for any model relation, with every reference closed.

Values follow the declared types, codebooks, sum arms and subtype constraints. Referenced rows are
reused or created on demand (an id is registered before its references are filled, so reference
cycles close on existing rows). Ids are random 16-byte hex strings: content-identity derivation is
not reproduced (see README: it is not expressible in SurrealQL and stays with the model).
"""
import random


class Synth:
    def __init__(self, model, seed=7, reuse=0.7):
        self.m = model
        self.rnd = random.Random(seed)
        self.reuse = reuse
        self.rows = {}  # relation -> list of rows

    def pool(self, name):
        """Rows of a relation; a lazy mapping (dict with __missing__) loads it on first use."""
        if hasattr(type(self.rows), "__missing__"):
            return self.rows[name]
        return self.rows.setdefault(name, [])

    def hexid(self):
        return "%032x" % self.rnd.getrandbits(128)

    def scalar(self, f):
        t = f["type"]
        if "codes" in f:
            return self.rnd.choice([c[0] for c in f["codes"]])
        if t == "int16":
            return self.rnd.randint(0, 300)
        if t in ("int32", "int64"):
            return self.rnd.randint(0, 10**6)
        if t == "bool":
            return self.rnd.random() < 0.5
        if t == "text":
            return self.rnd.choice(["alpha", "beta", "gamma", "délta", 'q"uote']) + str(self.rnd.randint(0, 999))
        if t == "digest":
            return "%064x" % self.rnd.getrandbits(256)
        if t == "binary":
            return bytes(self.rnd.getrandbits(8) for _ in range(self.rnd.randint(0, 8)))
        if t == "finite_f64":
            return round(self.rnd.uniform(-5, 5), 3)
        raise ValueError(t)

    def ref(self, f, depth, subtype=None):
        tgt = f["target"]
        pool = self.pool(tgt)
        if subtype is not None:
            tag = self.m.rel[tgt]["sum"]["tag"]
            pool = [r for r in pool if r.get(tag) == subtype]
        if pool and (depth > 3 or self.rnd.random() < self.reuse):
            return self.rnd.choice(pool)["id"]
        return self.make(tgt, depth=depth + 1, tag=subtype)["id"]

    def make(self, name, depth=0, tag=None, **fixed):
        rel = self.m.rel[name]
        row = {"id": self.hexid()}
        self.pool(name).append(row)  # register first: cycles close on this row
        arm_of, chosen = {}, None
        if "sum" in rel:
            s = rel["sum"]
            for arm in s["arms"]:
                for fl in arm["fields"]:
                    arm_of[fl["name"]] = (arm["code"], fl["required"])
            codes = [a["code"] for a in s["arms"]]
            chosen = tag if tag is not None else fixed.get(s["tag"], self.rnd.choice(codes))
            row[s["tag"]] = chosen
        for f in rel["fields"]:
            n = f["name"]
            if n in row:
                continue
            if n in fixed:
                row[n] = fixed[n]
                continue
            if n in arm_of:
                code, required = arm_of[n]
                if code != chosen or (not required and self.rnd.random() < 0.5):
                    row[n] = None
                    continue
            elif f["nullable"] and self.rnd.random() < 0.3:
                row[n] = None
                continue
            if f["type"] == "id":
                if f["list"]:
                    row[n] = [self.ref(f, depth) for _ in range(self.rnd.randint(0, 2))]
                else:
                    row[n] = self.ref(f, depth, f.get("subtype"))
            elif f["list"]:
                row[n] = [self.scalar(f) for _ in range(self.rnd.randint(0, 3))]
            else:
                row[n] = self.scalar(f)
        return row
