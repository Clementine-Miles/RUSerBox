-- The schema the last Flutter release (schema v43) created, frozen.
--
-- Fresh installs get exactly this; an existing store.db already has it. Later
-- steps are `schema::MIGRATIONS`, never an edit here. Every table is WITHOUT
-- ROWID, as Drift created them.

CREATE TABLE IF NOT EXISTS private_key (
  updated_at INTEGER NOT NULL DEFAULT 0,
  rev INTEGER NOT NULL DEFAULT 0,
  id TEXT NOT NULL,
  name TEXT NOT NULL UNIQUE,
  key TEXT NOT NULL,
  comment TEXT,
  PRIMARY KEY (id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS bmc_credential (
  updated_at INTEGER NOT NULL DEFAULT 0,
  rev INTEGER NOT NULL DEFAULT 0,
  id TEXT NOT NULL,
  name TEXT NOT NULL UNIQUE,
  user TEXT NOT NULL,
  pwd TEXT,
  PRIMARY KEY (id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS server (
  updated_at INTEGER NOT NULL DEFAULT 0,
  rev INTEGER NOT NULL DEFAULT 0,
  id TEXT NOT NULL,
  name TEXT NOT NULL,
  auto_connect INTEGER NOT NULL DEFAULT 1 CHECK ("auto_connect" IN (0, 1)),
  system_type TEXT,
  ssh_ip TEXT,
  ssh_port INTEGER,
  ssh_user TEXT,
  ssh_pwd TEXT,
  ssh_key_id TEXT REFERENCES private_key (id) ON DELETE SET NULL,
  ssh_key_path TEXT,
  ssh_alter_url TEXT,
  ssh_proxy_command TEXT,
  ssh_file_transport TEXT,
  ssh_allow_legacy_algorithms INTEGER NOT NULL DEFAULT 0 CHECK ("ssh_allow_legacy_algorithms" IN (0, 1)),
  preferred_transport TEXT,
  ssh_enabled INTEGER NOT NULL DEFAULT 1 CHECK ("ssh_enabled" IN (0, 1)),
  monitor_enabled INTEGER NOT NULL DEFAULT 1 CHECK ("monitor_enabled" IN (0, 1)),
  is_local INTEGER NOT NULL DEFAULT 0 CHECK ("is_local" IN (0, 1)),
  monitor_addr TEXT,
  monitor_user TEXT,
  monitor_pwd TEXT,
  monitor_cert_sha256 TEXT,
  monitor_allow_insecure INTEGER CHECK ("monitor_allow_insecure" IN (0, 1)),
  wol_mac TEXT,
  wol_ip TEXT,
  wol_pwd TEXT,
  bmc_addr TEXT,
  bmc_cert_sha256 TEXT,
  bmc_cred_id TEXT REFERENCES bmc_credential (id) ON DELETE SET NULL,
  prefer_temp_dev TEXT,
  temp_is_celsius INTEGER NOT NULL DEFAULT 1 CHECK ("temp_is_celsius" IN (0, 1)),
  logo_url TEXT,
  net_dev TEXT,
  script_dir TEXT,
  geo_lat REAL,
  geo_lon REAL,
  disk_counts_network INTEGER NOT NULL DEFAULT 0 CHECK ("disk_counts_network" IN (0, 1)),
  ssh_identity_agent TEXT,
  ssh_forward_agent INTEGER NOT NULL DEFAULT 0 CHECK ("ssh_forward_agent" IN (0, 1)),
  ssh_control_path TEXT,
  monitor_fallback_addr TEXT,
  PRIMARY KEY (id),
  CHECK (ssh_ip IS NOT NULL OR monitor_addr IS NOT NULL OR is_local = 1),
  CHECK (ssh_port IS NULL OR ssh_port BETWEEN 1 AND 65535)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS server_tag (
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  tag TEXT NOT NULL,
  PRIMARY KEY (server_id, tag)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS server_env (
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  PRIMARY KEY (server_id, key)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS server_jump (
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  ord INTEGER NOT NULL,
  jump_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  PRIMARY KEY (server_id, ord)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS server_identity_file (
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  ord INTEGER NOT NULL,
  path TEXT NOT NULL,
  PRIMARY KEY (server_id, ord)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS server_disabled_cmd (
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  cmd_type TEXT NOT NULL,
  PRIMARY KEY (server_id, cmd_type)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS known_host (
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  key_type TEXT NOT NULL,
  fingerprint TEXT NOT NULL,
  PRIMARY KEY (server_id, key_type)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS snippet (
  updated_at INTEGER NOT NULL DEFAULT 0,
  rev INTEGER NOT NULL DEFAULT 0,
  id TEXT NOT NULL,
  name TEXT NOT NULL UNIQUE,
  script TEXT NOT NULL,
  note TEXT,
  PRIMARY KEY (id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS snippet_tag (
  snippet_id TEXT NOT NULL REFERENCES snippet (id) ON DELETE CASCADE,
  tag TEXT NOT NULL,
  PRIMARY KEY (snippet_id, tag)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS snippet_auto_run_on (
  snippet_id TEXT NOT NULL REFERENCES snippet (id) ON DELETE CASCADE,
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  PRIMARY KEY (snippet_id, server_id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS port_forward (
  updated_at INTEGER NOT NULL DEFAULT 0,
  rev INTEGER NOT NULL DEFAULT 0,
  id TEXT NOT NULL,
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  type TEXT NOT NULL,
  local_host TEXT,
  local_port INTEGER NOT NULL DEFAULT 0,
  remote_host TEXT,
  remote_port INTEGER,
  PRIMARY KEY (id),
  CHECK (type IN ('local', 'remote', 'dynamic'))
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS remote_desktop_profile (
  updated_at INTEGER NOT NULL DEFAULT 0,
  rev INTEGER NOT NULL DEFAULT 0,
  id TEXT NOT NULL,
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  protocol TEXT NOT NULL,
  host TEXT NOT NULL DEFAULT '127.0.0.1',
  port INTEGER NOT NULL,
  username TEXT,
  password TEXT,
  domain TEXT,
  view_only INTEGER NOT NULL DEFAULT 0 CHECK ("view_only" IN (0, 1)),
  shared INTEGER NOT NULL DEFAULT 1 CHECK ("shared" IN (0, 1)),
  trusted_cert_sha256 TEXT,
  PRIMARY KEY (id),
  CHECK (protocol IN ('rdp', 'vnc')),
  CHECK (port BETWEEN 1 AND 65535),
  UNIQUE (server_id, name)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS container_host (
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  type TEXT NOT NULL,
  host TEXT NOT NULL,
  PRIMARY KEY (server_id, type),
  CHECK (type IN ('docker', 'podman'))
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS container_runtime (
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  type TEXT NOT NULL,
  PRIMARY KEY (server_id),
  CHECK (type IN ('docker', 'podman'))
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS server_pve (
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  addr TEXT NOT NULL,
  auth TEXT NOT NULL,
  pwd TEXT,
  token_id TEXT,
  token_secret TEXT,
  cert_sha256 TEXT,
  PRIMARY KEY (server_id),
  CHECK (auth IN ('password', 'token'))
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS conn_stat (
  id TEXT NOT NULL,
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  server_name TEXT NOT NULL,
  timestamp INTEGER NOT NULL,
  result TEXT NOT NULL,
  error_message TEXT NOT NULL DEFAULT '',
  duration_ms INTEGER NOT NULL,
  PRIMARY KEY (id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS server_dist (
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  dist TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (server_id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS benchmark_run (
  id TEXT NOT NULL,
  server_id TEXT NOT NULL REFERENCES server (id) ON DELETE CASCADE,
  started_at INTEGER NOT NULL,
  finished_at INTEGER,
  status TEXT NOT NULL,
  options TEXT NOT NULL,
  run_dir TEXT NOT NULL,
  result_json TEXT,
  log TEXT NOT NULL DEFAULT '',
  exit_code INTEGER,
  error TEXT NOT NULL DEFAULT '',
  PRIMARY KEY (id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS tombstone (
  tbl TEXT NOT NULL,
  row_id TEXT NOT NULL,
  deleted_at INTEGER NOT NULL,
  PRIMARY KEY (tbl, row_id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS sync_state (
  k TEXT NOT NULL,
  v TEXT NOT NULL,
  PRIMARY KEY (k)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS sync_record (
  c TEXT NOT NULL,
  id TEXT NOT NULL,
  hlc TEXT NOT NULL,
  PRIMARY KEY (c, id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS sync_passed (
  e TEXT NOT NULL,
  c TEXT NOT NULL,
  id TEXT NOT NULL,
  h TEXT NOT NULL,
  d TEXT NOT NULL,
  PRIMARY KEY (e, c, id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS kv (
  store      TEXT    NOT NULL,
  key        TEXT    NOT NULL,
  value      TEXT    NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (store, key)
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS idx_server_key ON server(ssh_key_id);
CREATE INDEX IF NOT EXISTS idx_server_bmc_cred ON server(bmc_cred_id);
CREATE INDEX IF NOT EXISTS idx_server_tag_tag ON server_tag(tag);
CREATE INDEX IF NOT EXISTS idx_server_jump_target ON server_jump(jump_id);
CREATE INDEX IF NOT EXISTS idx_snippet_tag_tag ON snippet_tag(tag);
CREATE INDEX IF NOT EXISTS idx_snippet_auto_run_server ON snippet_auto_run_on(server_id);
CREATE INDEX IF NOT EXISTS idx_port_forward_server ON port_forward(server_id);
CREATE INDEX IF NOT EXISTS idx_remote_desktop_profile_server ON remote_desktop_profile(server_id);
CREATE INDEX IF NOT EXISTS idx_conn_stat_server_ts ON conn_stat(server_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_conn_stat_ts ON conn_stat(timestamp);
CREATE INDEX IF NOT EXISTS idx_benchmark_run_server_started ON benchmark_run(server_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_tombstone_deleted ON tombstone(deleted_at);
CREATE INDEX IF NOT EXISTS idx_sync_record_hlc ON sync_record(hlc);
