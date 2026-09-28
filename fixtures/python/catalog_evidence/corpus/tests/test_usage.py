import pytest as pt
from pr3pkg import run


def test_mixed(fixture):
    run("ok")
    with pt.raises(ValueError):
        run("bad")
    assert run(fixture) == fixture


@pt.mark.xfail(reason="example")
def test_expected():
    run("bad")
