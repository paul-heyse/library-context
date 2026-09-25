"""Compose source discovery, mdBook, Pagefind and offline links; stdlib only."""

from __future__ import annotations

import argparse
import html
import json
import posixpath
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import quote, unquote, urlsplit

import design_sections as sections
from repo_paths import local_skill

ROOT = Path(__file__).resolve().parents[1]
SCOPES = {"Current", "Reference", "History"}


@dataclass(frozen=True)
class Page:
    path: Path
    title: str
    body: str
    collection: str
    scope: str
    status: str = ""


def config(root: Path) -> dict:
    return tomllib.loads((root / "docs/site.toml").read_text())


def run(argv: list[str], **kwargs) -> subprocess.CompletedProcess:
    return subprocess.run(argv, check=True, text=True, **kwargs)


def checked(root: Path, path: str | Path) -> Path:
    full = root / path
    if not full.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"path escapes repository: {path}")
    return full


def metadata(text: str) -> tuple[dict[str, str], str]:
    values = {}
    if text.startswith("---\n"):
        end = text.find("\n---\n", 4)
        if end < 0:
            raise ValueError("unterminated frontmatter")
        for key, value in re.findall(r"^(title|status):\s*(.*)$", text[4:end], re.M):
            if value.strip() not in {"|", ">"}:
                values[key] = value.strip().strip("\"'")
        text = text[end + 5 :]
    return values, text


def active_standard(root: Path) -> set[Path]:
    base = root / "docs/design_review/design_principles"
    manifest = tomllib.loads((base / "standard.toml").read_text())
    names = [manifest["core"][key] for key in ("principles", "template")]
    names += [manifest["binding"]["path"]]
    names += [profile[key] for profile in manifest["profiles"] for key in ("principles", "review")]
    return {(base / name).relative_to(root) for name in names}


def discover(root: Path, settings: dict) -> list[Page]:
    publication = settings["publication"]
    for asset in publication["assets"]:
        if not checked(root, asset).is_file():
            raise ValueError(f"missing declared asset: {asset}")
    excluded = set(publication["exclude"])
    current = set(publication["entrypoints"] + publication["current_work"])
    standard = active_standard(root)
    seen = set()
    pages = []
    for collection in settings["collections"]:
        paths = set()
        for pattern in collection["include"]:
            checked(root, pattern)
            matches = {p for p in root.glob(pattern) if p.is_file()}
            if not matches:
                raise ValueError(f"empty publication selection: {pattern}")
            paths.update(matches)
        for path in sorted(paths, key=lambda p: (p.name != "README.md", p.as_posix())):
            relative = path.relative_to(root)
            checked(root, relative)
            if relative.as_posix() in excluded:
                continue
            if relative in seen:
                raise ValueError(f"duplicate publication: {relative}")
            seen.add(relative)
            front, body = metadata(path.read_text())
            title = settings.get("titles", {}).get(relative.as_posix()) or front.get("title")
            title = title or next(
                (title for _, level, title in sections.headings(body) if level == 1), ""
            )
            if not title:
                raise ValueError(f"missing title/H1: {relative}")
            scope = collection["scope"]
            status = front.get("status", "")
            if relative.parent == Path("docs/adr") and re.match(r"\d{4}-", relative.name):
                if status not in {"accepted", "proposed", "superseded", "rejected"}:
                    raise ValueError(f"invalid ADR status: {relative}: {status}")
                scope = {"accepted": "Current", "proposed": "Reference"}.get(status, "History")
            if relative in standard or relative.as_posix() in current:
                scope = "Current"
            if scope not in SCOPES:
                raise ValueError(f"invalid scope: {scope}")
            pages.append(Page(relative, title, body, collection["title"], scope, status))
    for path in current | set(settings.get("titles", {})):
        if Path(path) not in seen:
            raise ValueError(f"configured page is not published: {path}")
    if standard - seen:
        raise ValueError(f"active standard is not published: {standard - seen}")
    return pages


def tracked_files(root: Path) -> set[str]:
    return set(
        run(
            ["git", "ls-tree", "-r", "--name-only", "-z", "HEAD"], cwd=root, capture_output=True
        ).stdout.split("\0")
    )


