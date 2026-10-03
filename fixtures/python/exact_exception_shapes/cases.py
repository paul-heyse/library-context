"""Independent Python-ordered exception expectations for the finite runtime model."""
def broad_first():
    try:
        raise ValueError("value")
    except Exception:
        return 1
    except ValueError:
        return 2

def tuple_match():
    try:
        raise ValueError()
    except (RuntimeError, ValueError):
        return 3

def unmatched():
    try:
        raise ValueError()
    except RuntimeError:
        return 4

def final_return():
    try:
        raise ValueError()
    finally:
        return 5

def final_raise():
    try:
        raise ValueError()
    finally:
        raise RuntimeError("replacement")

def reraised():
    try:
        raise ValueError()
    except Exception:
        raise

def named_disposal():
    try:
        raise TypeError()
    except Exception as error:
        return None

def named_retained(error):
    try:
        raise TypeError()
    except Exception as error:
        return None

def normal_else():
    try:
        pass
    except Exception:
        raise ValueError()
    else:
        raise RuntimeError()

def no_else_after_handler():
    try:
        raise ValueError()
    except Exception:
        pass
    else:
        raise RuntimeError()

def dynamic_constructor(factory):
    raise factory()

def unsupported_group():
    try:
        raise ValueError()
    except* Exception:
        pass

def argument_failure():
    def first():
        raise ValueError()
    raise ValueError(first())

def argument_opaque(action):
    raise ValueError(action())

def shadowed_constructor(ValueError):
    raise ValueError()

def invalid_handler():
    try:
        raise ValueError()
    except int:
        pass

def explicit_cause():
    raise ValueError() from RuntimeError()

def invalid_cause():
    raise ValueError() from 42

def argument_order(action):
    def first():
        raise ValueError()
    raise ValueError(first(), action())

def normal_plain():
    return None

def bare_class():
    raise ValueError

def handler_lookup_failure():
    def lookup():
        raise RuntimeError()
    try:
        raise ValueError()
    except lookup():
        pass

def starred_constructor(arguments):
    raise ValueError(*arguments)
