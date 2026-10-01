"""Shadowing never erases real nested reflection or an uncertain callee; never executed."""
import builtins


def parameter(getattr, obj, name):
    return getattr(obj, name)


def reassigned(obj, name, replacement):
    getattr = lambda item, field: 0
    result = getattr(obj, name)
    getattr = replacement
    return result


def nested(obj, name):
    getattr = lambda item, field: builtins.getattr(item, field)
    return getattr(obj, name)
