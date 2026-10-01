from depbase import Base


class Child(Base):
    def __init__(self):
        pass

    def ping(self, value):
        return value

    def __enter__(self):
        return self

    def __exit__(self, exception_type, exception, traceback):
        return False