def section_directory(root: Path, pages: list[Page]) -> tuple[Page, dict[Path, dict[str, str]]]:
    annotations: dict[Path, dict[str, str]] = {}
    rows = ["# Architecture sections", "", "Generated from the authoritative section owners.", ""]
    published = {p.path for p in pages}
    for ident, section in sections.resolve(root).items():
        path = section.path.relative_to(root)
        if path not in published:
            raise ValueError(f"section owner not published: {path}")
        annotations.setdefault(path, {})[ident] = sections.anchor(ident)
        rows.append(f"- [{section.title}]({path.as_posix()}#{sections.anchor(ident)})")
    return Page(
        Path("architecture-sections.md"),
        "Architecture sections",
        "\n".join(rows),
        "Architecture",
        "Current",
    ), annotations


def stage(root: Path, candidate: Path, pages: list[Page], settings: dict) -> list[Page]:
    directory, annotations = section_directory(root, pages)
    pages = [*pages, directory]
    src = candidate / "src"
    src.mkdir()
    summary = ["# Summary", ""]
    collection = ""
    for page in pages:
        body = page.body
        if page.path in annotations:
            lines = body.splitlines()
            for line, _, title in reversed(sections.headings(body)):
                ident = sections.IDENT.match(title)
                if not ident or ident[1] not in annotations[page.path]:
                    continue
                anchor = annotations[page.path][ident[1]]
                if not any(
                    f'id="{anchor}"' in previous for previous in lines[max(0, line - 2) : line]
                ):
                    lines.insert(line, f'<a id="{anchor}"></a>\n')
            body = "\n".join(lines) + "\n"
        destination = src / page.path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(body)
        if page.collection != collection:
            collection = page.collection
            summary += ["", f"# {collection}", ""]
        title = re.sub(r"[\[\]`*]", "", page.title)
        summary.append(f"- [{title}]({page.path.as_posix()})")
    (src / "SUMMARY.md").write_text("\n".join(summary) + "\n")
    book = (root / "docs/book.toml").read_text()
    site = settings["site"]
    additions = {
        "site-url": site["base_url"],
        "git-repository-url": site["repository"],
        "edit-url-template": f"{site['repository']}/edit/{site['branch']}/{{path}}",
    }
    book = book.replace(
        "[output.html]",
        "[output.html]\n"
        + "\n".join(f"{key} = {json.dumps(value)}" for key, value in additions.items()),
    )
    (candidate / "book.toml").write_text(book)
    return pages


class Links(HTMLParser):
    """Locate real HTML URL attributes; escaped code examples are never links."""

    def __init__(self, text: str):
        super().__init__(convert_charrefs=False)
        self.tags: list[tuple[int, str, list[tuple[str, str | None]]]] = []
        self.offsets = [0]
        for line in text.splitlines(keepends=True):
            self.offsets.append(self.offsets[-1] + len(line))
        self.feed(text)

    def handle_starttag(self, tag, attrs):
        if tag in {"a", "img", "source", "script", "link", "iframe"}:
            line, col = self.getpos()
            self.tags.append((self.offsets[line - 1] + col, self.get_starttag_text() or "", attrs))

    handle_startendtag = handle_starttag


