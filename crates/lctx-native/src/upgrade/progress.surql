DEFINE TABLE native_upgrade_progress_v1 SCHEMAFULL;
DEFINE FIELD contract ON native_upgrade_progress_v1 TYPE string;
DEFINE FIELD execution ON native_upgrade_progress_v1 TYPE string;
DEFINE FIELD state ON native_upgrade_progress_v1 TYPE string;
DEFINE FIELD revision ON native_upgrade_progress_v1 TYPE int;
DEFINE FIELD sealed ON native_upgrade_progress_v1 TYPE bool;
