"""Normal-return summary admission and controlling-frame boundaries."""


def plain_identity(value):
    """Return the supplied value unchanged."""
    return value


def nested_identity(value, enabled):
    if enabled:
        return value
    return None


def finally_identity(value):
    try:
        return value
    finally:
        marker = 1


def finally_pass_identity(value):
    try:
        return value
    finally:
        pass


def nested_finally_pass_identity(value):
    try:
        try:
            return value
        finally:
            pass
    finally:
        pass


def multi_pass_finally_identity(value):
    try:
        return value
    finally:
        pass
        pass


def pass_then_effect_finally(value):
    try:
        return value
    finally:
        pass
        marker = 1


def nested_effectful_finalizer(value):
    try:
        try:
            return value
        finally:
            pass
    finally:
        marker = 1


def with_identity(value, manager):
    with manager:
        return value


def recursive_before_return(value):
    recursive_before_return(value)
    return value


def recursive_base_identity(value, stop):
    if stop:
        return value
    return recursive_base_identity(value, True)


def recursive_false_control(value, stop):
    if stop:
        return value
    return recursive_false_control(value, False)


def recursive_keyword_true(value, stop):
    if stop:
        return value
    return recursive_keyword_true(value, stop=True)


def recursive_keyword_false(value, stop):
    if stop:
        return value
    return recursive_keyword_false(value, stop=False)


def recursive_all_keyword_true(value, stop):
    if stop:
        return value
    return recursive_all_keyword_true(value=value, stop=True)


def recursive_all_keyword_false(value, stop):
    if stop:
        return value
    return recursive_all_keyword_false(value=value, stop=False)


def recursive_reversed_keyword_true(value, stop):
    if stop:
        return value
    return recursive_reversed_keyword_true(stop=True, value=value)


def recursive_reversed_keyword_false(value, stop):
    if stop:
        return value
    return recursive_reversed_keyword_false(stop=False, value=value)


def keyword_local_wrapper(value):
    return plain_identity(value=value)


def unpacked_local_wrapper(value):
    return plain_identity(**{"value": value})


def guarded_symbolic_recursive(value, stop):
    if stop:
        return guarded_symbolic_base(value, stop)
    return None


def guarded_symbolic_base(value, stop):
    if stop:
        return value
    return guarded_symbolic_recursive(value, stop)


def conditional_self_recursive(value):
    if value:
        return value
    return conditional_self_recursive(value)


def prior_call_identity(value):
    plain_identity(value)
    return value


def alternate_branch_identity(value, invoke):
    if invoke:
        plain_identity(value)
    else:
        return value
    return None


__all__ = ["plain_identity", "nested_identity", "finally_identity", "finally_pass_identity", "nested_finally_pass_identity", "multi_pass_finally_identity", "pass_then_effect_finally", "with_identity", "recursive_before_return", "recursive_base_identity", "recursive_false_control", "recursive_keyword_true", "recursive_keyword_false", "recursive_all_keyword_true", "recursive_all_keyword_false", "recursive_reversed_keyword_true", "recursive_reversed_keyword_false", "keyword_local_wrapper", "unpacked_local_wrapper", "guarded_symbolic_recursive", "guarded_symbolic_base", "conditional_self_recursive", "prior_call_identity", "alternate_branch_identity"]
