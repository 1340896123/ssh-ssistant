use crate::db::get_db_path;
use crate::models::{
    AccessEndpoint, AssetFolder, AssetSessionConnectResult, AssetTag, AssetUpsertPayload,
    AuditEvent, Connection as SshConnection, CredentialRef, Environment, HostAsset,
    CloudAssetRecord, JobBatchPreview, JobBatchPreviewTarget, JobBatchRequest, JobBatchResult, JobBatchResultItem,
    JobRun, JobRunArchive, JobTemplate, OpsConsoleAnswer, OpsMatchedAsset, OpsPlanStep,
    OpsSession, SavedAssetView, SyncChangeLogEntry, SyncObjectVersionSummary, SyncOverview,
    SyncServiceConfig, SyncState,
};
use crate::ssh::{client, client::AppState, command};
use rusqlite::{params, Connection as SqliteConnection};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, State};
use uuid::Uuid;

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn slugify(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn parse_labels(raw: Option<String>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn join_labels(labels: &[String]) -> String {
    labels
        .iter()
        .map(|label| label.trim())
        .filter(|label| !label.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn default_credential_name(asset_name: &str, credential_kind: &str) -> String {
    match credential_kind {
        "sshKey" => format!("{} key credential", asset_name),
        "token" => format!("{} token credential", asset_name),
        _ => format!("{} password credential", asset_name),
    }
}

fn normalize_scope_type(scope_type: &str) -> String {
    match scope_type.trim().to_lowercase().as_str() {
        "label" | "labels" | "tag" | "tags" => "tag".to_string(),
        "env" | "environment" | "environments" => "environment".to_string(),
        "asset" | "assets" => "asset".to_string(),
        "all" => "all".to_string(),
        _ => "asset".to_string(),
    }
}

fn normalize_risk_level(risk_level: Option<String>) -> String {
    match risk_level
        .unwrap_or_else(|| "medium".to_string())
        .trim()
        .to_lowercase()
        .as_str()
    {
        "low" => "low".to_string(),
        "high" => "high".to_string(),
        "critical" => "critical".to_string(),
        _ => "medium".to_string(),
    }
}

fn severity_rank(severity: &str) -> i64 {
    match severity {
        "error" => 3,
        "warning" => 2,
        _ => 1,
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetAccessHistoryEntry {
    pub asset_id: i64,
    pub connected_at: i64,
    pub status: String,
    pub reason: Option<String>,
    pub source: String,
}

pub fn init_ops_schema(app_handle: &AppHandle) -> rusqlite::Result<()> {
    let db_path = get_db_path(app_handle);
    let conn = SqliteConnection::open(db_path)?;

    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS asset_folders (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            parent_id INTEGER,
            color TEXT,
            FOREIGN KEY(parent_id) REFERENCES asset_folders(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS environments (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            slug TEXT NOT NULL UNIQUE,
            color TEXT,
            description TEXT
        );

        CREATE TABLE IF NOT EXISTS asset_tags (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            color TEXT
        );

        CREATE TABLE IF NOT EXISTS host_assets (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            host TEXT NOT NULL,
            port INTEGER NOT NULL DEFAULT 22,
            username TEXT NOT NULL,
            password TEXT,
            auth_type TEXT NOT NULL DEFAULT 'password',
            ssh_key_id INTEGER,
            jump_host TEXT,
            jump_port INTEGER,
            jump_username TEXT,
            jump_password TEXT,
            platform TEXT NOT NULL DEFAULT 'Linux',
            folder_id INTEGER,
            env_id INTEGER,
            labels_csv TEXT NOT NULL DEFAULT '',
            owner TEXT,
            criticality TEXT NOT NULL DEFAULT 'medium',
            default_workspace_path TEXT,
            access_endpoint_id INTEGER,
            bastion_chain_id TEXT,
            health_summary TEXT,
            last_accessed_at INTEGER,
            is_favorite INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
            FOREIGN KEY(folder_id) REFERENCES asset_folders(id) ON DELETE SET NULL,
            FOREIGN KEY(env_id) REFERENCES environments(id) ON DELETE SET NULL,
            FOREIGN KEY(ssh_key_id) REFERENCES ssh_keys(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS host_asset_tags (
            asset_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            PRIMARY KEY(asset_id, tag_id),
            FOREIGN KEY(asset_id) REFERENCES host_assets(id) ON DELETE CASCADE,
            FOREIGN KEY(tag_id) REFERENCES asset_tags(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS credential_refs (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            credential_kind TEXT NOT NULL,
            username TEXT,
            secret TEXT,
            ssh_key_id INTEGER,
            asset_id INTEGER,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY(ssh_key_id) REFERENCES ssh_keys(id) ON DELETE SET NULL,
            FOREIGN KEY(asset_id) REFERENCES host_assets(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS access_endpoints (
            id INTEGER PRIMARY KEY,
            asset_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            host TEXT NOT NULL,
            port INTEGER NOT NULL DEFAULT 22,
            username TEXT NOT NULL,
            auth_type TEXT,
            credential_ref_id INTEGER,
            ssh_key_id INTEGER,
            jump_host TEXT,
            jump_port INTEGER,
            jump_username TEXT,
            jump_password TEXT,
            FOREIGN KEY(asset_id) REFERENCES host_assets(id) ON DELETE CASCADE,
            FOREIGN KEY(credential_ref_id) REFERENCES credential_refs(id) ON DELETE SET NULL,
            FOREIGN KEY(ssh_key_id) REFERENCES ssh_keys(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS saved_views (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            query_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS job_templates (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            command TEXT NOT NULL,
            scope_type TEXT NOT NULL DEFAULT 'asset',
            scope_value TEXT,
            risk_level TEXT NOT NULL DEFAULT 'medium',
            requires_confirmation INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS job_runs (
            id INTEGER PRIMARY KEY,
            asset_id INTEGER,
            session_id TEXT,
            template_id INTEGER,
            command TEXT NOT NULL,
            status TEXT NOT NULL,
            output TEXT,
            risk_level TEXT NOT NULL DEFAULT 'medium',
            initiated_by TEXT,
            source TEXT,
            created_at INTEGER NOT NULL,
            completed_at INTEGER,
            FOREIGN KEY(asset_id) REFERENCES host_assets(id) ON DELETE SET NULL,
            FOREIGN KEY(template_id) REFERENCES job_templates(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS audit_events (
            id INTEGER PRIMARY KEY,
            event_type TEXT NOT NULL,
            asset_id INTEGER,
            session_id TEXT,
            job_run_id INTEGER,
            title TEXT NOT NULL,
            detail TEXT,
            severity TEXT NOT NULL DEFAULT 'info',
            metadata_json TEXT,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(asset_id) REFERENCES host_assets(id) ON DELETE SET NULL,
            FOREIGN KEY(job_run_id) REFERENCES job_runs(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS sync_state (
            id INTEGER PRIMARY KEY,
            state_key TEXT NOT NULL UNIQUE,
            status TEXT NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            endpoint_url TEXT,
            last_synced_at INTEGER,
            last_error TEXT,
            metadata_json TEXT,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS job_run_archives (
            id INTEGER PRIMARY KEY,
            job_run_id INTEGER NOT NULL UNIQUE,
            asset_id INTEGER,
            session_id TEXT,
            command TEXT NOT NULL,
            status TEXT NOT NULL,
            risk_level TEXT NOT NULL,
            output TEXT,
            summary TEXT,
            archived_at INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            completed_at INTEGER,
            source TEXT,
            FOREIGN KEY(job_run_id) REFERENCES job_runs(id) ON DELETE CASCADE,
            FOREIGN KEY(asset_id) REFERENCES host_assets(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS sync_object_versions (
            object_type TEXT NOT NULL,
            object_id TEXT NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY(object_type, object_id)
        );

        CREATE TABLE IF NOT EXISTS sync_change_log (
            id INTEGER PRIMARY KEY,
            object_type TEXT NOT NULL,
            object_id TEXT NOT NULL,
            operation TEXT NOT NULL,
            object_version INTEGER NOT NULL,
            summary TEXT NOT NULL,
            payload_json TEXT,
            sync_status TEXT NOT NULL DEFAULT 'pending',
            service_key TEXT,
            created_at INTEGER NOT NULL,
            synced_at INTEGER
        );

        CREATE TABLE IF NOT EXISTS sync_services (
            id INTEGER PRIMARY KEY,
            service_key TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            base_url TEXT,
            auth_mode TEXT NOT NULL DEFAULT 'none',
            auth_token TEXT,
            enabled INTEGER NOT NULL DEFAULT 0,
            metadata_json TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_host_assets_last_accessed_at
            ON host_assets(last_accessed_at DESC);

        CREATE INDEX IF NOT EXISTS idx_audit_events_asset_created_at
            ON audit_events(asset_id, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_job_run_archives_asset_archived_at
            ON job_run_archives(asset_id, archived_at DESC);

        CREATE INDEX IF NOT EXISTS idx_sync_change_log_created_at
            ON sync_change_log(created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_sync_change_log_status
            ON sync_change_log(sync_status, created_at DESC);
        "#,
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO asset_folders (id, name, parent_id) SELECT id, name, parent_id FROM connection_groups",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO host_assets (
            id, name, host, port, username, password, auth_type, ssh_key_id, jump_host, jump_port, jump_username, jump_password,
            platform, folder_id, owner, criticality, default_workspace_path, last_accessed_at, is_favorite, created_at, updated_at
        )
        SELECT
            id, name, host, port, username, password, COALESCE(auth_type, 'password'), ssh_key_id, jump_host, jump_port, jump_username, jump_password,
            COALESCE(os_type, 'Linux'), group_id, NULL, 'medium',
            CASE
                WHEN COALESCE(os_type, 'Linux') = 'Windows' THEN 'C:/Users/' || username
                ELSE '/home/' || username
            END,
            NULL, 0, strftime('%s','now'), strftime('%s','now')
        FROM connections",
        [],
    )?;
    conn.execute("UPDATE host_assets SET owner = NULL", [])?;
    conn.execute(
        "INSERT OR IGNORE INTO credential_refs (id, name, credential_kind, username, secret, ssh_key_id, asset_id, created_at, updated_at)
         SELECT
            1000000 + id,
            name || ' password',
            'password',
            username,
            password,
            NULL,
            id,
            strftime('%s','now'),
            strftime('%s','now')
         FROM connections
         WHERE password IS NOT NULL AND TRIM(password) <> ''",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO credential_refs (id, name, credential_kind, username, secret, ssh_key_id, asset_id, created_at, updated_at)
         SELECT
            2000000 + id,
            name,
            'sshKey',
            NULL,
            NULL,
            id,
            NULL,
            created_at,
            created_at
         FROM ssh_keys",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO access_endpoints (
            id, asset_id, name, host, port, username, auth_type, ssh_key_id, jump_host, jump_port, jump_username, jump_password
        )
        SELECT
            id, id, name || ' endpoint', host, port, username, COALESCE(auth_type, 'password'), ssh_key_id, jump_host, jump_port, jump_username, jump_password
        FROM connections",
        [],
    )?;
    conn.execute(
        "UPDATE access_endpoints
         SET credential_ref_id = CASE
            WHEN auth_type = 'key' AND ssh_key_id IS NOT NULL THEN 2000000 + ssh_key_id
            WHEN auth_type = 'password' THEN 1000000 + asset_id
            ELSE credential_ref_id
         END
         WHERE credential_ref_id IS NULL",
        [],
    )?;
    conn.execute(
        "UPDATE host_assets
         SET access_endpoint_id = COALESCE(access_endpoint_id, (
            SELECT ae.id FROM access_endpoints ae WHERE ae.asset_id = host_assets.id LIMIT 1
         )),
         updated_at = strftime('%s','now')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO sync_state (id, state_key, status, version, updated_at)
         VALUES (1, 'local-default', 'idle', 1, strftime('%s','now'))",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO sync_services (
            id, service_key, display_name, base_url, auth_mode, auth_token, enabled, metadata_json, created_at, updated_at
         ) VALUES (
            1, 'local-first', 'Local First Mirror', NULL, 'none', NULL, 1,
            '{\"kind\":\"noop\",\"supportsPush\":false,\"supportsPull\":false}', strftime('%s','now'), strftime('%s','now')
         )",
        [],
    )?;

    // Clean up orphaned foreign keys before re-enabling constraints
    conn.execute(
        "UPDATE host_assets SET ssh_key_id = NULL WHERE ssh_key_id IS NOT NULL AND ssh_key_id NOT IN (SELECT id FROM ssh_keys)",
        [],
    )?;
    conn.execute(
        "UPDATE access_endpoints SET ssh_key_id = NULL WHERE ssh_key_id IS NOT NULL AND ssh_key_id NOT IN (SELECT id FROM ssh_keys)",
        [],
    )?;
    conn.execute(
        "UPDATE access_endpoints SET credential_ref_id = NULL WHERE credential_ref_id IS NOT NULL AND credential_ref_id NOT IN (SELECT id FROM credential_refs)",
        [],
    )?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    Ok(())
}

fn append_audit_event_with_conn(
    conn: &SqliteConnection,
    event_type: &str,
    asset_id: Option<i64>,
    session_id: Option<&str>,
    job_run_id: Option<i64>,
    title: &str,
    detail: Option<&str>,
    severity: &str,
    metadata_json: Option<&str>,
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO audit_events (event_type, asset_id, session_id, job_run_id, title, detail, severity, metadata_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            event_type,
            asset_id,
            session_id,
            job_run_id,
            title,
            detail,
            severity,
            metadata_json,
            now_ts()
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

fn bump_object_version(
    conn: &SqliteConnection,
    object_type: &str,
    object_id: &str,
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO sync_object_versions (object_type, object_id, version, updated_at)
         VALUES (?1, ?2, 1, ?3)
         ON CONFLICT(object_type, object_id) DO UPDATE SET
            version = sync_object_versions.version + 1,
            updated_at = excluded.updated_at",
        params![object_type, object_id, now_ts()],
    )
    .map_err(|e| e.to_string())?;

    conn.query_row(
        "SELECT version FROM sync_object_versions WHERE object_type = ?1 AND object_id = ?2",
        params![object_type, object_id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

fn record_change_log(
    conn: &SqliteConnection,
    object_type: &str,
    object_id: &str,
    operation: &str,
    summary: &str,
    payload_json: Option<String>,
    service_key: Option<&str>,
) -> Result<i64, String> {
    let object_version = bump_object_version(conn, object_type, object_id)?;
    conn.execute(
        "INSERT INTO sync_change_log (
            object_type, object_id, operation, object_version, summary, payload_json, sync_status, service_key, created_at, synced_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?8, NULL)",
        params![
            object_type,
            object_id,
            operation,
            object_version,
            summary,
            payload_json,
            service_key,
            now_ts()
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

pub fn append_audit_event(
    app_handle: &AppHandle,
    event_type: &str,
    asset_id: Option<i64>,
    session_id: Option<&str>,
    job_run_id: Option<i64>,
    title: &str,
    detail: Option<&str>,
    severity: &str,
    metadata_json: Option<&str>,
) -> Result<i64, String> {
    let db_path = get_db_path(app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    append_audit_event_with_conn(
        &conn,
        event_type,
        asset_id,
        session_id,
        job_run_id,
        title,
        detail,
        severity,
        metadata_json,
    )
}

fn upsert_asset_tags(conn: &SqliteConnection, asset_id: i64, labels: &[String]) -> Result<(), String> {
    conn.execute(
        "DELETE FROM host_asset_tags WHERE asset_id = ?1",
        params![asset_id],
    )
    .map_err(|e| e.to_string())?;

    for label in labels.iter().map(|item| item.trim()).filter(|item| !item.is_empty()) {
        conn.execute(
            "INSERT OR IGNORE INTO asset_tags (name) VALUES (?1)",
            params![label],
        )
        .map_err(|e| e.to_string())?;
        let tag_id: i64 = conn
            .query_row(
                "SELECT id FROM asset_tags WHERE name = ?1",
                params![label],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR IGNORE INTO host_asset_tags (asset_id, tag_id) VALUES (?1, ?2)",
            params![asset_id, tag_id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn map_job_run_archive_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobRunArchive> {
    Ok(JobRunArchive {
        id: row.get(0)?,
        job_run_id: row.get(1)?,
        asset_id: row.get(2)?,
        session_id: row.get(3)?,
        command: row.get(4)?,
        status: row.get(5)?,
        risk_level: row.get(6)?,
        output: row.get(7)?,
        summary: row.get(8)?,
        archived_at: row.get(9)?,
        created_at: row.get(10)?,
        completed_at: row.get(11)?,
        source: row.get(12)?,
    })
}

fn map_sync_change_log_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncChangeLogEntry> {
    Ok(SyncChangeLogEntry {
        id: row.get(0)?,
        object_type: row.get(1)?,
        object_id: row.get(2)?,
        operation: row.get(3)?,
        object_version: row.get(4)?,
        summary: row.get(5)?,
        payload_json: row.get(6)?,
        sync_status: row.get(7)?,
        service_key: row.get(8)?,
        created_at: row.get(9)?,
        synced_at: row.get(10)?,
    })
}

fn map_sync_service_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncServiceConfig> {
    Ok(SyncServiceConfig {
        id: row.get(0)?,
        service_key: row.get(1)?,
        display_name: row.get(2)?,
        base_url: row.get(3)?,
        auth_mode: row.get(4)?,
        auth_token: row.get(5)?,
        enabled: row.get::<_, i64>(6)? != 0,
        metadata_json: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn map_host_asset_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HostAsset> {
    Ok(HostAsset {
        id: row.get(0)?,
        name: row.get(1)?,
        host: row.get(2)?,
        port: row.get(3)?,
        platform: row.get(4)?,
        folder_id: row.get(5)?,
        env_id: row.get(6)?,
        labels: parse_labels(row.get(7)?),
        owner: row.get(8)?,
        criticality: row.get(9)?,
        default_workspace_path: row.get(10)?,
        access_endpoint_id: row.get(11)?,
        bastion_chain_id: row.get(12)?,
        health_summary: row.get(13)?,
        last_accessed_at: row.get(14)?,
        is_favorite: row
            .get::<_, Option<i64>>(15)?
            .map(|value| value != 0),
        group_id: row.get(16)?,
    })
}

fn map_folder_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssetFolder> {
    Ok(AssetFolder {
        id: row.get(0)?,
        name: row.get(1)?,
        parent_id: row.get(2)?,
        color: row.get(3)?,
    })
}

fn map_environment_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Environment> {
    Ok(Environment {
        id: row.get(0)?,
        name: row.get(1)?,
        slug: row.get(2)?,
        color: row.get(3)?,
        description: row.get(4)?,
    })
}

fn map_tag_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssetTag> {
    Ok(AssetTag {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
    })
}

fn map_saved_view_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SavedAssetView> {
    Ok(SavedAssetView {
        id: row.get(0)?,
        name: row.get(1)?,
        query_json: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn map_access_endpoint_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccessEndpoint> {
    Ok(AccessEndpoint {
        id: row.get(0)?,
        asset_id: row.get(1)?,
        name: row.get(2)?,
        host: row.get(3)?,
        port: row.get(4)?,
        username: row.get(5)?,
        auth_type: row.get(6)?,
        credential_ref_id: row.get(7)?,
        ssh_key_id: row.get(8)?,
        jump_host: row.get(9)?,
        jump_port: row.get(10)?,
        jump_username: row.get(11)?,
        jump_password: row.get(12)?,
    })
}

fn map_credential_ref_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CredentialRef> {
    Ok(CredentialRef {
        id: row.get(0)?,
        name: row.get(1)?,
        credential_kind: row.get(2)?,
        username: row.get(3)?,
        secret: row.get(4)?,
        ssh_key_id: row.get(5)?,
        asset_id: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn map_job_template_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobTemplate> {
    Ok(JobTemplate {
        id: row.get(0)?,
        name: row.get(1)?,
        command: row.get(2)?,
        scope_type: row.get(3)?,
        scope_value: row.get(4)?,
        risk_level: row.get(5)?,
        requires_confirmation: row.get::<_, i64>(6)? != 0,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn map_job_run_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobRun> {
    Ok(JobRun {
        id: row.get(0)?,
        asset_id: row.get(1)?,
        session_id: row.get(2)?,
        template_id: row.get(3)?,
        command: row.get(4)?,
        status: row.get(5)?,
        output: row.get(6)?,
        risk_level: row.get(7)?,
        initiated_by: row.get(8)?,
        source: row.get(9)?,
        created_at: row.get(10)?,
        completed_at: row.get(11)?,
    })
}

fn map_audit_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuditEvent> {
    Ok(AuditEvent {
        id: row.get(0)?,
        event_type: row.get(1)?,
        asset_id: row.get(2)?,
        session_id: row.get(3)?,
        job_run_id: row.get(4)?,
        title: row.get(5)?,
        detail: row.get(6)?,
        severity: row.get(7)?,
        metadata_json: row.get(8)?,
        created_at: row.get(9)?,
    })
}

fn map_access_history_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<AssetAccessHistoryEntry> {
    Ok(AssetAccessHistoryEntry {
        asset_id: row.get(0)?,
        connected_at: row.get(1)?,
        status: row.get(2)?,
        reason: row.get(3)?,
        source: row.get(4)?,
    })
}

fn resolve_job_targets(
    conn: &SqliteConnection,
    scope_type: &str,
    scope_value: Option<&str>,
    explicit_asset_ids: &[i64],
) -> Result<Vec<JobBatchPreviewTarget>, String> {
    let normalized_scope = normalize_scope_type(scope_type);
    let trimmed_scope_value = scope_value.map(str::trim).filter(|value| !value.is_empty());
    let mut targets: Vec<JobBatchPreviewTarget> = Vec::new();

    if !explicit_asset_ids.is_empty() {
        let placeholders = explicit_asset_ids
            .iter()
            .map(|_| "?".to_string())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT
                ha.id,
                ha.name,
                ha.host,
                ha.labels_csv,
                COALESCE(env.name, '') AS env_name,
                ha.criticality
             FROM host_assets ha
             LEFT JOIN environments env ON env.id = ha.env_id
             WHERE ha.id IN ({})
             ORDER BY ha.name COLLATE NOCASE ASC",
            placeholders
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(explicit_asset_ids.iter()), |row| {
                Ok(JobBatchPreviewTarget {
                    asset_id: row.get(0)?,
                    asset_name: row.get(1)?,
                    host: row.get(2)?,
                    labels: parse_labels(row.get(3)?),
                    environment_name: {
                        let value: String = row.get(4)?;
                        if value.is_empty() {
                            None
                        } else {
                            Some(value)
                        }
                    },
                    risk_level: row.get(5)?,
                    match_reason: "Explicit asset selection".to_string(),
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            targets.push(row.map_err(|e| e.to_string())?);
        }
        return Ok(targets);
    }

    match normalized_scope.as_str() {
        "all" => {
            let mut stmt = conn
                .prepare(
                    "SELECT
                        ha.id,
                        ha.name,
                        ha.host,
                        ha.labels_csv,
                        COALESCE(env.name, '') AS env_name,
                        ha.criticality
                     FROM host_assets ha
                     LEFT JOIN environments env ON env.id = ha.env_id
                     ORDER BY ha.name COLLATE NOCASE ASC",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(JobBatchPreviewTarget {
                        asset_id: row.get(0)?,
                        asset_name: row.get(1)?,
                        host: row.get(2)?,
                        labels: parse_labels(row.get(3)?),
                        environment_name: {
                            let value: String = row.get(4)?;
                            if value.is_empty() {
                                None
                            } else {
                                Some(value)
                            }
                        },
                        risk_level: row.get(5)?,
                        match_reason: "All assets".to_string(),
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                targets.push(row.map_err(|e| e.to_string())?);
            }
        }
        "environment" => {
            let env_value = trimmed_scope_value.ok_or_else(|| "Environment scope requires a value".to_string())?;
            let like_pattern = format!("%{}%", env_value);
            let mut stmt = conn
                .prepare(
                    "SELECT
                        ha.id,
                        ha.name,
                        ha.host,
                        ha.labels_csv,
                        env.name,
                        ha.criticality
                     FROM host_assets ha
                     INNER JOIN environments env ON env.id = ha.env_id
                     WHERE env.name LIKE ?1 OR env.slug LIKE ?1
                     ORDER BY ha.name COLLATE NOCASE ASC",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![like_pattern], |row| {
                    let env_name: String = row.get(4)?;
                    Ok(JobBatchPreviewTarget {
                        asset_id: row.get(0)?,
                        asset_name: row.get(1)?,
                        host: row.get(2)?,
                        labels: parse_labels(row.get(3)?),
                        environment_name: Some(env_name.clone()),
                        risk_level: row.get(5)?,
                        match_reason: format!("Environment match: {}", env_name),
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                targets.push(row.map_err(|e| e.to_string())?);
            }
        }
        "tag" => {
            let tag_value = trimmed_scope_value.ok_or_else(|| "Tag scope requires a value".to_string())?;
            let like_pattern = format!("%{}%", tag_value);
            let mut stmt = conn
                .prepare(
                    "SELECT
                        ha.id,
                        ha.name,
                        ha.host,
                        ha.labels_csv,
                        COALESCE(env.name, '') AS env_name,
                        ha.criticality
                     FROM host_assets ha
                     LEFT JOIN environments env ON env.id = ha.env_id
                     WHERE ha.labels_csv LIKE ?1
                        OR EXISTS (
                            SELECT 1
                            FROM host_asset_tags hat
                            INNER JOIN asset_tags tag ON tag.id = hat.tag_id
                            WHERE hat.asset_id = ha.id AND tag.name LIKE ?1
                        )
                     ORDER BY ha.name COLLATE NOCASE ASC",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![like_pattern], |row| {
                    Ok(JobBatchPreviewTarget {
                        asset_id: row.get(0)?,
                        asset_name: row.get(1)?,
                        host: row.get(2)?,
                        labels: parse_labels(row.get(3)?),
                        environment_name: {
                            let value: String = row.get(4)?;
                            if value.is_empty() {
                                None
                            } else {
                                Some(value)
                            }
                        },
                        risk_level: row.get(5)?,
                        match_reason: format!("Tag match: {}", tag_value),
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                targets.push(row.map_err(|e| e.to_string())?);
            }
        }
        _ => {
            let asset_value = trimmed_scope_value.ok_or_else(|| "Asset scope requires a value".to_string())?;
            let like_pattern = format!("%{}%", asset_value);
            let mut stmt = conn
                .prepare(
                    "SELECT
                        ha.id,
                        ha.name,
                        ha.host,
                        ha.labels_csv,
                        COALESCE(env.name, '') AS env_name,
                        ha.criticality
                     FROM host_assets ha
                     LEFT JOIN environments env ON env.id = ha.env_id
                     WHERE ha.name LIKE ?1 OR ha.host LIKE ?1 OR COALESCE(ha.owner, '') LIKE ?1
                     ORDER BY ha.name COLLATE NOCASE ASC",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![like_pattern], |row| {
                    Ok(JobBatchPreviewTarget {
                        asset_id: row.get(0)?,
                        asset_name: row.get(1)?,
                        host: row.get(2)?,
                        labels: parse_labels(row.get(3)?),
                        environment_name: {
                            let value: String = row.get(4)?;
                            if value.is_empty() {
                                None
                            } else {
                                Some(value)
                            }
                        },
                        risk_level: row.get(5)?,
                        match_reason: format!("Asset search: {}", asset_value),
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                targets.push(row.map_err(|e| e.to_string())?);
            }
        }
    }

    Ok(targets)
}

pub fn map_connection_from_endpoint(
    asset: &HostAsset,
    endpoint: &AccessEndpoint,
    credential_ref: Option<&CredentialRef>,
) -> SshConnection {
    let auth_type = endpoint
        .auth_type
        .clone()
        .or_else(|| credential_ref.map(|credential| match credential.credential_kind.as_str() {
            "sshKey" => "key".to_string(),
            _ => "password".to_string(),
        }))
        .unwrap_or_else(|| "password".to_string());

    let password = credential_ref.and_then(|credential| {
        if credential.credential_kind == "password" {
            credential.secret.clone()
        } else {
            None
        }
    });

    let ssh_key_id = endpoint
        .ssh_key_id
        .or_else(|| credential_ref.and_then(|credential| credential.ssh_key_id));

    SshConnection {
        id: asset.id,
        name: asset.name.clone(),
        host: endpoint.host.clone(),
        port: endpoint.port,
        username: endpoint.username.clone(),
        password,
        auth_type: Some(auth_type),
        ssh_key_id,
        jump_host: endpoint.jump_host.clone(),
        jump_port: endpoint.jump_port,
        jump_username: endpoint.jump_username.clone(),
        jump_password: None,
        group_id: asset.folder_id.or(asset.group_id),
        os_type: Some(asset.platform.clone()),
        key_content: None,
        key_passphrase: None,
    }
}

pub fn resolve_asset_bundle(
    conn: &SqliteConnection,
    asset_id: i64,
    endpoint_id: Option<i64>,
) -> Result<(HostAsset, AccessEndpoint, Option<CredentialRef>), String> {
    let asset = conn
        .query_row(
            "SELECT id, name, host, port, platform, folder_id, env_id, labels_csv, owner, criticality,
                    default_workspace_path, access_endpoint_id, bastion_chain_id, health_summary, last_accessed_at,
                    is_favorite, folder_id
             FROM host_assets WHERE id = ?1",
            params![asset_id],
            map_host_asset_row,
        )
        .map_err(|e| e.to_string())?;

    let resolved_endpoint_id = endpoint_id
        .or(asset.access_endpoint_id)
        .ok_or_else(|| "Asset has no default access endpoint".to_string())?;

    let endpoint = conn
        .query_row(
            "SELECT id, asset_id, name, host, port, username, auth_type, credential_ref_id, ssh_key_id, jump_host, jump_port, jump_username, jump_password
             FROM access_endpoints WHERE id = ?1 AND asset_id = ?2",
            params![resolved_endpoint_id, asset_id],
            map_access_endpoint_row,
        )
        .map_err(|e| e.to_string())?;

    let credential_ref = if let Some(credential_ref_id) = endpoint.credential_ref_id {
        Some(
            conn.query_row(
                "SELECT id, name, credential_kind, username, secret, ssh_key_id, asset_id, created_at, updated_at
                 FROM credential_refs WHERE id = ?1",
                params![credential_ref_id],
                map_credential_ref_row,
            )
            .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };

    Ok((asset, endpoint, credential_ref))
}

fn save_asset_bundle(
    tx: &SqliteConnection,
    existing_asset_id: Option<i64>,
    payload: AssetUpsertPayload,
) -> Result<(i64, HostAsset), String> {
    let AssetUpsertPayload {
        mut asset,
        mut default_access_endpoint,
        default_credential_ref,
    } = payload;

    let asset_id = existing_asset_id.unwrap_or_else(|| asset.id.unwrap_or_default());
    let timestamp = now_ts();
    let labels_csv = join_labels(&asset.labels);
    let folder_id = asset.folder_id.or(asset.group_id);
    let endpoint_username = default_access_endpoint.username.clone();
    let endpoint_auth_type = default_access_endpoint
        .auth_type
        .clone()
        .or_else(|| {
            default_credential_ref.as_ref().map(|credential_ref| {
                if credential_ref.credential_kind == "sshKey" {
                    "key".to_string()
                } else {
                    "password".to_string()
                }
            })
        })
        .unwrap_or_else(|| "password".to_string());
    let endpoint_password = if endpoint_auth_type == "password" {
        default_credential_ref
            .as_ref()
            .and_then(|credential_ref| normalize_optional_string(credential_ref.secret.clone()))
    } else {
        None
    };
    let endpoint_ssh_key_id = default_access_endpoint
        .ssh_key_id
        .or_else(|| default_credential_ref.as_ref().and_then(|credential_ref| credential_ref.ssh_key_id));

    tx.execute(
        "INSERT INTO host_assets (
            id, name, host, port, username, password, auth_type, ssh_key_id, jump_host, jump_port, jump_username, jump_password,
            platform, folder_id, env_id, labels_csv, owner, criticality, default_workspace_path,
            access_endpoint_id, bastion_chain_id, health_summary, last_accessed_at, is_favorite, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, ?12, ?13, ?14, ?15, ?16, ?17, ?18, NULL, ?19, ?20, ?21, ?22, ?23, ?24)
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            host = excluded.host,
            port = excluded.port,
            username = excluded.username,
            password = excluded.password,
            auth_type = excluded.auth_type,
            ssh_key_id = excluded.ssh_key_id,
            jump_host = excluded.jump_host,
            jump_port = excluded.jump_port,
            jump_username = excluded.jump_username,
            platform = excluded.platform,
            folder_id = excluded.folder_id,
            env_id = excluded.env_id,
            labels_csv = excluded.labels_csv,
            owner = excluded.owner,
            criticality = excluded.criticality,
            default_workspace_path = excluded.default_workspace_path,
            bastion_chain_id = excluded.bastion_chain_id,
            health_summary = excluded.health_summary,
            last_accessed_at = excluded.last_accessed_at,
            is_favorite = excluded.is_favorite,
            updated_at = excluded.updated_at",
        params![
            asset_id,
            asset.name,
            asset.host,
            asset.port,
            endpoint_username,
            endpoint_password,
            endpoint_auth_type,
            endpoint_ssh_key_id,
            normalize_optional_string(default_access_endpoint.jump_host.clone()),
            default_access_endpoint.jump_port,
            normalize_optional_string(default_access_endpoint.jump_username.clone()),
            asset.platform,
            folder_id,
            asset.env_id,
            labels_csv,
            normalize_optional_string(asset.owner.clone()),
            asset.criticality,
            normalize_optional_string(asset.default_workspace_path.clone()),
            normalize_optional_string(asset.bastion_chain_id.clone()),
            normalize_optional_string(asset.health_summary.clone()),
            asset.last_accessed_at,
            asset.is_favorite.unwrap_or(false) as i64,
            timestamp,
            timestamp
        ],
    )
    .map_err(|e| e.to_string())?;

    upsert_asset_tags(tx, asset_id, &asset.labels)?;

    let credential_ref_id = if let Some(mut credential_ref) = default_credential_ref {
        let credential_id = credential_ref.id.unwrap_or_default();
        let created_at = if existing_asset_id.is_some() && credential_id != 0 {
            credential_ref.created_at
        } else {
            timestamp
        };
        let credential_name = if credential_ref.name.trim().is_empty() {
            default_credential_name(&asset.name, credential_ref.credential_kind.as_str())
        } else {
            credential_ref.name.clone()
        };

        tx.execute(
            "INSERT INTO credential_refs (id, name, credential_kind, username, secret, ssh_key_id, asset_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                credential_kind = excluded.credential_kind,
                username = excluded.username,
                secret = excluded.secret,
                ssh_key_id = excluded.ssh_key_id,
                asset_id = excluded.asset_id,
                updated_at = excluded.updated_at",
            params![
                credential_id,
                credential_name,
                credential_ref.credential_kind,
                normalize_optional_string(credential_ref.username.clone()),
                normalize_optional_string(credential_ref.secret.clone()),
                credential_ref.ssh_key_id,
                Some(asset_id),
                created_at,
                timestamp
            ],
        )
        .map_err(|e| e.to_string())?;

        if credential_id == 0 {
            Some(tx.last_insert_rowid())
        } else {
            Some(credential_id)
        }
    } else {
        None
    };

    let endpoint_id = default_access_endpoint.id.unwrap_or(asset_id);
    let endpoint_name = if default_access_endpoint.name.trim().is_empty() {
        format!("{} default endpoint", asset.name)
    } else {
        default_access_endpoint.name.clone()
    };
    tx.execute(
        "INSERT INTO access_endpoints (id, asset_id, name, host, port, username, auth_type, credential_ref_id, ssh_key_id, jump_host, jump_port, jump_username, jump_password)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL)
         ON CONFLICT(id) DO UPDATE SET
            asset_id = excluded.asset_id,
            name = excluded.name,
            host = excluded.host,
            port = excluded.port,
            username = excluded.username,
            auth_type = excluded.auth_type,
            credential_ref_id = excluded.credential_ref_id,
            ssh_key_id = excluded.ssh_key_id,
            jump_host = excluded.jump_host,
            jump_port = excluded.jump_port,
            jump_username = excluded.jump_username,
            jump_password = NULL",
        params![
            endpoint_id,
            asset_id,
            endpoint_name,
            default_access_endpoint.host,
            default_access_endpoint.port,
            default_access_endpoint.username,
            Some(endpoint_auth_type),
            credential_ref_id,
            endpoint_ssh_key_id,
            normalize_optional_string(default_access_endpoint.jump_host.clone()),
            default_access_endpoint.jump_port,
            normalize_optional_string(default_access_endpoint.jump_username.clone())
        ],
    )
    .map_err(|e| e.to_string())?;

    tx.execute(
        "UPDATE host_assets SET access_endpoint_id = ?2, updated_at = ?3 WHERE id = ?1",
        params![asset_id, endpoint_id, timestamp],
    )
    .map_err(|e| e.to_string())?;

    asset.id = Some(asset_id);
    asset.access_endpoint_id = Some(endpoint_id);
    asset.folder_id = folder_id;
    asset.group_id = folder_id;
    asset.owner = normalize_optional_string(asset.owner);
    asset.default_workspace_path = normalize_optional_string(asset.default_workspace_path);
    asset.bastion_chain_id = normalize_optional_string(asset.bastion_chain_id);
    asset.health_summary = normalize_optional_string(asset.health_summary);
    asset.is_favorite = Some(asset.is_favorite.unwrap_or(false));

    Ok((asset_id, asset))
}

fn archive_job_run_with_conn(
    conn: &SqliteConnection,
    job_run_id: i64,
    summary: Option<String>,
) -> Result<JobRunArchive, String> {
    let archived_at = now_ts();
    conn.execute(
        "INSERT INTO job_run_archives (
            job_run_id, asset_id, session_id, command, status, risk_level, output, summary, archived_at, created_at, completed_at, source
        )
        SELECT
            id, asset_id, session_id, command, status, risk_level, output, ?2, ?3, created_at, completed_at, source
        FROM job_runs
        WHERE id = ?1
        ON CONFLICT(job_run_id) DO UPDATE SET
            asset_id = excluded.asset_id,
            session_id = excluded.session_id,
            command = excluded.command,
            status = excluded.status,
            risk_level = excluded.risk_level,
            output = excluded.output,
            summary = excluded.summary,
            archived_at = excluded.archived_at,
            created_at = excluded.created_at,
            completed_at = excluded.completed_at,
            source = excluded.source",
        params![job_run_id, summary, archived_at],
    )
    .map_err(|e| e.to_string())?;

    conn.query_row(
        "SELECT id, job_run_id, asset_id, session_id, command, status, risk_level, output, summary, archived_at, created_at, completed_at, source
         FROM job_run_archives WHERE job_run_id = ?1",
        params![job_run_id],
        map_job_run_archive_row,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn asset_get_host_assets(app_handle: AppHandle) -> Result<Vec<HostAsset>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, host, port, platform, folder_id, env_id, labels_csv, owner, criticality,
                    default_workspace_path, access_endpoint_id, bastion_chain_id, health_summary, last_accessed_at, is_favorite, folder_id
             FROM host_assets
             ORDER BY COALESCE(last_accessed_at, 0) DESC, name COLLATE NOCASE ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_host_asset_row)
        .map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn asset_search_host_assets(app_handle: AppHandle, query: String) -> Result<Vec<HostAsset>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let pattern = format!("%{}%", query.trim());
    let mut stmt = conn
        .prepare(
            "SELECT id, name, host, port, platform, folder_id, env_id, labels_csv, owner, criticality,
                    default_workspace_path, access_endpoint_id, bastion_chain_id, health_summary, last_accessed_at, is_favorite, folder_id
             FROM host_assets
             WHERE name LIKE ?1 OR host LIKE ?1 OR owner LIKE ?1 OR labels_csv LIKE ?1
             ORDER BY name COLLATE NOCASE ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![pattern], map_host_asset_row)
        .map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn asset_get_asset_folders(app_handle: AppHandle) -> Result<Vec<AssetFolder>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, parent_id, color FROM asset_folders ORDER BY name COLLATE NOCASE ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_folder_row)
        .map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn asset_get_environments(app_handle: AppHandle) -> Result<Vec<Environment>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, slug, color, description FROM environments ORDER BY name COLLATE NOCASE ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_environment_row)
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn asset_get_asset_tags(app_handle: AppHandle) -> Result<Vec<AssetTag>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, color FROM asset_tags ORDER BY name COLLATE NOCASE ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_tag_row)
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn asset_get_saved_views(app_handle: AppHandle) -> Result<Vec<SavedAssetView>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, query_json, created_at, updated_at FROM saved_views ORDER BY updated_at DESC, name COLLATE NOCASE ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_saved_view_row)
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn asset_get_access_history(
    app_handle: AppHandle,
    asset_id: Option<i64>,
    limit: Option<usize>,
) -> Result<Vec<AssetAccessHistoryEntry>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(200) as i64;

    let sql = if asset_id.is_some() {
        "SELECT asset_id, created_at, status, detail, source
         FROM (
            SELECT
                asset_id,
                created_at,
                CASE
                    WHEN event_type = 'session.connected' THEN 'success'
                    ELSE 'failed'
                END AS status,
                CASE
                    WHEN event_type = 'session.connected' THEN NULL
                    ELSE detail
                END AS detail,
                COALESCE(
                    json_extract(metadata_json, '$.source'),
                    CASE
                        WHEN event_type = 'session.connected' THEN 'tree'
                        ELSE 'tree'
                    END
                ) AS source
            FROM audit_events
            WHERE asset_id = ?1
              AND event_type IN ('session.connected', 'session.connectFailed')
         )
         ORDER BY created_at DESC
         LIMIT ?2"
    } else {
        "SELECT asset_id, created_at, status, detail, source
         FROM (
            SELECT
                asset_id,
                created_at,
                CASE
                    WHEN event_type = 'session.connected' THEN 'success'
                    ELSE 'failed'
                END AS status,
                CASE
                    WHEN event_type = 'session.connected' THEN NULL
                    ELSE detail
                END AS detail,
                COALESCE(
                    json_extract(metadata_json, '$.source'),
                    CASE
                        WHEN event_type = 'session.connected' THEN 'tree'
                        ELSE 'tree'
                    END
                ) AS source
            FROM audit_events
            WHERE asset_id IS NOT NULL
              AND event_type IN ('session.connected', 'session.connectFailed')
         )
         ORDER BY created_at DESC
         LIMIT ?1"
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = if let Some(asset_id) = asset_id {
        stmt.query_map(params![asset_id, limit], map_access_history_row)
            .map_err(|e| e.to_string())?
    } else {
        stmt.query_map(params![limit], map_access_history_row)
            .map_err(|e| e.to_string())?
    };

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn asset_import_legacy_client_state(
    app_handle: AppHandle,
    favorite_asset_ids: Vec<i64>,
    history_entries: Vec<AssetAccessHistoryEntry>,
) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for asset_id in favorite_asset_ids {
        tx.execute(
            "UPDATE host_assets SET is_favorite = 1, updated_at = ?2 WHERE id = ?1",
            params![asset_id, now_ts()],
        )
        .map_err(|e| e.to_string())?;
    }

    for entry in history_entries {
        let event_type = if entry.status == "success" {
            "session.connected"
        } else {
            "session.connectFailed"
        };
        let metadata_json = serde_json::json!({
            "source": entry.source,
            "imported": true,
        })
        .to_string();
        let existing_count: i64 = tx
            .query_row(
                "SELECT COUNT(1) FROM audit_events
                 WHERE asset_id = ?1
                   AND event_type = ?2
                   AND created_at = ?3
                   AND COALESCE(json_extract(metadata_json, '$.source'), '') = ?4",
                params![
                    entry.asset_id,
                    event_type,
                    entry.connected_at,
                    entry.source.as_str()
                ],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        if existing_count == 0 {
            tx.execute(
                "INSERT INTO audit_events (event_type, asset_id, session_id, job_run_id, title, detail, severity, metadata_json, created_at)
                 VALUES (?1, ?2, NULL, NULL, ?3, ?4, ?5, ?6, ?7)",
                params![
                    event_type,
                    entry.asset_id,
                    if entry.status == "success" {
                        "Imported legacy successful connection"
                    } else {
                        "Imported legacy failed connection"
                    },
                    entry.reason,
                    if entry.status == "success" { "info" } else { "warning" },
                    metadata_json,
                    entry.connected_at
                ],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn access_get_access_endpoints(
    app_handle: AppHandle,
    asset_id: Option<i64>,
) -> Result<Vec<AccessEndpoint>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let (sql, params_vec): (&str, Vec<i64>) = if let Some(asset_id) = asset_id {
        (
            "SELECT id, asset_id, name, host, port, username, auth_type, credential_ref_id, ssh_key_id, jump_host, jump_port, jump_username, jump_password
             FROM access_endpoints WHERE asset_id = ?1 ORDER BY id ASC",
            vec![asset_id],
        )
    } else {
        (
            "SELECT id, asset_id, name, host, port, username, auth_type, credential_ref_id, ssh_key_id, jump_host, jump_port, jump_username, jump_password
             FROM access_endpoints ORDER BY asset_id ASC, id ASC",
            Vec::new(),
        )
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = if params_vec.is_empty() {
        stmt.query_map([], map_access_endpoint_row)
            .map_err(|e| e.to_string())?
    } else {
        stmt.query_map(params![params_vec[0]], map_access_endpoint_row)
            .map_err(|e| e.to_string())?
    };

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn access_get_credential_refs(app_handle: AppHandle) -> Result<Vec<CredentialRef>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, credential_kind, username, secret, ssh_key_id, asset_id, created_at, updated_at
             FROM credential_refs
             ORDER BY updated_at DESC, name COLLATE NOCASE ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_credential_ref_row)
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn access_create_access_endpoint(
    app_handle: AppHandle,
    endpoint: AccessEndpoint,
) -> Result<AccessEndpoint, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO access_endpoints (asset_id, name, host, port, username, auth_type, credential_ref_id, ssh_key_id, jump_host, jump_port, jump_username, jump_password)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL)",
        params![
            endpoint.asset_id,
            endpoint.name,
            endpoint.host,
            endpoint.port,
            endpoint.username,
            endpoint.auth_type,
            endpoint.credential_ref_id,
            endpoint.ssh_key_id,
            normalize_optional_string(endpoint.jump_host.clone()),
            endpoint.jump_port,
            normalize_optional_string(endpoint.jump_username.clone())
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.execute(
        "UPDATE host_assets SET access_endpoint_id = COALESCE(access_endpoint_id, ?2), updated_at = ?3 WHERE id = ?1",
        params![endpoint.asset_id, id, now_ts()],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "access.endpointCreated",
        Some(endpoint.asset_id),
        None,
        None,
        "Created access endpoint",
        Some(endpoint.name.as_str()),
        "info",
        None,
    )?;
    Ok(AccessEndpoint { id: Some(id), ..endpoint })
}

#[tauri::command]
pub fn access_update_access_endpoint(
    app_handle: AppHandle,
    endpoint: AccessEndpoint,
) -> Result<AccessEndpoint, String> {
    let endpoint_id = endpoint
        .id
        .ok_or_else(|| "Endpoint ID is required".to_string())?;
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE access_endpoints
         SET asset_id = ?1, name = ?2, host = ?3, port = ?4, username = ?5, auth_type = ?6, credential_ref_id = ?7, ssh_key_id = ?8,
             jump_host = ?9, jump_port = ?10, jump_username = ?11, jump_password = NULL
         WHERE id = ?12",
        params![
            endpoint.asset_id,
            endpoint.name,
            endpoint.host,
            endpoint.port,
            endpoint.username,
            endpoint.auth_type,
            endpoint.credential_ref_id,
            endpoint.ssh_key_id,
            normalize_optional_string(endpoint.jump_host.clone()),
            endpoint.jump_port,
            normalize_optional_string(endpoint.jump_username.clone()),
            endpoint_id
        ],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "access.endpointUpdated",
        Some(endpoint.asset_id),
        None,
        None,
        "Updated access endpoint",
        Some(endpoint.name.as_str()),
        "info",
        None,
    )?;
    Ok(endpoint)
}

#[tauri::command]
pub fn access_delete_access_endpoint(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let asset_id: i64 = conn
        .query_row(
            "SELECT asset_id FROM access_endpoints WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM access_endpoints WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE host_assets
         SET access_endpoint_id = (
            SELECT ae.id FROM access_endpoints ae WHERE ae.asset_id = ?2 ORDER BY ae.id ASC LIMIT 1
         ),
         updated_at = ?3
         WHERE id = ?1",
        params![asset_id, asset_id, now_ts()],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "access.endpointDeleted",
        Some(asset_id),
        None,
        None,
        "Deleted access endpoint",
        None,
        "warning",
        None,
    )?;
    Ok(())
}

#[tauri::command]
pub fn access_create_credential_ref(
    app_handle: AppHandle,
    credential_ref: CredentialRef,
) -> Result<CredentialRef, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let created_at = if credential_ref.created_at == 0 {
        now_ts()
    } else {
        credential_ref.created_at
    };
    let updated_at = now_ts();
    conn.execute(
        "INSERT INTO credential_refs (name, credential_kind, username, secret, ssh_key_id, asset_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            credential_ref.name,
            credential_ref.credential_kind,
            normalize_optional_string(credential_ref.username.clone()),
            normalize_optional_string(credential_ref.secret.clone()),
            credential_ref.ssh_key_id,
            credential_ref.asset_id,
            created_at,
            updated_at
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    append_audit_event(
        &app_handle,
        "access.credentialCreated",
        credential_ref.asset_id,
        None,
        None,
        "Created credential reference",
        Some(credential_ref.name.as_str()),
        "info",
        None,
    )?;
    Ok(CredentialRef {
        id: Some(id),
        created_at,
        updated_at,
        ..credential_ref
    })
}

#[tauri::command]
pub fn access_update_credential_ref(
    app_handle: AppHandle,
    credential_ref: CredentialRef,
) -> Result<CredentialRef, String> {
    let id = credential_ref
        .id
        .ok_or_else(|| "Credential ref ID is required".to_string())?;
    let updated_at = now_ts();
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE credential_refs
         SET name = ?1, credential_kind = ?2, username = ?3, secret = ?4, ssh_key_id = ?5, asset_id = ?6, updated_at = ?7
         WHERE id = ?8",
        params![
            credential_ref.name,
            credential_ref.credential_kind,
            normalize_optional_string(credential_ref.username.clone()),
            normalize_optional_string(credential_ref.secret.clone()),
            credential_ref.ssh_key_id,
            credential_ref.asset_id,
            updated_at,
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "access.credentialUpdated",
        credential_ref.asset_id,
        None,
        None,
        "Updated credential reference",
        Some(credential_ref.name.as_str()),
        "info",
        None,
    )?;
    Ok(CredentialRef { updated_at, ..credential_ref })
}

#[tauri::command]
pub fn access_delete_credential_ref(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let asset_id = conn
        .query_row(
            "SELECT asset_id FROM credential_refs WHERE id = ?1",
            params![id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM credential_refs WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "access.credentialDeleted",
        asset_id,
        None,
        None,
        "Deleted credential reference",
        None,
        "warning",
        None,
    )?;
    Ok(())
}

#[tauri::command]
pub fn asset_create_host_asset(
    app_handle: AppHandle,
    payload: AssetUpsertPayload,
) -> Result<HostAsset, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO host_assets (
            name, host, port, platform, folder_id, env_id, labels_csv, owner, criticality, default_workspace_path,
            access_endpoint_id, bastion_chain_id, health_summary, last_accessed_at, is_favorite, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '', NULL, 'medium', NULL, NULL, NULL, NULL, NULL, 0, ?7, ?7)",
        params![
            payload.asset.name,
            payload.asset.host,
            payload.asset.port,
            payload.asset.platform,
            payload.asset.folder_id.or(payload.asset.group_id),
            payload.asset.env_id,
            now_ts()
        ],
    )
    .map_err(|e| e.to_string())?;
    let asset_id = tx.last_insert_rowid();
    let (_, saved_asset) = save_asset_bundle(
        &tx,
        Some(asset_id),
        AssetUpsertPayload {
            asset: HostAsset {
                id: Some(asset_id),
                ..payload.asset
            },
            default_access_endpoint: AccessEndpoint {
                id: payload.default_access_endpoint.id.or(Some(asset_id)),
                asset_id,
                ..payload.default_access_endpoint
            },
            default_credential_ref: payload.default_credential_ref,
        },
    )?;

    append_audit_event_with_conn(
        &tx,
        "asset.created",
        Some(asset_id),
        None,
        None,
        "Created host asset",
        Some("Host asset created from asset center."),
        "info",
        None,
    )?;
    record_change_log(
        &tx,
        "hostAsset",
        asset_id.to_string().as_str(),
        "create",
        "Created host asset",
        Some(json!(saved_asset.clone()).to_string()),
        Some("local-first"),
    )?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(saved_asset)
}

#[tauri::command]
pub fn asset_update_host_asset(
    app_handle: AppHandle,
    payload: AssetUpsertPayload,
) -> Result<HostAsset, String> {
    let asset_id = payload
        .asset
        .id
        .ok_or_else(|| "Asset ID is required".to_string())?;
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let (_, saved_asset) = save_asset_bundle(&tx, Some(asset_id), payload)?;

    append_audit_event_with_conn(
        &tx,
        "asset.updated",
        Some(asset_id),
        None,
        None,
        "Updated host asset",
        Some("Host asset updated from asset center."),
        "info",
        None,
    )?;
    record_change_log(
        &tx,
        "hostAsset",
        asset_id.to_string().as_str(),
        "update",
        "Updated host asset",
        Some(json!(saved_asset.clone()).to_string()),
        Some("local-first"),
    )?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(saved_asset)
}

#[tauri::command]
pub fn asset_delete_host_asset(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    append_audit_event_with_conn(
        &tx,
        "asset.deleted",
        Some(id),
        None,
        None,
        "Deleted host asset",
        Some("Host asset deleted from asset center."),
        "warning",
        None,
    )?;
    record_change_log(
        &tx,
        "hostAsset",
        id.to_string().as_str(),
        "delete",
        "Deleted host asset",
        Some(json!({ "assetId": id }).to_string()),
        Some("local-first"),
    )?;
    tx.execute("DELETE FROM host_assets WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn asset_import_cloud_records(
    app_handle: AppHandle,
    records: Vec<CloudAssetRecord>,
    replace_existing: bool,
) -> Result<usize, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    if replace_existing {
        tx.execute("DELETE FROM access_endpoints", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM credential_refs", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM host_assets", [])
            .map_err(|e| e.to_string())?;
    }

    let mut imported = 0usize;
    for record in records {
        let asset = record.asset;
        let asset_id = asset.id;
        let payload = AssetUpsertPayload {
            asset,
            default_access_endpoint: record.default_access_endpoint,
            default_credential_ref: record.default_credential_ref,
        };

        let saved_asset = if let Some(existing_id) = asset_id {
            let exists: i64 = tx
                .query_row(
                    "SELECT COUNT(1) FROM host_assets WHERE id = ?1",
                    params![existing_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            if exists > 0 {
                let (_, updated_asset) = save_asset_bundle(&tx, Some(existing_id), payload)?;
                updated_asset
            } else {
                tx.execute(
                    "INSERT INTO host_assets (
                        id, name, host, port, platform, folder_id, env_id, labels_csv, owner, criticality, default_workspace_path,
                        access_endpoint_id, bastion_chain_id, health_summary, last_accessed_at, is_favorite, created_at, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, '', NULL, 'medium', NULL, NULL, NULL, NULL, NULL, 0, ?8, ?8)",
                    params![
                        existing_id,
                        payload.asset.name,
                        payload.asset.host,
                        payload.asset.port,
                        payload.asset.platform,
                        payload.asset.folder_id.or(payload.asset.group_id),
                        payload.asset.env_id,
                        now_ts()
                    ],
                )
                .map_err(|e| e.to_string())?;
                let (_, created_asset) = save_asset_bundle(&tx, Some(existing_id), payload)?;
                created_asset
            }
        } else {
            tx.execute(
                "INSERT INTO host_assets (
                    name, host, port, platform, folder_id, env_id, labels_csv, owner, criticality, default_workspace_path,
                    access_endpoint_id, bastion_chain_id, health_summary, last_accessed_at, is_favorite, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '', NULL, 'medium', NULL, NULL, NULL, NULL, NULL, 0, ?7, ?7)",
                params![
                    payload.asset.name,
                    payload.asset.host,
                    payload.asset.port,
                    payload.asset.platform,
                    payload.asset.folder_id.or(payload.asset.group_id),
                    payload.asset.env_id,
                    now_ts()
                ],
            )
            .map_err(|e| e.to_string())?;
            let created_id = tx.last_insert_rowid();
            let (_, created_asset) = save_asset_bundle(
                &tx,
                Some(created_id),
                AssetUpsertPayload {
                    asset: HostAsset {
                        id: Some(created_id),
                        ..payload.asset
                    },
                    default_access_endpoint: AccessEndpoint {
                        asset_id: created_id,
                        ..payload.default_access_endpoint
                    },
                    default_credential_ref: payload.default_credential_ref,
                },
            )?;
            created_asset
        };

        append_audit_event_with_conn(
            &tx,
            "asset.cloudImported",
            saved_asset.id,
            None,
            None,
            "Imported cloud asset",
            Some(saved_asset.name.as_str()),
            "info",
            None,
        )?;

        imported += 1;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(imported)
}

#[tauri::command]
pub fn asset_touch_host_asset(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE host_assets SET last_accessed_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id, now_ts()],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "asset.accessed",
        Some(id),
        None,
        None,
        "Asset accessed",
        Some("Asset session entry recorded."),
        "info",
        None,
    )?;
    record_change_log(
        &conn,
        "hostAsset",
        id.to_string().as_str(),
        "touch",
        "Updated asset access timestamp",
        Some(json!({ "assetId": id, "lastAccessedAt": now_ts() }).to_string()),
        Some("local-first"),
    )?;
    Ok(())
}

#[tauri::command]
pub fn asset_toggle_favorite(app_handle: AppHandle, id: i64, is_favorite: bool) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE host_assets SET is_favorite = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, is_favorite as i64, now_ts()],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "asset.favoriteChanged",
        Some(id),
        None,
        None,
        if is_favorite {
            "Asset added to favorites"
        } else {
            "Asset removed from favorites"
        },
        None,
        "info",
        None,
    )?;
    record_change_log(
        &conn,
        "hostAsset",
        id.to_string().as_str(),
        "favorite",
        if is_favorite {
            "Marked asset as favorite"
        } else {
            "Removed asset favorite mark"
        },
        Some(json!({ "assetId": id, "isFavorite": is_favorite }).to_string()),
        Some("local-first"),
    )?;
    Ok(())
}

#[tauri::command]
pub fn asset_create_asset_folder(app_handle: AppHandle, folder: AssetFolder) -> Result<AssetFolder, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO connection_groups (name, parent_id) VALUES (?1, ?2)",
        params![folder.name, folder.parent_id],
    )
    .map_err(|e| e.to_string())?;
    let folder_id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO asset_folders (id, name, parent_id, color) VALUES (?1, ?2, ?3, ?4)",
        params![folder_id, folder.name, folder.parent_id, folder.color],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event_with_conn(
        &tx,
        "folder.created",
        None,
        None,
        None,
        "Created asset folder",
        Some("Asset folder created from asset center."),
        "info",
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(AssetFolder {
        id: Some(folder_id),
        ..folder
    })
}

#[tauri::command]
pub fn asset_update_asset_folder(app_handle: AppHandle, folder: AssetFolder) -> Result<(), String> {
    let folder_id = folder.id.ok_or_else(|| "Folder ID is required".to_string())?;
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE connection_groups SET name = ?1, parent_id = ?2 WHERE id = ?3",
        params![folder.name, folder.parent_id, folder_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE asset_folders SET name = ?1, parent_id = ?2, color = ?3 WHERE id = ?4",
        params![folder.name, folder.parent_id, folder.color, folder_id],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event_with_conn(
        &tx,
        "folder.updated",
        None,
        None,
        None,
        "Updated asset folder",
        None,
        "info",
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn asset_delete_asset_folder(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    append_audit_event_with_conn(
        &tx,
        "folder.deleted",
        None,
        None,
        None,
        "Deleted asset folder",
        None,
        "warning",
        None,
    )?;
    tx.execute("DELETE FROM asset_folders WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM connection_groups WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn asset_create_environment(app_handle: AppHandle, environment: Environment) -> Result<Environment, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO environments (name, slug, color, description) VALUES (?1, ?2, ?3, ?4)",
        params![
            environment.name,
            if environment.slug.trim().is_empty() {
                slugify(&environment.name)
            } else {
                environment.slug.clone()
            },
            environment.color,
            environment.description
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    append_audit_event(
        &app_handle,
        "environment.created",
        None,
        None,
        None,
        "Created environment",
        Some(environment.name.as_str()),
        "info",
        None,
    )?;
    Ok(Environment {
        id: Some(id),
        slug: if environment.slug.trim().is_empty() {
            slugify(&environment.name)
        } else {
            environment.slug
        },
        ..environment
    })
}

#[tauri::command]
pub fn asset_update_environment(app_handle: AppHandle, environment: Environment) -> Result<(), String> {
    let id = environment.id.ok_or_else(|| "Environment ID is required".to_string())?;
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE environments SET name = ?1, slug = ?2, color = ?3, description = ?4 WHERE id = ?5",
        params![
            environment.name,
            if environment.slug.trim().is_empty() {
                slugify(&environment.name)
            } else {
                environment.slug
            },
            environment.color,
            environment.description,
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "environment.updated",
        None,
        None,
        None,
        "Updated environment",
        Some(environment.name.as_str()),
        "info",
        None,
    )?;
    Ok(())
}

#[tauri::command]
pub fn asset_delete_environment(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM environments WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "environment.deleted",
        None,
        None,
        None,
        "Deleted environment",
        None,
        "warning",
        None,
    )?;
    Ok(())
}

#[tauri::command]
pub fn asset_create_asset_tag(app_handle: AppHandle, tag: AssetTag) -> Result<AssetTag, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO asset_tags (name, color) VALUES (?1, ?2)",
        params![tag.name, tag.color],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    append_audit_event(
        &app_handle,
        "tag.created",
        None,
        None,
        None,
        "Created asset tag",
        Some(tag.name.as_str()),
        "info",
        None,
    )?;
    Ok(AssetTag { id: Some(id), ..tag })
}

#[tauri::command]
pub fn asset_delete_asset_tag(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM asset_tags WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "tag.deleted",
        None,
        None,
        None,
        "Deleted asset tag",
        None,
        "warning",
        None,
    )?;
    Ok(())
}

#[tauri::command]
pub fn asset_create_saved_view(app_handle: AppHandle, view: SavedAssetView) -> Result<SavedAssetView, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let timestamp = now_ts();
    conn.execute(
        "INSERT INTO saved_views (name, query_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
        params![view.name, view.query_json, timestamp, timestamp],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    Ok(SavedAssetView {
        id: Some(id),
        created_at: timestamp,
        updated_at: timestamp,
        ..view
    })
}

#[tauri::command]
pub fn asset_delete_saved_view(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM saved_views WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn ops_list_job_templates(app_handle: AppHandle) -> Result<Vec<JobTemplate>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, command, scope_type, scope_value, risk_level, requires_confirmation, created_at, updated_at
             FROM job_templates
             ORDER BY updated_at DESC, name COLLATE NOCASE ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_job_template_row)
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn ops_create_job_template(app_handle: AppHandle, template: JobTemplate) -> Result<JobTemplate, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let timestamp = now_ts();
    conn.execute(
        "INSERT INTO job_templates (name, command, scope_type, scope_value, risk_level, requires_confirmation, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            template.name,
            template.command,
            template.scope_type,
            template.scope_value,
            template.risk_level,
            template.requires_confirmation as i64,
            timestamp,
            timestamp
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    append_audit_event(
        &app_handle,
        "jobTemplate.created",
        None,
        None,
        None,
        "Created job template",
        Some(template.name.as_str()),
        "info",
        None,
    )?;
    let saved = JobTemplate {
        id: Some(id),
        created_at: timestamp,
        updated_at: timestamp,
        ..template
    };
    record_change_log(
        &conn,
        "jobTemplate",
        id.to_string().as_str(),
        "create",
        "Created job template",
        Some(json!(saved.clone()).to_string()),
        Some("local-first"),
    )?;
    Ok(saved)
}

#[tauri::command]
pub fn ops_delete_job_template(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM job_templates WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "jobTemplate.deleted",
        None,
        None,
        None,
        "Deleted job template",
        None,
        "warning",
        None,
    )?;
    record_change_log(
        &conn,
        "jobTemplate",
        id.to_string().as_str(),
        "delete",
        "Deleted job template",
        Some(json!({ "jobTemplateId": id }).to_string()),
        Some("local-first"),
    )?;
    Ok(())
}

#[tauri::command]
pub fn ops_list_job_runs(app_handle: AppHandle, asset_id: Option<i64>) -> Result<Vec<JobRun>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let (sql, param): (&str, Option<i64>) = if let Some(asset_id) = asset_id {
        (
            "SELECT id, asset_id, session_id, template_id, command, status, output, risk_level, initiated_by, source, created_at, completed_at
             FROM job_runs WHERE asset_id = ?1 ORDER BY created_at DESC",
            Some(asset_id),
        )
    } else {
        (
            "SELECT id, asset_id, session_id, template_id, command, status, output, risk_level, initiated_by, source, created_at, completed_at
             FROM job_runs ORDER BY created_at DESC",
            None,
        )
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = if let Some(asset_id) = param {
        stmt.query_map(params![asset_id], map_job_run_row)
            .map_err(|e| e.to_string())?
    } else {
        stmt.query_map([], map_job_run_row)
            .map_err(|e| e.to_string())?
    };

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn ops_preview_job_batch(
    app_handle: AppHandle,
    request: JobBatchRequest,
) -> Result<JobBatchPreview, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let targets = resolve_job_targets(
        &conn,
        request.scope_type.as_str(),
        request.scope_value.as_deref(),
        &request.target_asset_ids,
    )?;
    let clients = request.target_asset_ids.len();
    let mut warnings = Vec::new();
    let risk_level = normalize_risk_level(request.risk_level.clone());

    if targets.is_empty() {
        warnings.push("No matching assets were found for the current scope.".to_string());
    }
    if targets.len() > 10 {
        warnings.push(format!(
            "This batch targets {} assets. Consider narrowing scope or running in stages.",
            targets.len()
        ));
    }
    if risk_level == "critical" || risk_level == "high" {
        warnings.push("High-risk commands should be reviewed carefully before execution.".to_string());
    }

    Ok(JobBatchPreview {
        command: request.command_text,
        scope_type: normalize_scope_type(request.scope_type.as_str()),
        scope_value: request.scope_value,
        risk_level: risk_level.clone(),
        target_count: targets.len(),
        requires_confirmation: risk_level != "low" || targets.len() > 1,
        suggested_session_reuse: clients.min(targets.len()),
        targets,
        warnings,
    })
}

#[tauri::command]
pub async fn ops_execute_job(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    asset_id: Option<i64>,
    command_text: String,
    risk_level: Option<String>,
    source: Option<String>,
) -> Result<JobRun, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let created_at = now_ts();
    conn.execute(
        "INSERT INTO job_runs (asset_id, session_id, command, status, output, risk_level, initiated_by, source, created_at, completed_at)
         VALUES (?1, ?2, ?3, 'running', NULL, ?4, 'local-user', ?5, ?6, NULL)",
        params![
            asset_id,
            session_id,
            command_text,
            risk_level.clone().unwrap_or_else(|| "medium".to_string()),
            source.clone(),
            created_at
        ],
    )
    .map_err(|e| e.to_string())?;
    let job_run_id = conn.last_insert_rowid();
    drop(conn);

    append_audit_event(
        &app_handle,
        "job.started",
        asset_id,
        Some(session_id.as_str()),
        Some(job_run_id),
        "Started job execution",
        Some(command_text.as_str()),
        "info",
        None,
    )?;

    let output = command::exec_command(
        app_handle.clone(),
        state,
        session_id.clone(),
        command_text.clone(),
        None,
    )
    .await;

    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    match output {
        Ok(result) => {
            conn.execute(
                "UPDATE job_runs SET status = 'completed', output = ?2, completed_at = ?3 WHERE id = ?1",
                params![job_run_id, result, now_ts()],
            )
            .map_err(|e| e.to_string())?;
            append_audit_event(
                &app_handle,
                "job.completed",
                asset_id,
                Some(session_id.as_str()),
                Some(job_run_id),
                "Completed job execution",
                Some(command_text.as_str()),
                "info",
                None,
            )?;
        }
        Err(error) => {
            conn.execute(
                "UPDATE job_runs SET status = 'error', output = ?2, completed_at = ?3 WHERE id = ?1",
                params![job_run_id, error, now_ts()],
            )
            .map_err(|e| e.to_string())?;
            append_audit_event(
                &app_handle,
                "job.failed",
                asset_id,
                Some(session_id.as_str()),
                Some(job_run_id),
                "Job execution failed",
                Some(command_text.as_str()),
                "warning",
                None,
            )?;
        }
    }

    conn.query_row(
        "SELECT id, asset_id, session_id, template_id, command, status, output, risk_level, initiated_by, source, created_at, completed_at
         FROM job_runs WHERE id = ?1",
        params![job_run_id],
        map_job_run_row,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ops_execute_job_batch(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    request: JobBatchRequest,
) -> Result<JobBatchResult, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(&db_path).map_err(|e| e.to_string())?;
    let preview_targets = resolve_job_targets(
        &conn,
        request.scope_type.as_str(),
        request.scope_value.as_deref(),
        &request.target_asset_ids,
    )?;
    drop(conn);

    let started_at = now_ts();
    let mut items = Vec::new();
    let mut warnings = Vec::new();
    let mut completed = 0usize;
    let mut failed = 0usize;
    let normalized_risk = normalize_risk_level(request.risk_level.clone());

    if preview_targets.is_empty() {
        return Err("No matching assets found for batch execution".to_string());
    }

    for target in preview_targets {
        let mut created_temp_session = false;
        let mut used_existing_session = false;
        let session_id = {
            let clients = state.clients.lock().map_err(|e| e.to_string())?;
            clients.iter().find_map(|(session_id, client)| {
                if client.asset_id == Some(target.asset_id) {
                    Some(session_id.clone())
                } else {
                    None
                }
            })
        };

        let active_session_id = if let Some(existing_session_id) = session_id {
            used_existing_session = true;
            existing_session_id
        } else {
            let conn = SqliteConnection::open(&db_path).map_err(|e| e.to_string())?;
            let (asset, endpoint, credential_ref) =
                resolve_asset_bundle(&conn, target.asset_id, None)?;
            drop(conn);
            let ssh_config = map_connection_from_endpoint(&asset, &endpoint, credential_ref.as_ref());
            let temp_session_id = client::connect(
                app_handle.clone(),
                state.clone(),
                ssh_config,
                Some(format!("batch-{}-{}", target.asset_id, Uuid::new_v4())),
            )
            .await?;

            {
                let mut clients = state.clients.lock().map_err(|e| e.to_string())?;
                if let Some(client) = clients.get_mut(&temp_session_id) {
                    client.asset_id = Some(target.asset_id);
                    client.access_endpoint_id = endpoint.id;
                    client.credential_ref_id = endpoint
                        .credential_ref_id
                        .or_else(|| credential_ref.as_ref().and_then(|item| item.id));
                    client.bastion_chain_id = asset.bastion_chain_id.clone();
                }
            }
            created_temp_session = true;
            temp_session_id
        };

        let job_run_outcome = ops_execute_job(
            app_handle.clone(),
            state.clone(),
            active_session_id.clone(),
            Some(target.asset_id),
            request.command_text.clone(),
            Some(normalized_risk.clone()),
            request.source.clone().or_else(|| Some("job-batch".to_string())),
        )
        .await;

        match job_run_outcome {
            Ok(job_run) => {
                let summary = if job_run.status == "completed" {
                    format!("Batch execution completed on {}", target.asset_name)
                } else {
                    format!("Batch execution ended with status {} on {}", job_run.status, target.asset_name)
                };
                let conn = SqliteConnection::open(&db_path).map_err(|e| e.to_string())?;
                if let Some(job_run_id) = job_run.id {
                    let _ = archive_job_run_with_conn(&conn, job_run_id, Some(summary.clone()));
                    let _ = record_change_log(
                        &conn,
                        "jobRun",
                        job_run_id.to_string().as_str(),
                        "archive",
                        summary.as_str(),
                        Some(
                            json!({
                                "assetId": target.asset_id,
                                "assetName": target.asset_name,
                                "status": job_run.status,
                                "source": request.source.clone().unwrap_or_else(|| "job-batch".to_string())
                            })
                            .to_string(),
                        ),
                        Some("local-first"),
                    );
                }
                if job_run.status == "completed" {
                    completed += 1;
                } else {
                    failed += 1;
                }
                items.push(JobBatchResultItem {
                    asset_id: target.asset_id,
                    asset_name: target.asset_name,
                    session_id: Some(active_session_id.clone()),
                    job_run_id: job_run.id,
                    status: job_run.status,
                    output: job_run.output,
                    error: None,
                    risk_level: job_run.risk_level,
                    used_existing_session,
                });
            }
            Err(error) => {
                failed += 1;
                warnings.push(format!("{}: {}", target.asset_name, error));
                let conn = SqliteConnection::open(&db_path).map_err(|e| e.to_string())?;
                let _ = append_audit_event_with_conn(
                    &conn,
                    "job.batchFailed",
                    Some(target.asset_id),
                    Some(active_session_id.as_str()),
                    None,
                    "Batch job execution failed",
                    Some(error.as_str()),
                    "warning",
                    Some(
                        json!({
                            "assetName": target.asset_name,
                            "command": request.command_text,
                            "scopeType": request.scope_type,
                            "scopeValue": request.scope_value,
                        })
                        .to_string()
                        .as_str(),
                    ),
                );
                items.push(JobBatchResultItem {
                    asset_id: target.asset_id,
                    asset_name: target.asset_name,
                    session_id: Some(active_session_id.clone()),
                    job_run_id: None,
                    status: "error".to_string(),
                    output: None,
                    error: Some(error),
                    risk_level: normalized_risk.clone(),
                    used_existing_session,
                });
            }
        }

        if created_temp_session {
            let _ = client::disconnect(state.clone(), active_session_id).await;
        }
    }

    Ok(JobBatchResult {
        total: items.len(),
        completed,
        failed,
        started_at,
        completed_at: now_ts(),
        items,
        warnings,
    })
}

#[tauri::command]
pub fn ops_list_job_archives(
    app_handle: AppHandle,
    asset_id: Option<i64>,
    limit: Option<usize>,
) -> Result<Vec<JobRunArchive>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(100) as i64;
    let (sql, asset_param): (&str, Option<i64>) = if let Some(asset_id) = asset_id {
        (
            "SELECT id, job_run_id, asset_id, session_id, command, status, risk_level, output, summary, archived_at, created_at, completed_at, source
             FROM job_run_archives WHERE asset_id = ?1 ORDER BY archived_at DESC LIMIT ?2",
            Some(asset_id),
        )
    } else {
        (
            "SELECT id, job_run_id, asset_id, session_id, command, status, risk_level, output, summary, archived_at, created_at, completed_at, source
             FROM job_run_archives ORDER BY archived_at DESC LIMIT ?1",
            None,
        )
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = if let Some(asset_id) = asset_param {
        stmt.query_map(params![asset_id, limit], map_job_run_archive_row)
            .map_err(|e| e.to_string())?
    } else {
        stmt.query_map(params![limit], map_job_run_archive_row)
            .map_err(|e| e.to_string())?
    };

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn ops_console_query(
    app_handle: AppHandle,
    query: String,
    selected_asset_id: Option<i64>,
) -> Result<OpsConsoleAnswer, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let trimmed_query = query.trim();
    if trimmed_query.is_empty() {
        return Err("Query is required".to_string());
    }

    let normalized_query = trimmed_query.to_lowercase();
    let pattern = format!("%{}%", trimmed_query);
    let asset_sql = if selected_asset_id.is_some() {
        "SELECT
            ha.id,
            ha.name,
            ha.host,
            ha.criticality,
            env.name,
            ha.health_summary,
            CASE
                WHEN ha.name LIKE ?1 THEN 'Name matched'
                WHEN ha.host LIKE ?1 THEN 'Host matched'
                WHEN ha.labels_csv LIKE ?1 THEN 'Label matched'
                WHEN COALESCE(ha.owner, '') LIKE ?1 THEN 'Owner matched'
                ELSE 'Selected asset'
            END AS match_reason
         FROM host_assets ha
         LEFT JOIN environments env ON env.id = ha.env_id
         WHERE ha.id = ?2
         ORDER BY ha.name COLLATE NOCASE ASC"
    } else {
        "SELECT
            ha.id,
            ha.name,
            ha.host,
            ha.criticality,
            env.name,
            ha.health_summary,
            CASE
                WHEN ha.name LIKE ?1 THEN 'Name matched'
                WHEN ha.host LIKE ?1 THEN 'Host matched'
                WHEN ha.labels_csv LIKE ?1 THEN 'Label matched'
                WHEN COALESCE(ha.owner, '') LIKE ?1 THEN 'Owner matched'
                WHEN COALESCE(env.name, '') LIKE ?1 THEN 'Environment matched'
                ELSE 'Critical asset context'
            END AS match_reason
         FROM host_assets ha
         LEFT JOIN environments env ON env.id = ha.env_id
         WHERE ha.name LIKE ?1
            OR ha.host LIKE ?1
            OR ha.labels_csv LIKE ?1
            OR COALESCE(ha.owner, '') LIKE ?1
            OR COALESCE(env.name, '') LIKE ?1
         ORDER BY
            CASE ha.criticality
                WHEN 'critical' THEN 1
                WHEN 'high' THEN 2
                WHEN 'medium' THEN 3
                ELSE 4
            END,
            ha.name COLLATE NOCASE ASC
         LIMIT 8"
    };

    let mut matched_assets = Vec::new();
    let mut stmt = conn.prepare(asset_sql).map_err(|e| e.to_string())?;
    if let Some(asset_id) = selected_asset_id {
        let rows = stmt
            .query_map(params![pattern, asset_id], |row| {
                Ok(OpsMatchedAsset {
                    asset_id: row.get(0)?,
                    asset_name: row.get(1)?,
                    host: row.get(2)?,
                    criticality: row.get(3)?,
                    environment_name: row.get(4)?,
                    health_summary: row.get(5)?,
                    match_reason: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            matched_assets.push(row.map_err(|e| e.to_string())?);
        }
    } else {
        let rows = stmt
            .query_map(params![pattern], |row| {
                Ok(OpsMatchedAsset {
                    asset_id: row.get(0)?,
                    asset_name: row.get(1)?,
                    host: row.get(2)?,
                    criticality: row.get(3)?,
                    environment_name: row.get(4)?,
                    health_summary: row.get(5)?,
                    match_reason: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            matched_assets.push(row.map_err(|e| e.to_string())?);
        }
    }

    let mut recent_events = Vec::new();
    if !matched_assets.is_empty() {
        let asset_ids = matched_assets
            .iter()
            .map(|item| item.asset_id.to_string())
            .collect::<Vec<_>>();
        let sql = format!(
            "SELECT id, event_type, asset_id, session_id, job_run_id, title, detail, severity, metadata_json, created_at
             FROM audit_events
             WHERE asset_id IN ({})
             ORDER BY created_at DESC
             LIMIT 10",
            asset_ids.join(",")
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], map_audit_event_row)
            .map_err(|e| e.to_string())?;
        for row in rows {
            recent_events.push(row.map_err(|e| e.to_string())?);
        }
    }

    let top_event = recent_events
        .iter()
        .max_by_key(|event| (severity_rank(event.severity.as_str()), event.created_at));

    let status_explanation = if let Some(event) = top_event {
        Some(format!(
            "Recent signal: [{}] {}. {}",
            event.severity,
            event.title,
            event.detail.clone().unwrap_or_else(|| "Review latest audit trail and validate the current state before making changes.".to_string())
        ))
    } else if !matched_assets.is_empty() {
        Some("No recent audit anomalies found for the matched assets. Start with a read-only validation pass.".to_string())
    } else {
        Some("No direct asset matches were found. Try searching by hostname, label, owner, or environment.".to_string())
    };

    let recommended_checks = if normalized_query.contains("disk") || normalized_query.contains("storage") {
        vec![
            "Run `df -h` to confirm filesystem pressure.".to_string(),
            "Run `du -sh <path>` on the hottest paths before cleanup.".to_string(),
            "Verify whether log rotation or backup retention changed recently.".to_string(),
        ]
    } else if normalized_query.contains("cpu") || normalized_query.contains("load") || normalized_query.contains("slow") {
        vec![
            "Run `uptime` and `top -bn1 | head -30` to inspect load and top processes.".to_string(),
            "Check whether the issue is isolated to one asset or a tagged fleet.".to_string(),
            "Inspect deploy, cron, and batch job windows around the first symptom.".to_string(),
        ]
    } else if normalized_query.contains("memory") || normalized_query.contains("oom") {
        vec![
            "Run `free -m` and inspect cgroup or service memory ceilings.".to_string(),
            "Review `dmesg | tail -n 50` for OOM killer activity.".to_string(),
            "Validate whether restart loops are masking memory pressure.".to_string(),
        ]
    } else {
        vec![
            "Confirm the scope: one asset, an environment, or a tagged batch.".to_string(),
            "Start with read-only inspection commands before remediation.".to_string(),
            "Capture a brief result review after each execution step.".to_string(),
        ]
    };

    let primary_asset = matched_assets.first();
    let plan_steps = vec![
        OpsPlanStep {
            id: "inspect".to_string(),
            title: "Inspect the current state".to_string(),
            description: "Run a read-only validation step on the primary target and verify the symptom is reproducible.".to_string(),
            command: Some(if normalized_query.contains("disk") || normalized_query.contains("storage") {
                "df -h".to_string()
            } else if normalized_query.contains("memory") || normalized_query.contains("oom") {
                "free -m && dmesg | tail -n 30".to_string()
            } else if normalized_query.contains("cpu") || normalized_query.contains("load") || normalized_query.contains("slow") {
                "uptime && top -bn1 | head -n 20".to_string()
            } else {
                "uname -a && uptime".to_string()
            }),
            target_asset_id: primary_asset.map(|asset| asset.asset_id),
            target_asset_name: primary_asset.map(|asset| asset.asset_name.clone()),
            risk_level: "low".to_string(),
            requires_confirmation: false,
            runbook: Some("Capture the output, compare with recent healthy baseline, and only then proceed.".to_string()),
        },
        OpsPlanStep {
            id: "scope".to_string(),
            title: "Confirm affected scope".to_string(),
            description: "Use tags, environment, or owner information to decide whether remediation should be single-host or batch.".to_string(),
            command: None,
            target_asset_id: None,
            target_asset_name: None,
            risk_level: "medium".to_string(),
            requires_confirmation: true,
            runbook: Some("If more than one critical asset is involved, prepare a staged rollout or canary.".to_string()),
        },
        OpsPlanStep {
            id: "review".to_string(),
            title: "Review execution results".to_string(),
            description: "Archive the outputs, update audit notes, and confirm the service returned to a steady state.".to_string(),
            command: None,
            target_asset_id: None,
            target_asset_name: None,
            risk_level: "low".to_string(),
            requires_confirmation: true,
            runbook: Some("Summarize impact, changes made, and any follow-up tasks for the next operator.".to_string()),
        },
    ];

    let summary = if matched_assets.is_empty() {
        format!("No direct asset match for '{}'. Broaden the query or select an asset to continue.", trimmed_query)
    } else {
        format!(
            "Matched {} asset(s) for '{}'. Prioritize read-only inspection first, then move to confirmed remediation.",
            matched_assets.len(),
            trimmed_query
        )
    };

    let review_checklist = vec![
        "Was the target scope confirmed before any write operation?".to_string(),
        "Did the command output get archived for later audit review?".to_string(),
        "Is rollback or follow-up work clearly captured?".to_string(),
    ];

    let sources = matched_assets
        .iter()
        .map(|asset| format!("asset:{} ({})", asset.asset_name, asset.host))
        .collect::<Vec<_>>();

    Ok(OpsConsoleAnswer {
        summary,
        matched_assets,
        status_explanation,
        recommended_checks,
        plan_steps,
        review_checklist,
        sources,
    })
}

#[tauri::command]
pub fn audit_list_events(
    app_handle: AppHandle,
    asset_id: Option<i64>,
    limit: Option<usize>,
) -> Result<Vec<AuditEvent>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(200) as i64;
    let (sql, params_asset): (&str, Option<i64>) = if let Some(asset_id) = asset_id {
        (
            "SELECT id, event_type, asset_id, session_id, job_run_id, title, detail, severity, metadata_json, created_at
             FROM audit_events WHERE asset_id = ?1 ORDER BY created_at DESC LIMIT ?2",
            Some(asset_id),
        )
    } else {
        (
            "SELECT id, event_type, asset_id, session_id, job_run_id, title, detail, severity, metadata_json, created_at
             FROM audit_events ORDER BY created_at DESC LIMIT ?1",
            None,
        )
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = if let Some(asset_id) = params_asset {
        stmt.query_map(params![asset_id, limit], map_audit_event_row)
            .map_err(|e| e.to_string())?
    } else {
        stmt.query_map(params![limit], map_audit_event_row)
            .map_err(|e| e.to_string())?
    };
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn audit_search_events(
    app_handle: AppHandle,
    query: Option<String>,
    severity: Option<String>,
    asset_id: Option<i64>,
    limit: Option<usize>,
) -> Result<Vec<AuditEvent>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut clauses = vec!["1 = 1".to_string()];
    let mut params_vec: Vec<String> = Vec::new();

    if let Some(query) = query.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
        clauses.push("(title LIKE ? OR COALESCE(detail, '') LIKE ? OR COALESCE(metadata_json, '') LIKE ?)".to_string());
        let pattern = format!("%{}%", query);
        params_vec.push(pattern.clone());
        params_vec.push(pattern.clone());
        params_vec.push(pattern);
    }
    if let Some(severity) = severity.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
        clauses.push("severity = ?".to_string());
        params_vec.push(severity.to_string());
    }
    if let Some(asset_id) = asset_id {
        clauses.push("asset_id = ?".to_string());
        params_vec.push(asset_id.to_string());
    }

    let sql = format!(
        "SELECT id, event_type, asset_id, session_id, job_run_id, title, detail, severity, metadata_json, created_at
         FROM audit_events
         WHERE {}
         ORDER BY created_at DESC
         LIMIT {}",
        clauses.join(" AND "),
        limit.unwrap_or(200)
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(params_vec.iter()), map_audit_event_row)
        .map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn audit_create_event(app_handle: AppHandle, event: AuditEvent) -> Result<AuditEvent, String> {
    let id = append_audit_event(
        &app_handle,
        event.event_type.as_str(),
        event.asset_id,
        event.session_id.as_deref(),
        event.job_run_id,
        event.title.as_str(),
        event.detail.as_deref(),
        event.severity.as_str(),
        event.metadata_json.as_deref(),
    )?;
    Ok(AuditEvent {
        id: Some(id),
        created_at: now_ts(),
        ..event
    })
}

#[tauri::command]
pub fn sync_get_state(app_handle: AppHandle) -> Result<SyncState, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, state_key, status, version, endpoint_url, last_synced_at, last_error, metadata_json, updated_at
         FROM sync_state ORDER BY id ASC LIMIT 1",
        [],
        |row| {
            Ok(SyncState {
                id: row.get(0)?,
                state_key: row.get(1)?,
                status: row.get(2)?,
                version: row.get(3)?,
                endpoint_url: row.get(4)?,
                last_synced_at: row.get(5)?,
                last_error: row.get(6)?,
                metadata_json: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sync_get_overview(app_handle: AppHandle) -> Result<SyncOverview, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let state = sync_get_state(app_handle.clone())?;
    let pending_changes: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM sync_change_log WHERE sync_status = 'pending'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let total_changes: i64 = conn
        .query_row("SELECT COUNT(1) FROM sync_change_log", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let last_change_at: Option<i64> = conn
        .query_row(
            "SELECT MAX(created_at) FROM sync_change_log",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let mut services_stmt = conn
        .prepare(
            "SELECT id, service_key, display_name, base_url, auth_mode, auth_token, enabled, metadata_json, created_at, updated_at
             FROM sync_services
             ORDER BY enabled DESC, display_name COLLATE NOCASE ASC",
        )
        .map_err(|e| e.to_string())?;
    let services_rows = services_stmt
        .query_map([], map_sync_service_row)
        .map_err(|e| e.to_string())?;
    let mut services = Vec::new();
    for row in services_rows {
        services.push(row.map_err(|e| e.to_string())?);
    }

    let mut changes_stmt = conn
        .prepare(
            "SELECT id, object_type, object_id, operation, object_version, summary, payload_json, sync_status, service_key, created_at, synced_at
             FROM sync_change_log
             ORDER BY created_at DESC
             LIMIT 30",
        )
        .map_err(|e| e.to_string())?;
    let changes_rows = changes_stmt
        .query_map([], map_sync_change_log_row)
        .map_err(|e| e.to_string())?;
    let mut recent_changes = Vec::new();
    for row in changes_rows {
        recent_changes.push(row.map_err(|e| e.to_string())?);
    }

    let mut versions_stmt = conn
        .prepare(
            "SELECT object_type, COUNT(1), MAX(version)
             FROM sync_object_versions
             GROUP BY object_type
             ORDER BY object_type ASC",
        )
        .map_err(|e| e.to_string())?;
    let version_rows = versions_stmt
        .query_map([], |row| {
            Ok(SyncObjectVersionSummary {
                object_type: row.get(0)?,
                count: row.get(1)?,
                max_version: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut object_version_summary = Vec::new();
    for row in version_rows {
        object_version_summary.push(row.map_err(|e| e.to_string())?);
    }

    Ok(SyncOverview {
        state,
        pending_changes,
        total_changes,
        last_change_at,
        services,
        recent_changes,
        protocol_version: "local-first/v1".to_string(),
        strategy: "append-only change log with object versioning".to_string(),
        object_version_summary,
    })
}

#[tauri::command]
pub fn sync_save_state(app_handle: AppHandle, state: SyncState) -> Result<SyncState, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let updated_at = now_ts();
    conn.execute(
        "INSERT INTO sync_state (id, state_key, status, version, endpoint_url, last_synced_at, last_error, metadata_json, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(state_key) DO UPDATE SET
            status = excluded.status,
            version = excluded.version,
            endpoint_url = excluded.endpoint_url,
            last_synced_at = excluded.last_synced_at,
            last_error = excluded.last_error,
            metadata_json = excluded.metadata_json,
            updated_at = excluded.updated_at",
        params![
            state.id.unwrap_or(1),
            state.state_key,
            state.status,
            state.version,
            state.endpoint_url,
            state.last_synced_at,
            state.last_error,
            state.metadata_json,
            updated_at
        ],
    )
    .map_err(|e| e.to_string())?;
    record_change_log(
        &conn,
        "syncState",
        state.state_key.as_str(),
        "update",
        "Updated sync state",
        Some(json!(state.clone()).to_string()),
        Some("local-first"),
    )?;
    sync_get_state(app_handle)
}

#[tauri::command]
pub fn sync_list_change_log(
    app_handle: AppHandle,
    status: Option<String>,
    object_type: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SyncChangeLogEntry>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut clauses = vec!["1 = 1".to_string()];
    let mut params_vec: Vec<String> = Vec::new();

    if let Some(status) = status.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
        clauses.push("sync_status = ?".to_string());
        params_vec.push(status.to_string());
    }
    if let Some(object_type) = object_type.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
        clauses.push("object_type = ?".to_string());
        params_vec.push(object_type.to_string());
    }

    let sql = format!(
        "SELECT id, object_type, object_id, operation, object_version, summary, payload_json, sync_status, service_key, created_at, synced_at
         FROM sync_change_log
         WHERE {}
         ORDER BY created_at DESC
         LIMIT {}",
        clauses.join(" AND "),
        limit.unwrap_or(200)
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(params_vec.iter()), map_sync_change_log_row)
        .map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn sync_mark_changes_synced(
    app_handle: AppHandle,
    change_ids: Vec<i64>,
    service_key: Option<String>,
) -> Result<usize, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let timestamp = now_ts();
    let mut updated = 0usize;
    for change_id in change_ids {
        updated += conn
            .execute(
                "UPDATE sync_change_log
                 SET sync_status = 'synced', service_key = COALESCE(?2, service_key), synced_at = ?3
                 WHERE id = ?1",
                params![change_id, service_key, timestamp],
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(updated)
}

#[tauri::command]
pub fn sync_list_services(app_handle: AppHandle) -> Result<Vec<SyncServiceConfig>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, service_key, display_name, base_url, auth_mode, auth_token, enabled, metadata_json, created_at, updated_at
             FROM sync_services
             ORDER BY enabled DESC, display_name COLLATE NOCASE ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_sync_service_row)
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn sync_upsert_service(
    app_handle: AppHandle,
    service: SyncServiceConfig,
) -> Result<SyncServiceConfig, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let created_at = if service.created_at == 0 {
        now_ts()
    } else {
        service.created_at
    };
    let updated_at = now_ts();
    conn.execute(
        "INSERT INTO sync_services (
            id, service_key, display_name, base_url, auth_mode, auth_token, enabled, metadata_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(service_key) DO UPDATE SET
            display_name = excluded.display_name,
            base_url = excluded.base_url,
            auth_mode = excluded.auth_mode,
            auth_token = excluded.auth_token,
            enabled = excluded.enabled,
            metadata_json = excluded.metadata_json,
            updated_at = excluded.updated_at",
        params![
            service.id,
            service.service_key,
            service.display_name,
            service.base_url,
            service.auth_mode,
            service.auth_token,
            service.enabled as i64,
            service.metadata_json,
            created_at,
            updated_at
        ],
    )
    .map_err(|e| e.to_string())?;
    record_change_log(
        &conn,
        "syncService",
        service.service_key.as_str(),
        "upsert",
        "Upserted sync service configuration",
        Some(json!(service.clone()).to_string()),
        Some("local-first"),
    )?;
    Ok(SyncServiceConfig {
        created_at,
        updated_at,
        ..service
    })
}

#[tauri::command]
pub fn ai_plan_action(asset: HostAsset, user_request: String) -> Result<String, String> {
    Ok(format!(
        "Plan action for asset '{}' ({}): inspect current state, identify safe commands, require confirmation before write operations. Request: {}",
        asset.name, asset.host, user_request
    ))
}

#[tauri::command]
pub fn ai_explain_state(asset: HostAsset, observed_state: String) -> Result<String, String> {
    Ok(format!(
        "Asset '{}' on platform '{}' is currently observed as: {}. Explain likely causes, impact, and safe next checks.",
        asset.name, asset.platform, observed_state
    ))
}

#[tauri::command]
pub fn ai_generate_runbook(asset: HostAsset, target: String) -> Result<String, String> {
    Ok(format!(
        "Runbook for asset '{}': define pre-checks, read-only validation, guarded remediation, and rollback notes for {}.",
        asset.name, target
    ))
}

#[tauri::command]
pub async fn session_connect_asset(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    asset_id: i64,
    access_endpoint_id: Option<i64>,
    existing_session_id: Option<String>,
    source: Option<String>,
) -> Result<AssetSessionConnectResult, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let (asset, endpoint, credential_ref) = resolve_asset_bundle(&conn, asset_id, access_endpoint_id)?;
    drop(conn);
    let created_at = now_ts();

    let ssh_config = map_connection_from_endpoint(&asset, &endpoint, credential_ref.as_ref());
    let session_id = client::connect(
        app_handle.clone(),
        state.clone(),
        ssh_config,
        existing_session_id,
    )
    .await?;

    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE host_assets SET last_accessed_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![asset_id, created_at],
    )
    .map_err(|e| e.to_string())?;
    {
        let mut clients = state.clients.lock().map_err(|e| e.to_string())?;
        if let Some(client) = clients.get_mut(&session_id) {
            client.asset_id = Some(asset_id);
            client.access_endpoint_id = endpoint.id;
            client.credential_ref_id = endpoint
                .credential_ref_id
                .or_else(|| credential_ref.as_ref().and_then(|item| item.id));
            client.bastion_chain_id = asset.bastion_chain_id.clone();
        }
    }
    let audit_metadata = serde_json::json!({
        "source": source.clone().unwrap_or_else(|| "tree".to_string()),
        "accessEndpointId": endpoint.id,
        "credentialRefId": endpoint.credential_ref_id.or_else(|| credential_ref.as_ref().and_then(|item| item.id)),
    })
    .to_string();
    append_audit_event(
        &app_handle,
        "session.connected",
        Some(asset_id),
        Some(session_id.as_str()),
        None,
        "Connected asset session",
        Some(asset.name.as_str()),
        "info",
        Some(audit_metadata.as_str()),
    )?;

    Ok(AssetSessionConnectResult {
        session_id,
        asset_id,
        asset_name: asset.name,
        created_at,
        env_id: asset.env_id,
        access_endpoint_id: endpoint.id,
        credential_ref_id: endpoint
            .credential_ref_id
            .or_else(|| credential_ref.and_then(|item| item.id)),
        bastion_chain_id: asset.bastion_chain_id,
        risk_level: asset.criticality,
        health_summary: asset.health_summary,
        os_info: asset.platform,
    })
}

#[tauri::command]
pub async fn session_disconnect_asset(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    asset_id: Option<i64>,
) -> Result<(), String> {
    client::disconnect(state, session_id.clone()).await?;
    append_audit_event(
        &app_handle,
        "session.disconnected",
        asset_id,
        Some(session_id.as_str()),
        None,
        "Disconnected asset session",
        None,
        "info",
        None,
    )?;
    Ok(())
}

#[tauri::command]
pub fn session_get_ops_sessions(state: State<'_, AppState>) -> Result<Vec<OpsSession>, String> {
    let clients = state.clients.lock().map_err(|e| e.to_string())?;
    let mut sessions = Vec::new();

    for (session_id, client) in clients.iter() {
        if let Some(asset_id) = client.asset_id {
            sessions.push(OpsSession {
                id: session_id.clone(),
                asset_id,
                asset_name: String::new(),
                created_at: now_ts(),
                access_endpoint_id: client.access_endpoint_id,
                credential_ref_id: client.credential_ref_id,
                bastion_chain_id: client.bastion_chain_id.clone(),
                current_path: Some(".".to_string()),
                risk_level: "medium".to_string(),
                health_summary: None,
                last_job_run_id: None,
            });
        }
    }

    Ok(sessions)
}
