def effect():
    return None

def read(value):
    return value

def expressions():
    a = False and effect()
    b = True or effect()
    c = 3 if True else effect()
    d = 1 + 2
    e = -2
    f = not ()
    g = "hello"
    h = 999999999999999999999999999999999999999 + 1
    i = True and effect()
    return a

def finally_returns():
    try:
        raise None
    finally:
        return 1

def finally_raises():
    try:
        return 1
    finally:
        raise None

def bare_handler():
    try:
        raise None
    except:
        raise

def handler_name_cleanup():
    try:
        raise None
    except TypeError as error:
        return None


def complex_value():
    return 1j


def body_pass():
    pass


def body_stops():
    return 1
    effect()


def field_read(value):
    return value.field
