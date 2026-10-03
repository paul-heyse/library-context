GLOBAL = 7

def stable(value: int) -> int:
    local = 3
    def inner() -> int:
        return value + local + GLOBAL
    return inner()

def late() -> int:
    def reader() -> int:
        return cell
    cell = 4
    return reader()

def mutable() -> int:
    cell = 1
    def writer() -> None:
        nonlocal cell
        cell = 2
    def reader() -> int:
        return cell
    writer()
    return reader()
