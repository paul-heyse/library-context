# Retained failure inventory

Report SHA-256: `9cab4f4bfff5d2d46f01c8cc282da226f371e1a93b8a6dad5fe09ac0ec0c8ad2`.

Derived from the retained raw XML by `classify.py`; see README for causal limitations.

| Review classification | Cases |
|---|---:|
| missing_runtime_prerequisite | 679 |
| refusal_assertion_with_prerequisite_masking_candidate | 3 |
| completion_contract_expectation | 2 |

The complete failed-case list and per-suite totals are in `inventory.json`.

| Immediate reported reason | Cases |
|---|---:|
| owned disposable compiler runtime configuration is required: NotPresent | 355 |
| called `Result::unwrap()` on an `Err` value: Codec("environment variable not found") | 188 |
| stable validation runtime | 67 |
| owned persistent native fixture | 16 |
| owned native fixture required | 12 |
| installed validation runtime | 9 |
| owned compiler native fixture | 8 |
| called `Result::unwrap()` on an `Err` value: "codec: environment variable not found" | 5 |
| stable validation runtime: NotPresent | 4 |
| owned disposable compiler fixture: NotPresent | 3 |
| owned persistent native fixture: NotPresent | 3 |
| owned native fixture: NotPresent | 2 |
| native ty reference controls failed: exit status: 101 | 1 |
| Err(Codec("environment variable not found")) | 1 |
| assertion failed: error.to_string().contains("capture") &#124;&#124; error.to_string().contains("changed") | 1 |
| assertion failed: error.to_string().contains("scheduled profile differs") | 1 |
| case 0: codec: environment variable not found | 1 |
| assertion failed: error.to_string().contains("different attempt resource pool") | 1 |
| stable validation service | 1 |
| owned compiler fixture | 1 |
| owned native fixture | 1 |
| assertion `left == right` failed | 1 |
| called `Result::unwrap()` on an `Err` value: Infrastructure { class: State, detail: "native compiler authority closed or failed" } | 1 |
| installed validation configuration is required: NotPresent | 1 |
