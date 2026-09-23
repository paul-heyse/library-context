"""Statically decided branches (H1 C1): each clause marked as Pyrefly decides it."""

import os
import sys
import typing
from typing import TYPE_CHECKING


class Cfg:
    version_info_cache = (3, 9)


cfg = Cfg()

if not TYPE_CHECKING:
    runtime_only = 1

if typing.TYPE_CHECKING:
    checked = 1

if os.name == "nt":
    windows = 1
else:
    posix = 1

if cfg.version_info_cache:  # an ordinary attribute: not a static test
    cached = 1
elif sys.version_info < (3, 9):
    old_python = 1
else:
    new_python = 1

if sys.version_info >= (3, 10):
    modern = 1
elif sys.platform == "win32":
    win_old = 1
else:
    other = 1

if sys.version_info >= (3, 10) and sys.platform == "linux":
    mixed = 1

if False:
    never = 1

# Nested: a clause inside a pruned clause is pruned, whatever its own test decides (H1 review F1).
if not TYPE_CHECKING:
    if sys.version_info >= (3, 10):
        nested_a = 1

if TYPE_CHECKING:
    nested_b = 1
else:
    if os.name == "posix":
        nested_d = 1
    if True:
        nested_e = 1

if sys.version_info < (3, 9):
    if TYPE_CHECKING:
        nested_f = 1

if sys.version_info >= (3, 10):
    if TYPE_CHECKING:
        nested_g = 1