def rewrite_links(
    root: Path,
    site: Path,
    page: Page,
    text: str,
    published: set[Path],
    tracked: set[str],
    settings: dict,
    revision: str,
) -> str:
    for offset, tag, attrs in reversed(Links(text).tags):
        original_length = len(tag)
        for attr, value in attrs:
            if attr not in {"href", "src"} or not value:
                continue
            url = urlsplit(value)
            if url.scheme or url.netloc or not url.path:
                continue
            relative = Path(
                posixpath.normpath(posixpath.join(page.path.parent.as_posix(), unquote(url.path)))
            )
            checked(root, relative)
            # Renderer files and already published paths need no source transport.
            if relative in published or (site / relative).is_file():
                continue
            source = relative
            if source.suffix == ".html" and source.with_suffix(".md") in {
                p.with_suffix(".md") for p in published
            }:
                continue
            if source.suffix == ".html" and (root / source.with_suffix(".md")).is_file():
                source = source.with_suffix(".md")
            original = checked(root, source)
            if local_skill(root, original):
                tag = re.sub(r"\s" + attr + r'="[^"]*"', "", tag, count=1)
                tag = (
                    tag[:-1]
                    + ' title="Local capability reference; available in the operator checkout" '
                    'class="local-reference">'
                )
                if attr == "href":
                    tag += '<span class="local-reference-label">[local reference] </span>'
                continue
            exists_in_git = source.as_posix() in tracked or any(
                name.startswith(source.as_posix().rstrip("/") + "/") for name in tracked
            )
            if not original.exists():
                raise ValueError(
                    f"{page.path}: unresolved publication/source target: {value} ({source})"
                )
            if source.as_posix() in settings["publication"]["assets"] or (
                attr == "src"
                and source.suffix.lower() in {".png", ".jpg", ".jpeg", ".svg", ".gif", ".webp"}
                and "evidence" not in source.parts
            ):
                destination = checked(site, source)
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(original, destination)
                continue
            if not exists_in_git:
                tag = re.sub(r"\s" + attr + r'="[^"]*"', "", tag, count=1)
                tag = (
                    tag[:-1] + ' title="Uncommitted local source; absent at build revision" '
                    'class="local-reference">'
                )
                if attr == "href":
                    tag += '<span class="local-reference-label">[uncommitted local source] </span>'
                continue
            # An omitted source is a reference, never an attempted HTML chapter.
            kind = "tree" if original.is_dir() else "blob"
            new = f"{settings['site']['repository']}/{kind}/{revision}/{quote(source.as_posix())}"
            if url.fragment:
                new += "#" + url.fragment
            tag = re.sub(
                r"(" + attr + r'=")[^"]*(")',
                lambda m, target=new: m[1] + html.escape(target, quote=True) + m[2],
                tag,
                count=1,
            )
        text = text[:offset] + tag + text[offset + original_length :]
    return text


def annotate(root: Path, site: Path, pages: list[Page], settings: dict) -> None:
    revision = run(["git", "rev-parse", "HEAD"], cwd=root, capture_output=True).stdout.strip()
    dirty = bool(run(["git", "status", "--porcelain"], cwd=root, capture_output=True).stdout)
    tracked = tracked_files(root)
    published = {p.path.with_suffix(".html") for p in pages}
    errors = []
    for page in pages:
        path = site / page.path.with_suffix(".html")
        text = path.read_text()
        try:
            text = rewrite_links(root, site, page, text, published, tracked, settings, revision)
        except ValueError as error:
            errors.append(str(error))
            continue
        prefix = posixpath.relpath(".", page.path.parent.as_posix())
        text = text.replace(
            "<main>",
            f'<main data-pagefind-body data-pagefind-filter="scope:{page.scope}" '
            f'data-doc-title="{html.escape(page.title, quote=True)}" '
            f'data-doc-kind="{html.escape(page.collection, quote=True)}" '
            f'data-doc-status="{html.escape(page.status, quote=True)}" '
            'data-pagefind-meta="title[data-doc-title],kind[data-doc-kind],status[data-doc-status]">',
            1,
        )
        banner = (
            f'<aside class="build-context" data-pagefind-ignore>Source revision {revision[:12]}'
        )
        banner += " · dirty local preview; repository links show committed files" if dirty else ""
        banner += (
            f" · {page.scope}"
            + (f" · {html.escape(page.status)}" if page.status else "")
            + "</aside>"
        )
        text = text.replace("</main>", banner + "</main>", 1)
        assets = (
            f'<link rel="stylesheet" href="{prefix}/pagefind/pagefind-component-ui.css">'
            f'<link rel="stylesheet" href="{prefix}/theme/search.css">'
            f'<script type="module" src="{prefix}/theme/search.js"></script>'
        )
        text = text.replace("</head>", assets + "</head>")
        path.write_text(text)
    if errors:
        raise ValueError("\n".join(errors))
    for path in site.rglob("*.html"):
        if path.relative_to(site) not in published:
            # mdBook's landing/print/404 are presentation aliases, never indexed.
            text = path.read_text().replace("<html ", '<html data-pagefind-ignore="all" ', 1)
            path.write_text(text)
    # Landing is the same useful entry page, excluded as an alias from indexing.
    landing = (
        (site / "README.html")
        .read_text()
        .replace("data-pagefind-body", 'data-pagefind-ignore="all"')
    )
    (site / "index.html").write_text(landing)
    shutil.copytree(root / "docs/theme", site / "theme", dirs_exist_ok=True)


