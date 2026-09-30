from helper import outside


def a():
    b()
    c()
    b()
    outside()


def b():
    d()


def c():
    d()


def d():
    a()


def isolated():
    return 0


def recursive():
    recursive()


def unresolved(callback):
    callback()
