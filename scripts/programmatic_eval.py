"""Private offline development evaluation. Rust owns semantic and wire contracts.

One bounded worker serves a campaign, including shrink candidates. Observation inputs
are exact response captures; this module does not render expected production text.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import selectors
import subprocess
import time
from collections import Counter
from collections.abc import Callable, Iterable, Iterator
from pathlib import Path
from typing import Any

MAX_LINE = 4 * 1024 * 1024
MAX_BATCH = 32


class WorkerError(RuntimeError):
    """Explicit transport/admission failure, never a task score."""


class Worker:
    """Synchronous one-in-flight JSONL exchange with bounded bytes and cancellation."""

    def __init__(self, executable: Path, timeout: float = 20.0):
        self.timeout = timeout
        self.process = subprocess.Popen(
            [str(executable)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        assert self.process.stdin is not None and self.process.stdout is not None
        os.set_blocking(self.process.stdin.fileno(), False)
        os.set_blocking(self.process.stdout.fileno(), False)
        self.pending = bytearray()
        try:
            self.schema = self.request({"operation": "schema"})
            if self.schema.get("protocol_version") != 2 or self.schema["case"]["$schema"] != "https://json-schema.org/draft/2020-12/schema":
                raise WorkerError("unsupported generated wire schema")
        except BaseException:
            self.close()
            raise

    def request(self, value: dict[str, Any]) -> Any:
        encoded = json.dumps(value, ensure_ascii=False, separators=(",", ":"), allow_nan=False).encode() + b"\n"
        if len(encoded) > MAX_LINE:
            raise WorkerError("request exceeds 4MiB")
        assert self.process.stdin is not None and self.process.stdout is not None
        deadline = time.monotonic() + self.timeout
        offset = 0
        with selectors.DefaultSelector() as selector:
            selector.register(self.process.stdin, selectors.EVENT_WRITE)
            selector.register(self.process.stdout, selectors.EVENT_READ)
            while b"\n" not in self.pending:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    self.close()
                    raise WorkerError("worker timeout; process cancelled")
                for key, _ in selector.select(remaining):
                    if key.fileobj is self.process.stdin:
                        offset += os.write(self.process.stdin.fileno(), encoded[offset:])
                        if offset == len(encoded):
                            selector.unregister(self.process.stdin)
                    else:
                        block = os.read(self.process.stdout.fileno(), 65536)
                        if not block:
                            self.close()
                            raise WorkerError("worker closed without a complete response")
                        self.pending.extend(block)
                        if len(self.pending) > MAX_LINE:
                            self.close()
                            raise WorkerError("worker response exceeds 4MiB")
        line, _, tail = self.pending.partition(b"\n")
        self.pending = bytearray(tail)
        response = json.loads(line)
        if response["status"] != "completed":
            raise WorkerError(response["reason"])
        return response["result"]

    def judge(self, cases: Iterable[dict[str, Any]]) -> Iterator[dict[str, Any]]:
        batch: list[dict[str, Any]] = []
        for case in cases:
            batch.append(case)
            if len(batch) == MAX_BATCH:
                yield from self.request({"operation": "judge", "cases": batch})
                batch.clear()
        if batch:
            yield from self.request({"operation": "judge", "cases": batch})

    def close(self) -> None:
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=1)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=1)
        for stream in (self.process.stdin, self.process.stdout):
            if stream is not None:
                stream.close()

    def __enter__(self) -> Worker:
        return self

    def __exit__(self, *_: object) -> None:
        self.close()




def grounded_feedback(
    worker: Worker, case: dict[str, Any], current_revision: str,
    grounding: str, causes: list[str], proposed_change: str,
    affected_tasks: list[str], new_evaluator_revision: str | None = None,
) -> dict[str, Any]:
    """Route grounded proposals to either owner, binding exact retained observations.

    Rust supplies the typed observation digest; Python never guesses its serialization.
    A disposition is a proposal route, not automatic source truth or implementation.
    """
    judgment = list(worker.judge([case]))[0]
    proposal = {
        "task_id": case["task"]["id"], "packet_digest": judgment["observation_digest"],
        "observation": case["observation"], "independent_basis": case["task"]["oracle"],
        "grounding": grounding, "causes": causes, "proposed_change": proposed_change,
        "affected_tasks": affected_tasks, "evaluator_revision": new_evaluator_revision,
    }
    return worker.request({"operation": "feedback", "proposal": proposal,
                           "current_revision": current_revision})


def content_digest(value: Any) -> str:
    """Hash actual canonical input bytes, not a descriptive label."""
    encoded = json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"), allow_nan=False).encode()
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def prepare_comparison(
    worker: Worker, cases: list[dict[str, Any]], revision: str,
    baseline: dict[str, Any], candidate: dict[str, Any],
    changed_variables: list[str], meanings: dict[str, Any],
) -> dict[str, Any]:
    """Freeze actual development inventory and selected nonpopulation policies.

    All named meanings are required by Rust. Split groups are grammar/release
    families; a family crossing development and validation refuses admission.
    """
    if not cases:
        raise WorkerError("cannot freeze an empty campaign population")
    groups: dict[str, str] = {}
    for case in cases:
        task = case["task"]
        if task["split"] != "development":
            raise WorkerError("optimization comparisons admit development only")
        if groups.setdefault(task["family"], task["split"]) != task["split"]:
            raise WorkerError("source/grammar family split leakage")
    schema = worker.request({"operation": "schema"})
    frozen_meanings = {
        **{key: content_digest(value) for key, value in meanings.items()},
        "judgment": content_digest(schema["kernel_source_revision"]),
        "observation": content_digest(schema["case"]["$defs"]["Observation"]),
        "wire_schema": content_digest(schema),
        "task_population": content_digest([{"task": case["task"], "mode": case["mode"]} for case in cases]),
        "split_keys": content_digest(groups),
        "public_requests": content_digest([case["task"]["request"] for case in cases]),
        "oracle": content_digest([case["task"]["oracle"] for case in cases]),
        "journey_limits": content_digest([case["task"]["envelope"] for case in cases]),
        "completeness_applicability": content_digest([
            [case["task"]["intent"], case["task"]["oracle"]["completeness"]]
            for case in cases
        ]),
    }
    experiment = {"revision": revision, "split": "development", "meanings": frozen_meanings,
                  "baseline": baseline, "candidate": candidate,
                  "changed_variables": changed_variables}
    return worker.request({"operation": "freeze", "experiment": experiment})


def frozen_judgments(
    worker: Worker, frozen: dict[str, Any], cases: list[dict[str, Any]],
    experiment: dict[str, Any], *, lane: str,
) -> list[dict[str, Any]]:
    if lane not in ("baseline", "candidate"):
        raise WorkerError("frozen run requires an explicit baseline or candidate lane")
    schema = worker.request({"operation": "schema"})
    meanings = frozen["experiment"]["meanings"]
    actual = {"judgment": content_digest(schema["kernel_source_revision"]),
              "observation": content_digest(schema["case"]["$defs"]["Observation"]),
              "wire_schema": content_digest(schema)}
    if any(meanings[key] != value for key, value in actual.items()):
        raise WorkerError("actual running kernel/schema meanings differ from frozen comparison")
    population = [{"task": case["task"], "mode": case["mode"]} for case in cases]
    if content_digest(population) != meanings["task_population"]:
        raise WorkerError("actual task/oracle/mode inventory differs from frozen comparison")
    selected = frozen["experiment"][lane]["observation_realization"]
    if not selected or any(case["observation"]["realization"] != selected for case in cases):
        raise WorkerError("captured observation realization differs from selected comparison lane")
    worker.request({"operation": "admit", "frozen": frozen, "experiment": experiment})
    return list(worker.judge(cases))


def summary(judgments: Iterable[dict[str, Any]]) -> dict[str, Any]:
    rows = list(judgments)
    scored = [row for row in rows if row["scorable"]]
    return {
        "total_tasks": len(rows), "scorable_tasks": len(scored),
        "epistemic_counts": dict(Counter(row["epistemic"] for row in rows)),
        "execution_counts": dict(Counter(row["execution"] for row in rows)),
        "unscored_tasks": [row["task_id"] for row in rows if not row["scorable"]],
    }


def minimize(case: dict[str, Any], preserves: Callable[[dict[str, Any]], bool]) -> dict[str, Any]:
    """Shrink supported finite fixture bytes with fixed source/task/oracle meaning.

    This explicitly changes the captured packet for diagnostic controls. It does
    not rewrite producer metadata or pretend to be a production MCP observer.
    """
    current = copy.deepcopy(case)
    if current["observation"]["observer_format"] != "finite_packet_v1":
        raise WorkerError("no shrinker for this independent capture format")
    for segment in range(len(current["observation"]["segments"])):
        for field in ("groups", "references"):
            index = 0
            while True:
                packet = json.loads(current["observation"]["segments"][segment])
                if index >= len(packet[field]):
                    break
                trial = copy.deepcopy(current)
                del packet[field][index]
                trial["observation"]["segments"][segment] = json.dumps(packet, ensure_ascii=False)
                if preserves(trial):
                    current = trial
                else:
                    index += 1
    return current


def packet_ceiling(
    worker: Worker, task: dict[str, Any], alternatives: Iterable[dict[str, Any]],
    mode: str = "immediate", max_alternatives: int = 256,
) -> dict[str, Any]:
    """Exact emitted-envelope alternatives from an actual renderer/capture seam.

    Alternatives must already include setup, separators and deduplication. No
    additive cost approximation is used; Rust measures final UTF-8 emitted bytes.
    """
    observations = []
    for observation in alternatives:
        if len(observations) == max_alternatives:
            raise WorkerError("finite packet ceiling bound exceeded")
        observations.append(observation)
    if not observations:
        return {"status": "inventory_infeasible", "reason": "no supplied packet inventory"}
    cases = [{"task": task, "observation": item, "mode": mode} for item in observations]
    judgments = list(worker.judge(cases))
    sufficient = [index for index, row in enumerate(judgments) if row["epistemic"] == "sufficient"]
    if sufficient:
        best = min(sufficient, key=lambda i: len(observations[i]["segments"][0].encode()))
        return {"status": "feasible", "alternative": best, "judgments": judgments}
    relaxed = []
    for case, row in zip(cases, judgments, strict=True):
        if row["epistemic"] == "budget_infeasible":
            trial = copy.deepcopy(case)
            trial["task"]["envelope"] = {
                "max_calls": len(trial["observation"]["expansions"]),
                "max_bytes": sum(len(segment.encode()) for segment in trial["observation"]["segments"]),
            }
            relaxed.append(trial)
    diagnostic_rows = list(worker.judge(relaxed))
    if any(row["epistemic"] == "sufficient" for row in diagnostic_rows):
        status = "budget_infeasible"
    elif all(row["scorable"] or row["epistemic"] == "budget_infeasible" for row in judgments) and all(row["scorable"] for row in diagnostic_rows):
        status = "inventory_infeasible"
    else:
        status = "inconclusive"
    return {"status": status, "judgments": judgments,
            "budget_relaxation_diagnostic_only": diagnostic_rows}



def minimize_generated(
    factors: dict[str, Any],
    build: Callable[[dict[str, Any]], dict[str, Any]],
    preserves: Callable[[dict[str, Any]], bool],
) -> dict[str, Any]:
    """Rebuild source, task/context and independent oracle together while shrinking.

    Only named removable lists/maps shrink. The supported generator owns semantic
    factors and re-authors every oracle after a change; no production classifier runs.
    """
    current = copy.deepcopy(factors)
    if not preserves(build(current)):
        raise WorkerError("initial generated case does not preserve declared failure")
    for key, values in list(current.items()):
        if isinstance(values, list):
            index = 0
            while index < len(current[key]):
                trial = copy.deepcopy(current)
                del trial[key][index]
                if preserves(build(trial)):
                    current = trial
                else:
                    index += 1
        elif isinstance(values, dict):
            for field in list(values):
                trial = copy.deepcopy(current)
                del trial[key][field]
                if preserves(build(trial)):
                    current = trial
    return {"factors": current, "case": build(current)}


def mutation_outcome(original: dict[str, Any], mutant: dict[str, Any], equivalent: bool = False) -> str:
    if equivalent:
        return "equivalent"
    if mutant["applicability"] == "invalid_task":
        return "invalid"
    if not original["scorable"] or not mutant["scorable"]:
        return "inconclusive"
    return "caught" if original["epistemic"] != mutant["epistemic"] else "surviving"


def load_cases(path: Path) -> Iterator[dict[str, Any]]:
    # This entry point admits private development files only, never sealed populations.
    root = Path(__file__).resolve().parents[1] / "eval/programmatic"
    resolved = path.resolve()
    if not resolved.is_relative_to(root) or resolved.suffix != ".jsonl":
        raise WorkerError("campaign input must be private eval/programmatic JSONL")
    with resolved.open(encoding="utf-8") as handle:
        while line := handle.readline(MAX_LINE + 1):
            if len(line.encode()) > MAX_LINE:
                raise WorkerError("case line exceeds 4MiB")
            if line.strip():
                yield json.loads(line)



def load_observations(path: Path) -> dict[str, Any]:
    root = Path(__file__).resolve().parents[1] / "eval/programmatic"
    resolved = path.resolve()
    if not resolved.is_relative_to(root) or resolved.suffix != ".json":
        raise WorkerError("observation captures must be private eval/programmatic JSON")
    if resolved.stat().st_size > MAX_LINE:
        raise WorkerError("observation inventory exceeds 4MiB")
    return json.loads(resolved.read_text(encoding="utf-8"))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cases", type=Path)
    parser.add_argument("--worker", type=Path, default=Path("target/release/lctx-eval"))
    parser.add_argument("--observations", type=Path, help="exact observations keyed by task ID")
    parser.add_argument("--minimize", action="store_true", help="shrink insufficiency observations with fixed supported tasks")
    args = parser.parse_args()
    cases = list(load_cases(args.cases))
    if args.observations:
        observations = load_observations(args.observations)
        for case in cases:
            case["observation"] = observations[case["task"]["id"]]
    with Worker(args.worker.resolve()) as worker:
        judgments = list(worker.judge(cases))
        minimized = []
        if args.minimize:
            for case, original in zip(cases, judgments, strict=True):
                if original["epistemic"] == "insufficient":
                    def preserves(trial: dict[str, Any], expected: dict[str, Any] = original) -> bool:
                        row = list(worker.judge([trial]))[0]
                        return row["epistemic"] == expected["epistemic"] and row["reason"] == expected["reason"]
                    minimized.append(minimize(case, preserves))
        print(json.dumps({"lane": "offline_capture", "judgments": judgments,
                          "summary": summary(judgments), "minimized": minimized}, indent=2))


if __name__ == "__main__":
    main()
