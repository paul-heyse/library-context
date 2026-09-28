def run(value: str = "ok") -> str:
    if value == "bad":
        raise ValueError(value)
    return value


def unseeded(value: str) -> str:
    return value