def check_tools(settings: dict) -> None:
    for tool, version in settings["tools"].items():
        try:
            output = run([tool, "--version"], capture_output=True).stdout.strip()
        except FileNotFoundError as error:
            raise ValueError(f"missing {tool}; run just bootstrap-docs") from error
        if not re.search(r"(?<![\d.])v?" + re.escape(version) + r"(?![\d.])", output):
            raise ValueError(f"{tool}: expected {version}, got {output}; run just bootstrap-docs")


def replace_site(candidate: Path, destination: Path) -> None:
    backup = destination.with_name("previous")
    if backup.exists():
        shutil.rmtree(backup)
    if destination.exists():
        destination.rename(backup)
    try:
        candidate.rename(destination)
    except OSError:
        if backup.exists():
            backup.rename(destination)
        raise
    if backup.exists():
        shutil.rmtree(backup)


def build(root: Path, settings: dict) -> Path:
    base = settings["site"]["base_url"]
    if (
        not base.startswith("/")
        or not base.endswith("/")
        or ".." in base.split("/")
        or urlsplit(base).netloc
    ):
        raise ValueError("site base_url must be an absolute URL path ending in / without ..")
    check_tools(settings)
    pages = discover(root, settings)
    work = root / "build/docs"
    work.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="candidate-", dir=work) as temporary:
        candidate = Path(temporary)
        pages = stage(root, candidate, pages, settings)
        site = candidate / "site"
        run(["mdbook", "build", str(candidate), "--dest-dir", str(site)])
        annotate(root, site, pages, settings)
        run(["pagefind", "--site", str(site), "--output-subdir", "pagefind"])
        link_root = site
        if base != "/":
            link_root = candidate / "link-root"
            mount = checked(link_root, base.lstrip("/"))
            mount.parent.mkdir(parents=True, exist_ok=True)
            mount.symlink_to(site, target_is_directory=True)
        run(
            [
                "lychee",
                "--offline",
                "--include-fragments=anchor-only",
                "--root-dir",
                str(link_root.resolve()),
                "--no-progress",
                str(site),
            ]
        )
        replace_site(site, work / "site")
    print(f"docs: passed; {len(pages)} canonical pages; {work / 'site'}")
    return work / "site"


def pytest_version(root: Path) -> str:
    lock = tomllib.loads((root / "uv.lock").read_text())
    return next(package["version"] for package in lock["package"] if package["name"] == "pytest")


def isolated(root: Path, *args: str, online: bool = False) -> list[str]:
    command = [
        "uv",
        "run",
        "--no-project",
        "--python",
        (root / ".python-version").read_text().strip(),
    ]
    if not online:
        command += ["--offline", "--no-python-downloads"]
    return [*command, *args]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command", choices=["build", "check", "test", "serve", "bootstrap", "doctor"]
    )
    parser.add_argument("--port", type=int, default=8000)
    parser.add_argument("--base-url")
    args = parser.parse_args()
    settings = config(ROOT)
    if args.base_url:
        settings["site"]["base_url"] = args.base_url
    if args.command == "bootstrap":
        for tool, version in settings["tools"].items():
            run(
                [
                    "cargo-binstall",
                    "--no-confirm",
                    "--disable-strategies",
                    "compile",
                    "--version",
                    version,
                    tool,
                ]
            )
        run(
            isolated(
                ROOT,
                "--with",
                f"pytest=={pytest_version(ROOT)}",
                "python",
                "-c",
                "import pytest; print(pytest.__version__)",
                online=True,
            )
        )
        check_tools(settings)
    elif args.command == "doctor":
        check_tools(settings)
        print("docs tools: passed")
    elif args.command == "test":
        run(
            isolated(
                ROOT,
                "--with",
                f"pytest=={pytest_version(ROOT)}",
                "pytest",
                "-q",
                "tests/scripts/test_adr.py",
                "tests/scripts/test_design_sections.py",
                "tests/scripts/test_docs.py",
            ),
            cwd=ROOT,
        )
    else:
        if args.command == "check":
            run(isolated(ROOT, "python", "scripts/adr.py", "lint"), cwd=ROOT)
            run(isolated(ROOT, "python", "scripts/check_agents.py"), cwd=ROOT)
        site = build(ROOT, settings)
        if args.command == "serve":
            run(
                [
                    sys.executable,
                    "-m",
                    "http.server",
                    str(args.port),
                    "--bind",
                    "127.0.0.1",
                    "--directory",
                    str(site),
                ]
            )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"docs: failed: {error}", file=sys.stderr)
        sys.exit(1)
