"""Reference solution for held-out task T06."""

from dataclasses import dataclass

from fastmcp import FastMCP

mcp = FastMCP("Users")


@dataclass
class UserProfile:
    name: str
    age: int
    email: str


@mcp.tool
def get_profile(user_id: str) -> UserProfile:
    """Get a user's profile."""
    return UserProfile(name="Alice", age=30, email="alice@example.com")


@mcp.tool
def count_users() -> int:
    """Count registered users."""
    return 3
