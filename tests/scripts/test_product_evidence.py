"""The unstructured baseline reads immutable originals and excludes working-tree additions."""
import asyncio
import subprocess

from fastmcp import Client
from product_evidence import Evidence, server


def test_original_blob_and_mcp_read(tmp_path):
    def git(*args):
        return subprocess.run(["git", "-C", str(tmp_path), *args], check=True,
                              capture_output=True, text=True).stdout.strip()
    git("init")
    (tmp_path / "original.py").write_text('def register_tool(name):\n    """Register a tool."""\n    pass\n')
    (tmp_path / "guide.md").write_text("Resources may contain text.\n")
    git("add", "original.py", "guide.md")
    git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit", "-m", "original evidence")
    commit = git("rev-parse", "HEAD")
    (tmp_path / "original.py").write_text("changed after pin\n")
    (tmp_path / "secret.md").write_text("untracked derived answers\n")
    evidence = Evidence(tmp_path, commit)
    assert "secret.md" not in evidence.inventory()["files"]
    assert "register_tool" in evidence.read("original.py")["text"]
    assert evidence.search("register tool")["results"][0]["path"] == "original.py"

    async def call():
        async with Client(server(evidence)) as client:
            assert {t.name for t in await client.list_tools()} == {"search_original", "read_original"}
            result = await client.call_tool("read_original", {"path": "original.py"})
            assert result.structured_content["commit"] == commit
    asyncio.run(call())
