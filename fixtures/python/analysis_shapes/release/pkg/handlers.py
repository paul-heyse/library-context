from pkg.helpers import finish


class Handler:
    def handle(self):
        return finish(1)


class Special(Handler):
    def handle(self):
        return 2


def run(h: Handler):
    # An override-open dispatch: Pysa reports `Overrides(Handler.handle)`, a candidate.
    return h.handle()
