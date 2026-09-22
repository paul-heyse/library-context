from cycle import a


def right(n):
    if n:
        return a.left(n - 1)
    return ""


def use_right():
    return right(3).upper()


FLAG = True
Y = a.X if FLAG else ""


def use_y():
    return Y.upper()
