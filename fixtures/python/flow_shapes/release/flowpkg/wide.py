def wide(
    a00, a01, a02, a03, a04, a05, a06, a07, a08,
    a09, a10, a11, a12, a13, a14, a15, a16,
):
    if (
        a00 or a01 or a02 or a03 or a04 or a05 or a06 or a07 or a08
        or a09 or a10 or a11 or a12 or a13 or a14 or a15 or a16
    ):
        return "reachable"
    return "fallback"
