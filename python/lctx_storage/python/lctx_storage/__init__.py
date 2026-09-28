"""Explicit, lifespan-owned Rust PostgreSQL services; importing does not connect."""

from ._storage import Repository, StorageError, open_repository

__all__ = ["Repository", "StorageError", "open_repository"]
