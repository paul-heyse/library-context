class Left:
    pass

class Right:
    pass

class Forward(Left, Right):
    pass

class Reverse(Right, Left):
    pass

class Broken(Forward, Reverse):
    def __init__(self):
        pass

    def ping(self, value):
        return value

    def __enter__(self):
        return None

    def __exit__(self, kind, value, traceback):
        return False
