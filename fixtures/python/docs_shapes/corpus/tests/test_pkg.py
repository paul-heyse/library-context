"""A usage module (C5b): the library as its users call it, through the installed package."""

from pkg import Server, make_server


def test_server() -> None:
    server = make_server("x")
    server.tool(print)
    assert isinstance(server, Server)
