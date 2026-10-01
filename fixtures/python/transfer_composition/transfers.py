def identity(value):
    return value


def mutate(value, target):
    target.field = value
    return target


def variadic(value, *rest, **named):
    return value


def guarded(timeout):
    if timeout is None:
        return 0
    return timeout


def rebound(value):
    value = object()
    return value


def run(seed, target):
    identity(seed)
    mutate(seed, target)
    variadic(seed, target, key=target)
    guarded(seed)
    rebound(seed)

def defaulted(value, target=None):
    return value


class ReceiverCase:
    def update(self, value):
        self.field = value
        return value

    @classmethod
    def update_class(cls, value):
        return value


def receivers(seed, instance: ReceiverCase):
    instance.update(seed)
    ReceiverCase.update(instance, seed)
    ReceiverCase.update_class(seed)
    defaulted(seed)


def fresh_source():
    def fresh():
        return None
    return fresh()


def fresh_unproven_body():
    def fresh_effect():
        return object()
    return fresh_effect()


def fresh_default_header():
    def fresh_default(value=None):
        return value
    return fresh_default()


def fresh_async_header():
    async def fresh_async():
        return None
    return fresh_async()
