import pytest as pt
import unittest as ut
from pr3pkg import run


def test_header():
    with pt.raises(ValueError, match=run("header")):
        run("protected")


def test_deferred():
    with pt.raises(ValueError):
        def later():
            return run("deferred")
    later()


class TestUsage(ut.TestCase):
    def test_negative(self):
        with self.assertRaises(ValueError):
            run("unit-bad")


def test_generator():
    with pt.raises(ValueError):
        pending = (run("generator-deferred") for item in run("generator-eager") if run("generator-filter"))
    list(pending)


class TestOverride(ut.TestCase):
    def assertRaises(self, exception):
        return ordinary_manager()

    def test_ordinary(self):
        with self.assertRaises(ValueError):
            run("overridden")
