def pong(n):
    from shapes.ping import ping

    return ping(n - 1) if n else 0
