-- Discovery can disappear without erasing durable observations. Recovered attempts carry
-- unknown historical fields explicitly: observed_at is not a fabricated start time.
ALTER TABLE lctx_ops.attempts ALTER COLUMN library DROP NOT NULL;
ALTER TABLE lctx_ops.attempts ALTER COLUMN started_at DROP NOT NULL;
ALTER TABLE lctx_ops.attempts ADD COLUMN registration text NOT NULL DEFAULT 'started'
    CHECK (registration IN ('started', 'reconciled'));
ALTER TABLE lctx_ops.attempts ADD COLUMN observed_at timestamptz NOT NULL DEFAULT clock_timestamp();
ALTER TABLE lctx_ops.attempts ADD CONSTRAINT attempt_provenance CHECK (
    (registration = 'started' AND library IS NOT NULL AND started_at IS NOT NULL)
    OR (registration = 'reconciled' AND library IS NULL AND started_at IS NULL));
ALTER TABLE lctx_ops.snapshots ADD COLUMN available boolean NOT NULL DEFAULT true;
ALTER TABLE lctx_ops.generations ADD COLUMN store_path text;
-- Old index entries need canonical provenance verification before becoming discoverable.
ALTER TABLE lctx_ops.generations ADD COLUMN available boolean NOT NULL DEFAULT false;
ALTER TABLE lctx_ops.generations ADD CONSTRAINT generation_provenance
    CHECK (NOT available OR store_path IS NOT NULL);
