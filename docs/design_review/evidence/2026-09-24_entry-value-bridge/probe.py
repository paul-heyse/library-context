def direct(mode):
    if mode == "sse":
        return 1
    return 0


def rebound(mode):
    mode = "http"
    if mode == "sse":
        return 1
    return 0


def after_call(mode, callback):
    if mode == "sse":
        callback()
        if mode == "http":
            return 1
    return 0


def changed_closure(mode):
    def change():
        nonlocal mode
        mode = "http"

    if mode == "sse":
        change()
        if mode == "http":
            return 1
    return 0
