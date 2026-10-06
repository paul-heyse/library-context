"""A separate owner domain in the same immutable input."""


def independent_argument():
    def fresh(item):
        return item
    fresh(13)
    return 4
