from api import parse as aliased, replacement, Box, apply
from api import replacement as simple
from reexport import public_parse as forwarded

def example(unknown):
    apply(replacement, 7)
    simple(6)
    aliased(1)
    forwarded('text')
    rebound = aliased
    rebound(2)
    rebound = replacement
    rebound(3)
    aliased(object())
    box = Box()
    box.run(4)
    unknown(5)
