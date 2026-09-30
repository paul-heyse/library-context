from typing import TypedDict, Unpack

Keys = TypedDict("Keys", {"": int, " ": int, "not-an-identifier": str}, total=False)

def accept(**kwargs: Unpack[Keys]) -> None: ...

accept(**{"": 1, " ": 2, "not-an-identifier": "value"})

