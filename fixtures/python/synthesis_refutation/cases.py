"""Native input for the exact finite negative Summary question; never executed."""
__all__ = ["recursive_false_control"]


def recursive_false_control(value, stop):
    """Return the input value when the stop condition is true."""
    if stop:
        return value
    return recursive_false_control(value, False)
