from typing import Callable, overload


class Base:
    def méthode(self, x: int) -> int:
        return x


class Écrivain(Base):
    """Écrit des données — “quoted”."""

    def __new__(cls, *args: object, **kwargs: object) -> "Écrivain":
        return super().__new__(cls)

    def __init__(self, nom: str = "défaut") -> None:
        self.nom = nom

    @property
    def étiquette(self) -> str:
        return f"«{self.nom}»"

    def méthode(self, x: int) -> int:
        return super().méthode(x) + 1


@overload
def build(x: int) -> Écrivain: ...
@overload
def build(x: str) -> Écrivain: ...
def build(x: int | str) -> Écrivain:
    w = Écrivain("naïve")
    s = w.étiquette
    print(s)
    return w


def λ_helper(f: Callable[[int], int], v: int) -> int:
    return f(v)


def utilise() -> str:
    r = λ_helper(lambda z: z * 2, 3)
    e = Écrivain()
    n = e.méthode(r)
    return f"résultat={n} {e!r} {'é' * 2}"
