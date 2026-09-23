"""Reference solution for held-out task T11."""

from fastmcp import FastMCP
from fastmcp.exceptions import ResourceError

mcp = FastMCP("Weather")

TEMPERATURE = {"c": 22, "f": 71.6}


@mcp.resource("weather://{city}/current{?units}", mime_type="application/json")
def current_weather(city: str, units: str = "c") -> dict:
    """Current weather for a city, in Celsius (c) or Fahrenheit (f)."""
    if units not in TEMPERATURE:
        raise ResourceError("units must be 'c' or 'f'")
    return {"city": city, "temperature": TEMPERATURE[units], "units": units}
