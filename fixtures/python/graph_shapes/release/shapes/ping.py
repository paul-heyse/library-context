def ping(n):
    # Mutual recursion across two files.
    from shapes.pong import pong

    return pong(n - 1) if n else 0
