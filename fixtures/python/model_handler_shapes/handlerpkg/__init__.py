"""Potential modeled exceptions nested under authored handler clauses."""

def exact_handler(path):
    try:
        return open(path)
    except OSError:
        return None


def broader_handler(path):
    try:
        return open(path)
    except Exception:
        return None


def bare_handler(path):
    try:
        return open(path)
    except:
        return None


def shadowed_handler(path, OSError):
    try:
        return open(path)
    except OSError:
        return None


def outside_handler(path):
    result = open(path)
    try:
        return result
    except OSError:
        return None


def nested_function(path):
    try:
        def inner():
            return open(path)
        return inner()
    except OSError:
        return None


def nested_tries(path):
    try:
        try:
            return open(path)
        except TypeError:
            return None
    except OSError:
        return None
