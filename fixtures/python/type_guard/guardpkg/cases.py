def exact(x):
    if type(x) is str:
        return x
    return None


def shadowed_type(type, x):
    if type(x) is str:
        return x
    return None


def shadowed_class(str, x):
    if type(x) is str:
        return x
    return None


def after_call(x):
    observer()
    if type(x) is str:
        return x
    return None


def nested(x):
    if type(x) is str:
        if x == "sse":
            return x
    return None


def nested_after_call(x):
    if type(x) is str:
        observer()
        if x == "sse":
            return x
    return None
