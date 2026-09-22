"""A return-type inference cycle across two modules (review F6): `left` and `right` are
unannotated and call each other, and each result is a method receiver, so the solved types
reach `pysa_calls.receiver_class`."""

from cycle import b


def left(n):
    if n:
        return b.right(n - 1)
    return 0


def use_left():
    return left(3).bit_length()


X = b.Y if b.FLAG else 1


def use_x():
    return X.bit_length()
