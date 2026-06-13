-- Private observability ledger for AURA-owned live evals.
--
-- The public status page still reads the latest static status.json. These
-- append-friendly tables retain every workflow/release snapshot so internal
-- tooling can answer: which eval is flaky, when did it regress, and how did
-- latency move over time?

CREATE TABLE observability_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source TEXT NOT NULL,
    environment TEXT NOT NULL,
    external_run_id TEXT,
    external_run_attempt INTEGER NOT NULL DEFAULT 1,
    git_sha TEXT,
    git_branch TEXT,
    workflow_name TEXT,
    release_channel TEXT,
    generated_at TIMESTAMPTZ NOT NULL,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    overall_status TEXT NOT NULL CHECK (
        overall_status IN ('operational','degraded','partial_outage','major_outage','unknown','maintenance')
    ),
    totals JSONB NOT NULL DEFAULT '{}',
    snapshot JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_observability_runs_external
    ON observability_runs (source, external_run_id, external_run_attempt)
    WHERE external_run_id IS NOT NULL;

CREATE INDEX idx_observability_runs_generated_at
    ON observability_runs (generated_at DESC, created_at DESC);

CREATE INDEX idx_observability_runs_source_env_time
    ON observability_runs (source, environment, generated_at DESC, created_at DESC);

CREATE TABLE observability_feature_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES observability_runs(id) ON DELETE CASCADE,
    feature_id TEXT NOT NULL,
    label TEXT NOT NULL,
    category TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 999,
    status TEXT NOT NULL CHECK (
        status IN ('operational','degraded','partial_outage','major_outage','unknown','maintenance')
    ),
    last_checked_at TIMESTAMPTZ,
    checks_passed INTEGER NOT NULL DEFAULT 0,
    checks_total INTEGER NOT NULL DEFAULT 0,
    checks_failed INTEGER NOT NULL DEFAULT 0,
    checks_unknown INTEGER NOT NULL DEFAULT 0,
    latency_p95_ms INTEGER,
    message TEXT NOT NULL DEFAULT '',
    feature JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_observability_features_run
    ON observability_feature_results (run_id);

CREATE INDEX idx_observability_features_history
    ON observability_feature_results (feature_id, created_at DESC);

CREATE INDEX idx_observability_features_failures
    ON observability_feature_results (feature_id, created_at DESC)
    WHERE status IN ('degraded','partial_outage','major_outage','unknown');

CREATE TABLE observability_check_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES observability_runs(id) ON DELETE CASCADE,
    feature_id TEXT NOT NULL,
    check_id TEXT NOT NULL,
    required BOOLEAN NOT NULL DEFAULT TRUE,
    status TEXT NOT NULL CHECK (status IN ('pass','warn','fail','skip','unknown')),
    feature_status TEXT NOT NULL CHECK (
        feature_status IN ('operational','degraded','partial_outage','major_outage','unknown','maintenance')
    ),
    message TEXT NOT NULL DEFAULT '',
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    latency_ms INTEGER,
    evidence JSONB NOT NULL DEFAULT '{}',
    check_payload JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_observability_checks_run
    ON observability_check_results (run_id);

CREATE INDEX idx_observability_checks_history
    ON observability_check_results (check_id, ended_at DESC NULLS LAST, created_at DESC);

CREATE INDEX idx_observability_checks_feature_time
    ON observability_check_results (feature_id, ended_at DESC NULLS LAST, created_at DESC);

CREATE INDEX idx_observability_checks_failures
    ON observability_check_results (check_id, ended_at DESC NULLS LAST, created_at DESC)
    WHERE status IN ('warn','fail','unknown');
