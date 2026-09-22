class A:
    class Config:
        def m(self) -> int:
            return 1


class B:
    class Config:
        def m(self) -> int:
            return 2


def use(a: A.Config, b: B.Config) -> int:
    return a.m() + b.m()
