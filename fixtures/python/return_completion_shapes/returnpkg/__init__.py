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


def with_identity(value, manager):
    with manager:
        return value


__all__ = ["plain_identity", "nested_identity", "finally_identity", "finally_pass_identity", "nested_finally_pass_identity", "with_identity"]
