# Z3 5 migration — 2026-09-24

The Linux NVIDIA vLLM service now uses Z3 5.1.0.0 and a source-built
TileLang `0.1.12+z3.5.1`. vLLM remains `0.30.0`, which requires TileLang
`==0.1.12`; the local version satisfies that public-version requirement.
The compiler's Rust/Python dependency sets are unchanged.

## Why a local wheel

Stock TileLang 0.1.12 requires `z3-solver>=4.13.0,<4.15.5`, and its native
`libtvm_compiler.so` has `DT_NEEDED=libz3.so.4.15`. Upgrading only the Python
Z3 package would leave a missing native dependency. TileLang 0.1.14 still
has the same upper bound, and vLLM pins 0.1.12, so this change rebuilds the
pinned source against Z3 5 instead of overriding incompatible metadata.

The patch changes only Z3 build/runtime requirements and the local version.
The new wheel names `libz3.so.5.1`, with relative runtime paths to its
installed Python dependencies. `manifest.json` records source and wheel
hashes. This is a locally maintained build, not an upstream-supported Z3 5
release. Its profile is Linux x86_64, Python 3.14.7, CUDA enabled, ROCm and
LLVM codegen disabled; it is not a portable macOS/Windows/ROCm wheel.

`services/vllm/pyproject.toml` explicitly selects this wheel with a relative
path and narrows the service lock to Linux x86_64. The lock consequently
loses other-platform wheels/variants. `uv sync --locked` changed exactly
two installed packages: TileLang and z3-solver. The existing Torch, CUDA,
Triton and vLLM installations were not changed. Git LFS covers the wheel
and raw build/test logs through the existing repository attributes.

## Qualification

All results below are from 2026-09-24.

- **passed**: four upstream TileLang arithmetic/SMT tests, plus
  `test_gemm_f16f16f32_nn` on the RTX 5090; five tests passed with none skipped.
  The copied test files are unmodified upstream sources. They ran outside
  the source checkout with `--noconftest` so upstream conftest could not
  prepend the source tree and invalidate the installed-wheel test.
- **passed**: installed service imports TileLang, Z3 and vLLM; Z3 reports
  5.1.0, SAT/model/UNSAT passes, and `/proc/self/maps` contains only
  `libz3.so.5.1` from the service's Z3 wheel.
- **passed**: `uv pip check --python services/vllm/.venv/bin/python` and
  `uv lock --check --project services/vllm`.
- **not_run**: complete model-serving/inference qualification; this is a
  bounded native-library migration, not a new full-service acceptance claim.

The first isolated wheel build failed because Cython 3.3 rejects TileLang's
cp38 limited-API target. The final build uses the attached constraint
`cython==3.2.9`. Deprecation and unregistered-marker warnings appeared in the
copied upstream tests; no errors or skips occurred.

## Reproduce

1. Download the source URL in `manifest.json`, verify its SHA-256, and extract
   it to a separate build directory.
2. In that extracted tree, apply `tilelang-z3-5.patch` with `patch -p1`.
3. Build with Python 3.14 and a CUDA 13.4 toolkit:

```sh
CMAKE_BUILD_PARALLEL_LEVEL=8 NO_VERSION_LABEL=ON USE_CUDA=ON USE_ROCM=OFF \
  uv build --wheel --python python3.14 \
  --build-constraints /absolute/path/to/tilelang-build-constraints.txt \
  --out-dir /absolute/path/to/wheels \
  --config-setting cmake.define.USE_CUDA=ON \
  --config-setting cmake.define.USE_ROCM=OFF \
  --config-setting cmake.define.USE_LLVM=OFF /absolute/path/to/tilelang-0.1.12
```

4. Install the wheel with Z3 5.1.0.0 in an isolated environment containing
   the service dependencies and pytest. Run:

```sh
python -m pytest -q --noconftest /path/to/test_arith_hard.py \
  /path/to/test_tilelang_kernel_gemm.py::test_gemm_f16f16f32_nn
```

5. Recheck the actual loaded library and the wheel's `DT_NEEDED`, then update
   the relative wheel source and lock together. Do not replace the wheel
   without updating its provenance and fresh qualification.

## Host LLVM evidence

Separately, LLVM/Clang/LLD 23.1.2 were built from the official release with
explicit `/usr/local` Z3 5.1 paths and installed under
`~/.local/opt/llvm-23.1.2-z3-5.1` (X86 and NVPTX targets). All 13 upstream
Clang `Analysis/z3` tests passed. An installed LLVM SMTAPI consumer built
using that Clang and LLD passed SAT/model/UNSAT/push/pop, and its LLVM shared
library records `libz3.so.5.1`. This does not migrate every repo to LLVM 23.
The host build, resume scripts and package-removal findings are in
`~/.local/share/z3-pivot/2026-09-24/README.md`.

## Operator handoff

At the user's request, migration work in this concurrently edited repository
stopped after this documentation update. The separate `just check` run passed
278 Rust tests, then was terminated during Python fixture generation. It is
**interrupted**, not a passing full-repository gate. No migration commit or
push was made. Full vLLM inference acceptance remains unrun.

The host-only migration script is
`/home/paul/.local/share/z3-pivot/2026-09-24/finish-system.sh`.
It defaults to a package-removal preview; privileged removal and permanent
LLVM selection remain operator actions. The host runbook describes runtime
libraries that must remain for unrelated Ubuntu applications. This script
does not modify this repository or its environments.
