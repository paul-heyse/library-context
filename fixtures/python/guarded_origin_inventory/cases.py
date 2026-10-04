def simple_join(value, flag):
    if flag:
        value = 0
    return value

def repeated_test(value, flag):
    if flag:
        value = 0
    if not flag:
        return value
    return 0

def expression_choice(value, flag):
    if flag:
        value = 0
    return value if not flag else 0

def early_return(value, flag):
    if flag:
        value = 0
        return value
    return value

def boolean_choice(value, flag):
    if flag:
        value = 0
    return not flag and value

def branch_local(value, flag):
    if flag:
        return value
    value = 0
    return value

def loop_carried(value, items):
    for item in items:
        value = item
    return value

def deleted(value, flag):
    if flag:
        del value
    return value
