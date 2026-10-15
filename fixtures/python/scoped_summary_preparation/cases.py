"""Stable guarded transfers plus an explicitly unsupported truthiness guard."""


def guarded_pair_relay(value, other, gate):
    def fresh(item, alternative, flag):
        if flag is None:
            return item
        return alternative

    return fresh(value, other, gate)


def unsupported_truthy_pair_relay(value, other, gate):
    def truthy_fresh(item, alternative, flag):
        if flag:
            return item
        return alternative

    return truthy_fresh(value, other, gate)
