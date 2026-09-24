"""The increment-2 review's F4 shape: a subclass whose public path sorts before its base's, so a
consumer that named each method by its least path would name `Server`'s methods through it."""

from pkg.server import Server


class Alpha(Server):
    """A server by another name."""
