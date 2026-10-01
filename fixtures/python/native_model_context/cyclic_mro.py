class Broken(Base):
    def __init__(self):
        pass

    def ping(self, value):
        return value

    def __enter__(self):
        return None

    def __exit__(self, kind, value, traceback):
        return False

class Base(Broken):
    pass
