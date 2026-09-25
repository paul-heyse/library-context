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


def known_first(path):
    try:
        return open(path)
    except OSError:
        return None
    except Exception:
        return None


def unknown_first(path, Unknown):
    try:
        return open(path)
    except Unknown:
        return None
    except OSError:
        return None


def indirect_call_result(path):
    value = open(path)
    return value


def computed_handler(path):
    try:
        return open(path)
    except OSError:
        return len(path)


def two_action_handler(path):
    try:
        return open(path)
    except OSError:
        marker = 1
        return None


def with_intervening(path, manager):
    try:
        with manager:
            return open(path)
    except OSError:
        return None


def finalizer_changes_return(path):
    try:
        return open(path)
    except OSError:
        return None
    finally:
        return "override"
