"""Action triggers; static analyzer input, never executed."""
from json import dump
from gzip import compress
from atexit import register


def normal(value, sink):
    dump(value, sink, skipkeys=False, ensure_ascii=True, check_circular=True, allow_nan=True, cls=None, indent=None, separators=None, default=None, sort_keys=False)


def before_raise(value, sink):
    dump(value, sink, skipkeys=False, ensure_ascii=True, check_circular=True, allow_nan=True, cls=None, indent=None, separators=None, default=None, sort_keys=False)
    raise None


def after_raise(value, sink):
    raise None
    dump(value, sink, skipkeys=False, ensure_ascii=True, check_circular=True, allow_nan=True, cls=None, indent=None, separators=None, default=None, sort_keys=False)


def raising_argument(value, sink):
    dump(value, sink, skipkeys=1 / 0, ensure_ascii=True, check_circular=True, allow_nan=True, cls=None, indent=None, separators=None, default=None, sort_keys=False)


def after_opaque(value, sink, unknown):
    unknown()
    dump(value, sink, skipkeys=False, ensure_ascii=True, check_circular=True, allow_nan=True, cls=None, indent=None, separators=None, default=None, sort_keys=False)


def missing_defaults(value, sink):
    dump(value, sink)


def skipped(value, sink):
    if False:
        dump(value, sink, skipkeys=False, ensure_ascii=True, check_circular=True, allow_nan=True, cls=None, indent=None, separators=None, default=None, sort_keys=False)


def compression(value):
    compress(value, compresslevel=9, mtime=0)


def registration(callback):
    register(callback)


def acquisition(path):
    return open(path)
