"""Stage 2's shapes: rebound fallbacks to settings, a mode test, a field stored and read in another
method, a parameter never read, a log-only use, and `**kwargs` rejected."""

import logging

import bpkg.config
from bpkg.config import settings

logger = logging.getLogger(__name__)


def _bind(host, port):
    return (host, port)


def _stdio():
    return None


def serve(host=None, port=None, transport="stdio"):
    host = host if host is not None else settings.host
    if port is None:
        port = settings.port
    if transport in ("http", "sse"):
        return _bind(host, port)
    return _stdio()


def debug_enabled():
    return bpkg.config.settings.debug


def ignore(a, b):
    return a


def start(stateless=False):
    logger.info(f"starting{' (stateless)' if stateless else ''}")
    return _stdio()


def make(**kwargs):
    if kwargs:
        raise TypeError(f"unexpected {sorted(kwargs)}")
    return _stdio()


class Session:
    def __init__(self, name, prior=None, mode="auto"):
        self.name = name
        self._prior = prior
        self._mode = mode

    def call(self, x):
        logger.info(f"{self.name}: calling")
        return x

    def adopt(self):
        if self._mode == "pinned":
            return self._prior
        return None


def paged(size=None, verbose=False):
    if size is not None and size <= 0:
        raise ValueError("size must be positive")
    if verbose:
        _stdio()
    return _bind(size, None)


def _describe(name):
    logger.info(f"describing {name}")
    return "described"


def labelled(name, level=None):
    chosen = (level if level is not None else settings.log_level).lower()
    return _describe(name), chosen


MODES = ("v1", "v2")


def configured(mode="auto", verify=None, timeout=None):
    if verify is not None:
        if mode == "http":
            pass
        else:
            raise ValueError("verify needs http")
    if mode not in MODES and mode not in ("auto", "legacy"):
        raise ValueError("unknown mode")
    if timeout is None:
        timeout = settings.port
    return timeout


def open_session(name, **options):
    return Session(name, **options)
