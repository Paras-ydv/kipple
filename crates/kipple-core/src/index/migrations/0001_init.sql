CREATE TABLE scans (
    id TEXT PRIMARY KEY,
    started_at INTEGER NOT NULL,
    finished_at INTEGER,
    roots TEXT NOT NULL,
    kipple_version TEXT NOT NULL,
    status TEXT NOT NULL,
    json TEXT NOT NULL
);

CREATE TABLE items (
    scan_id TEXT NOT NULL,
    id TEXT NOT NULL,
    integration TEXT NOT NULL,
    category TEXT NOT NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    locator TEXT NOT NULL,
    apparent INTEGER NOT NULL,
    on_disk INTEGER NOT NULL,
    reclaimable INTEGER NOT NULL,
    project_id TEXT,
    last_used INTEGER,
    safety TEXT NOT NULL,
    json TEXT NOT NULL,
    PRIMARY KEY (scan_id, id)
);

CREATE TABLE projects (
    scan_id TEXT NOT NULL,
    id TEXT NOT NULL,
    root TEXT NOT NULL,
    name TEXT NOT NULL,
    last_used INTEGER,
    exists_ INTEGER NOT NULL,
    json TEXT NOT NULL,
    PRIMARY KEY (scan_id, id)
);

-- scope: total | category | integration | project
CREATE TABLE size_points (
    ts INTEGER NOT NULL,
    scope TEXT NOT NULL,
    key TEXT NOT NULL,
    reclaimable INTEGER NOT NULL
);

CREATE INDEX items_by_integration ON items(scan_id, integration);
CREATE INDEX size_points_by_scope ON size_points(scope, key, ts);
