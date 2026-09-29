from contextlib import nullcontext


def choose(α: int, fallback: int = 3) -> int:
    with nullcontext(α) as first, nullcontext(fallback) as second:
        return first if first else second


answer = choose(7)
