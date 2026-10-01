def source():
    return [1, 2]


def element(x):
    return x


def test(x):
    return bool(x)


generator = (element(x) for x in source() if test(x))
multiple = (element(y) for x in source() for y in source() if test(y))
list_values = [element(x) for x in source() if test(x)]
set_values = {element(x) for x in source() if test(x)}
dict_values = {element(x): element(x) for x in source() if test(x)}
