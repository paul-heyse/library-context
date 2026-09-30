@dec
def f(a, b=1, *, c: int = 2):
    """Doc."""
    import os.path as p
    from . import x
__all__ = ["f"]
class C:
    y: int = 3
