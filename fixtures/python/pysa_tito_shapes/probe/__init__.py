"""Independent Pysa TITO control and counter-control; never executed by the compiler."""


def source() -> str:
    return "source is modeled by Pysa"


def sink(value: str) -> None:
    pass


def identity(value: str) -> str:
    return value


def constant(value: str) -> str:
    return "fixed"


def wrapper(value: str) -> str:
    return identity(value)


def exercise() -> None:
    sink(identity(source()))
    sink(constant(source()))
    sink(wrapper(source()))
