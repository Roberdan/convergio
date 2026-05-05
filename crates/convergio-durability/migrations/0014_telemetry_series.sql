-- Rolling 7-day telemetry time-series at 1-minute resolution.
--
-- Each row holds one (metric, minute-bucket) pair.  The UNIQUE
-- constraint lets the collector upsert without racing: two ticks that
-- land in the same minute bucket overwrite rather than duplicate.
--
-- Retention (7 d = 10 080 rows per metric) is enforced by the
-- telemetry-collector loop on every tick (DELETE WHERE bucket_ts < cutoff).
CREATE TABLE IF NOT EXISTS telemetry_series (
    id        INTEGER  PRIMARY KEY AUTOINCREMENT,
    bucket_ts TEXT     NOT NULL,  -- RFC3339, seconds truncated to :00Z
    metric    TEXT     NOT NULL,
    value     INTEGER  NOT NULL,
    UNIQUE(bucket_ts, metric)
) STRICT;

CREATE INDEX IF NOT EXISTS idx_telemetry_series_bucket
    ON telemetry_series (bucket_ts);
