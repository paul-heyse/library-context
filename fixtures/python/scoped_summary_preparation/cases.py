"""One checked call with two guarded value ports and deliberately disjoint pairs."""


def guarded_pair_relay(value, other, gate):
    def fresh(item, alternative, flag):
        if flag:
            return item
        return alternative

    return fresh(value, other, gate)
