"""Call sites resolution leaves open: in a callee the value is forwarded to, and in the operation."""


def forward_open(data):
    return _helper(data)


def _helper(payload):
    sink = globals()["missing"]
    return sink(payload)


def own_open(fn, x):
    return fn(x)
