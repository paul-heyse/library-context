"""Compose source discovery, mdBook, Pagefind and offline links; stdlib only."""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import html
import json
import os
import posixpath
import re
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import uuid
from contextvars import ContextVar
from dataclasses import dataclass, field
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import quote, unquote, urlsplit

import design_sections as sections
from repo_paths import local_skill

ROOT = Path(__file__).resolve().parents[1]
SCOPES = {"Current", "Reference", "History"}
RECEIPT = ".publication.json"
_CLEANUPS: ContextVar[list | None] = ContextVar("docs_process_cleanup", default=None)


def receipt_digest(body: dict) -> str:
    """Detect incomplete/corrupted inventories; this is integrity, not authentication."""
    return hashlib.sha256(
        json.dumps(body, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


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
    """Keep the leader waitable through descendant cleanup, including renderer failures."""
    from harness import SpawnGuard, observe_exit

    capture = kwargs.pop("capture_output", False)
    timeout = kwargs.pop("timeout", None)
    with contextlib.ExitStack() as resources:
        output = resources.enter_context(tempfile.TemporaryFile()) if capture else None
        errors = resources.enter_context(tempfile.TemporaryFile()) if capture else None
        if capture:
            kwargs.update(stdout=output, stderr=errors)
        child = subprocess.Popen(argv, start_new_session=True, text=True, **kwargs)
        guard = SpawnGuard(child)
        try:
            with guard:
                started = time.monotonic()
                while observe_exit(child) is None:
                    if timeout is not None and time.monotonic() - started >= timeout:
                        raise subprocess.TimeoutExpired(argv, timeout)
                    time.sleep(0.01)
        finally:
            receipts = _CLEANUPS.get()
            if receipts is not None:
                receipts.append(guard.cleanup)
        stdout = stderr = None
        if output is not None and errors is not None:
            output.seek(0)
            errors.seek(0)
            stdout, stderr = output.read().decode(), errors.read().decode()
        if guard.cleanup.get("status") != "confirmed":
            raise RuntimeError("documentation subprocess cleanup unresolved")
        if guard.returncode is None:
            raise RuntimeError("documentation subprocess exit unresolved")
        if guard.returncode:
            raise subprocess.CalledProcessError(guard.returncode, argv, stdout, stderr)
        return subprocess.CompletedProcess(argv, guard.returncode, stdout, stderr)


@contextlib.contextmanager
def candidate_output(work: Path):
    """A publisher-owned temporary generation; release starts its policy grace."""
    import storage_owners
    from harness import ProcessIdentity, hold_lock, write_json_atomic
    from storage_lifecycle import Storage, admission, now

    storage = Storage()
    with admission([work]):
        candidate = Path(tempfile.mkdtemp(prefix="candidate-", dir=work))
        owner = storage.state / "owners/docs" / uuid.uuid4().hex
        lock = None
        enrolled = False
        record = {
            "schema": 1,
            "owner": ProcessIdentity.of().to_json(),
            "phase": "active",
            "cleanup": {"status": "unknown"},
            "path": str(candidate),
        }
        try:
            owner.mkdir(parents=True, mode=0o700)
            lock = hold_lock(owner / "owner.lock")
            write_json_atomic(owner / "record.json", record)
            enrolled = True
        except (OSError, ValueError, RuntimeError) as error:
            print(
                f"storage: documentation owner unavailable; candidate retained: {error}",
                file=sys.stderr,
            )
        token = _CLEANUPS.set([])
        consumer = f"docs:{candidate.name}"
        ident = (
            storage_owners.enroll(
                candidate,
                "docs-output",
                {"kind": "docs", "path": str(owner)},
                consumer,
                temporary_days=0,
            )
            if enrolled
            else None
        )
        try:
            yield candidate
        finally:
            receipts = _CLEANUPS.get()
            confirmed = receipts is not None and all(
                item.get("status") == "confirmed" for item in receipts
            )
            record.update(
                phase="finished",
                cleanup={"status": "confirmed" if confirmed else "unknown", "observed": now()},
            )
            try:
                if enrolled:
                    write_json_atomic(owner / "record.json", record)
            except (OSError, ValueError, RuntimeError) as error:
                confirmed = False
                print(
                    "storage: documentation cleanup receipt unavailable; "
                    f"candidate retained: {error}",
                    file=sys.stderr,
                )
            finally:
                if lock is not None:
                    os.close(lock)
                _CLEANUPS.reset(token)
            if confirmed:
                storage_owners.complete(ident, consumer)


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
    if heuristics := manifest["core"].get("heuristics"):
        names.append(heuristics)
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
    capture: PublicationCapture | None = None,
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
            reference = (
                capture.reference(relative) if capture else reference_identity(root, relative)
            )
            if reference["local"]:
                tag = re.sub(r"\s" + attr + r'="[^"]*"', "", tag, count=1)
                tag = (
                    tag[:-1]
                    + ' title="Local capability reference; available in the operator checkout" '
                    'class="local-reference">'
                )
                if attr == "href":
                    tag += '<span class="local-reference-label">[local reference] </span>'
                continue
            checked(root, relative)
            # Renderer files and already published paths need no source transport.
            if relative in published or (site / relative).is_file():
                continue
            source = relative
            if source.suffix == ".html" and source.with_suffix(".md") in {
                p.with_suffix(".md") for p in published
            }:
                continue
            source = Path(reference["source"])
            original = checked(root, source)
            exists_in_git = source.as_posix() in tracked or any(
                name.startswith(source.as_posix().rstrip("/") + "/") for name in tracked
            )
            if reference["kind"] == "missing":
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
                if capture:
                    destination.write_bytes(capture.read_input(source))
                else:
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
            kind = "tree" if reference["kind"] == "directory" else "blob"
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


def scope_options(pages: list[Page]) -> list[str]:
    """Search scopes with published pages, in reading order; Everything spans them."""
    present = {page.scope for page in pages}
    return [scope for scope in ("Current", "Reference", "History") if scope in present] + [
        "Everything"
    ]


def annotate(
    root: Path,
    site: Path,
    pages: list[Page],
    settings: dict,
    capture: PublicationCapture | None = None,
) -> None:
    context = capture.git if capture else git_context(root)
    revision = context["revision"]
    # A scope with no pages (History once retired records leave the tree) is never offered.
    scopes = ",".join(scope_options(pages))
    dirty = bool(context["dirty"])
    tracked = set(context["tracked"])
    published = {p.path.with_suffix(".html") for p in pages}
    errors = []
    for page in pages:
        path = site / page.path.with_suffix(".html")
        text = path.read_text()
        try:
            text = rewrite_links(
                root, site, page, text, published, tracked, settings, revision, capture
            )
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
            f'<meta name="doc-scopes" content="{scopes}">'
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
    if capture:
        for relative, data in capture.files.items():
            if relative.is_relative_to("docs/theme"):
                destination = site / "theme" / relative.relative_to("docs/theme")
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(data)
    else:
        shutil.copytree(root / "docs/theme", site / "theme", dirs_exist_ok=True)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def git_context(root: Path) -> dict:
    environment = {**os.environ, "GIT_OPTIONAL_LOCKS": "0"}
    return {
        "revision": run(
            ["git", "rev-parse", "HEAD"], cwd=root, capture_output=True, env=environment
        ).stdout.strip(),
        "dirty": run(
            ["git", "status", "--porcelain"], cwd=root, capture_output=True, env=environment
        ).stdout,
        "tracked": sorted(tracked_files(root)),
    }


def reference_identity(root: Path, relative: Path) -> dict:
    if local_skill(root, root / relative):
        return {"local": True, "source": relative.as_posix(), "kind": "local"}
    checked(root, relative)
    source = relative
    if source.suffix == ".html" and checked(root, source.with_suffix(".md")).is_file():
        source = source.with_suffix(".md")
    path = checked(root, source)
    kind = "file" if path.is_file() else "directory" if path.is_dir() else "missing"
    return {"local": False, "source": source.as_posix(), "kind": kind}


def publication_paths(root: Path, settings: dict) -> set[Path]:
    """The publisher owns its input inventory, including discovered membership."""
    paths = {
        Path("docs/site.toml"),
        Path("docs/book.toml"),
        Path("docs/design_review/design_principles/standard.toml"),
    }
    excluded = set(settings["publication"]["exclude"])
    for collection in settings["collections"]:
        for pattern in collection["include"]:
            checked(root, pattern)
            matches = {p.relative_to(root) for p in root.glob(pattern) if p.is_file()}
            if not matches:
                raise ValueError(f"empty publication selection: {pattern}")
            paths.update(p for p in matches if p.as_posix() not in excluded)
    paths.update(p.relative_to(root) for p in sections.source_paths(root))
    paths.update(Path(p) for p in settings["publication"]["assets"])
    paths.update(p.relative_to(root) for p in (root / "docs/theme").rglob("*") if p.is_file())
    index = Path(".claude/skills/README.md")
    if (root / index).exists():
        paths.add(index)
    return paths


def publisher_identity() -> dict:
    identities = {}
    for module in (sys.modules[__name__], sections, sys.modules[local_skill.__module__]):
        filename = module.__file__
        if filename is None:
            raise ValueError(f"publisher module has no source file: {module.__name__}")
        path = Path(filename)
        identities[path.name] = digest(path.read_bytes())
    return identities


def tool_identities(settings: dict, *, check_versions: bool) -> dict:
    identities = {}
    for tool, declared in settings["tools"].items():
        resolved = shutil.which(tool)
        if not resolved:
            raise ValueError(f"missing {tool}; run just bootstrap-docs")
        binary = Path(resolved).resolve()
        identity = {
            "path": str(binary),
            "sha256": digest(binary.read_bytes()),
            "declared": declared,
        }
        if check_versions:
            output = run([str(binary), "--version"], capture_output=True).stdout.strip()
            if declared and not re.search(
                r"(?<![\d.])v?" + re.escape(declared) + r"(?![\d.])", output
            ):
                raise ValueError(
                    f"{tool}: expected {declared}, got {output}; run just bootstrap-docs"
                )
            identity["version"] = output
            if digest(binary.read_bytes()) != identity["sha256"]:
                raise ValueError(f"{tool} changed during version observation")
        identities[tool] = identity
    return identities


@dataclass
class PublicationCapture:
    root: Path
    settings: dict
    files: dict[Path, bytes]
    git: dict
    tools: dict
    publisher: dict
    environment: dict[str, str]
    references: dict[str, dict] = field(default_factory=dict)

    def reference(self, relative: Path) -> dict:
        key = relative.as_posix()
        if key not in self.references:
            self.references[key] = reference_identity(self.root, relative)
        return self.references[key]

    def read_input(self, relative: Path) -> bytes:
        if relative not in self.files:
            self.files[relative] = checked(self.root, relative).read_bytes()
        return self.files[relative]

    def receipt(self) -> dict:
        body = {
            "schema": 2,
            "selection": self.settings,
            "files": {p.as_posix(): digest(data) for p, data in sorted(self.files.items())},
            "git": {**self.git, "tracked": digest("\0".join(self.git["tracked"]).encode())},
            "tools": self.tools,
            "publisher": self.publisher,
            "environment": {
                key: digest(value.encode())
                for key, value in self.environment.items()
                if key.startswith(("MDBOOK_", "PAGEFIND_", "LYCHEE_", "LC_"))
                or key in {"LANG", "TZ", "SOURCE_DATE_EPOCH"}
            },
            "references": self.references,
        }
        return {**body, "receipt_sha256": receipt_digest(body)}

    def stage_inputs(self, destination: Path) -> None:
        for relative, data in self.files.items():
            path = destination / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)


def capture_publication(
    root: Path, settings: dict, *, check_versions: bool = True
) -> PublicationCapture:
    # Copy caller overrides as well as the on-disk configuration. A partial/overridden
    # publication therefore cannot certify the default full site.
    selection = json.loads(json.dumps(settings))
    return PublicationCapture(
        root,
        selection,
        {p: checked(root, p).read_bytes() for p in publication_paths(root, selection)},
        git_context(root),
        tool_identities(selection, check_versions=check_versions),
        LOADED_PUBLISHER_IDENTITY if check_versions else publisher_identity(),
        dict(os.environ),
    )


def current_receipt(root: Path, settings: dict, previous: dict) -> dict:
    current = capture_publication(root, settings, check_versions=False)
    for tool, identity in current.tools.items():
        old = previous["tools"].get(tool, {})
        # No executable invocation during freshness observation. Identical binary bytes
        # retain the version actually checked at publication; changed bytes mismatch.
        if all(old.get(key) == value for key, value in identity.items()):
            identity["version"] = old.get("version")
    for key in previous["references"]:
        current.reference(Path(key))
    for key in previous["files"]:
        current.read_input(Path(key))
    return current.receipt()


def observe_publication(root: Path) -> tuple[str, str]:
    """Read-only input agreement; never build, install, render or run a tool binary."""
    path = root / "build/docs/site" / RECEIPT
    if not path.is_file():
        return "not_run", "publication receipt missing; run just docs-check"
    try:
        previous = json.loads(path.read_text())
        if (
            not isinstance(previous, dict)
            or previous.get("schema") != 2
            or not all(
                isinstance(previous.get(key), dict)
                for key in (
                    "selection",
                    "files",
                    "git",
                    "tools",
                    "publisher",
                    "references",
                    "environment",
                )
            )
            or not previous["files"]
            or not previous["tools"]
            or any(
                not isinstance(tool, dict) or not tool.get("version")
                for tool in previous["tools"].values()
            )
        ):
            return "not_run", "publication receipt incomplete; run just docs-check"
        body = {key: value for key, value in previous.items() if key != "receipt_sha256"}
        if previous.get("receipt_sha256") != receipt_digest(body):
            return "not_run", "publication receipt integrity mismatch; run just docs-check"
        current = current_receipt(root, config(root), previous)
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as error:
        return "stale", f"publication inputs unavailable or changed: {error}"
    if current != previous:
        return "stale", "publication inputs differ; run just docs-check"
    return "clean", "named publication inputs match (not link acceptance)"


def check_tools(settings: dict) -> None:
    """Every listed tool is present; a tool listed with a version (a pin) is exactly that one."""
    for tool, version in settings["tools"].items():
        try:
            output = run([tool, "--version"], capture_output=True).stdout.strip()
        except FileNotFoundError as error:
            raise ValueError(f"missing {tool}; run just bootstrap-docs") from error
        if version and not re.search(r"(?<![\d.])v?" + re.escape(version) + r"(?![\d.])", output):
            raise ValueError(f"{tool}: expected {version}, got {output}; run just bootstrap-docs")


def install_command(tool: str, version: str) -> list[str]:
    """Install a pinned tool at its version, and an unversioned one at the latest release."""
    pin = ["--version", version] if version else []
    return ["cargo-binstall", "--no-confirm", "--disable-strategies", "compile", *pin, tool]


def replace_site(candidate: Path, destination: Path) -> None:
    # A qualified replacement releases the previous generation into the same rules as failed
    # candidates. A leftover legacy ``previous`` directory is not deletion authority.
    with candidate_output(destination.parent) as previous:
        backup = previous / "previous"
        if destination.exists():
            destination.rename(backup)
        try:
            candidate.rename(destination)
        except OSError:
            if backup.exists():
                backup.rename(destination)
            raise


def build(root: Path, settings: dict) -> Path:
    base = settings["site"]["base_url"]
    if (
        not base.startswith("/")
        or not base.endswith("/")
        or ".." in base.split("/")
        or urlsplit(base).netloc
    ):
        raise ValueError("site base_url must be an absolute URL path ending in / without ..")
    capture = capture_publication(root, settings)
    settings = capture.settings
    work = root / "build/docs"
    work.mkdir(parents=True, exist_ok=True)
    with candidate_output(work) as candidate:
        inputs = candidate / "inputs"
        capture.stage_inputs(inputs)
        pages = stage(inputs, candidate, discover(inputs, settings), settings)
        site = candidate / "site"
        run(
            [capture.tools["mdbook"]["path"], "build", str(candidate), "--dest-dir", str(site)],
            env=capture.environment,
        )
        annotate(root, site, pages, settings, capture)
        run(
            [capture.tools["pagefind"]["path"], "--site", str(site), "--output-subdir", "pagefind"],
            env=capture.environment,
        )
        link_root = site
        if base != "/":
            link_root = candidate / "link-root"
            mount = checked(link_root, base.lstrip("/"))
            mount.parent.mkdir(parents=True, exist_ok=True)
            mount.symlink_to(site, target_is_directory=True)
        run(
            [
                capture.tools["lychee"]["path"],
                "--offline",
                "--include-fragments=anchor-only",
                "--root-dir",
                str(link_root.resolve()),
                "--no-progress",
                str(site),
            ],
            env=capture.environment,
        )
        receipt = capture.receipt()
        (site / RECEIPT).write_text(json.dumps(receipt, sort_keys=True, indent=2) + "\n")
        if current_receipt(root, settings, receipt) != receipt:
            raise ValueError(
                "publication inputs changed during rendering; prior site preserved; "
                "rerun just docs-check"
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
            run(install_command(tool, version))
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
            from storage_lifecycle import admission

            with admission([site]):
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


# Bind a publication to the implementation loaded by this process. Reading newer
# live source at capture time must not certify an older already-imported publisher.
LOADED_PUBLISHER_IDENTITY = publisher_identity()


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"docs: failed: {error}", file=sys.stderr)
        sys.exit(1)
