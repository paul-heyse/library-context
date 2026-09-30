from typing import Literal, TypedDict

NUL: Literal["\0"] = "\0"
UNICODE = "é\0終"
KEYS = {"\0": 1}
NulKeys = TypedDict("NulKeys", {"\0": int}, total=False)

def identity(value: str) -> str:
    if value == "\0":
        return "é\0終"
    return value
