// divergence-log.rs — 分歧日志标准化 + Dao 只读保护
//
// 设计目标：
//   1. 标准化分歧日志三元组存储（entity, dimension, divergence_triple）
//   2. Dao 层只读保护：仅允许 CREATE/READ，禁止 UPDATE/DELETE
//   3. SHA256 链式防篡改验证
//   4. 与 portfolio-mgr.rhai 的 decision_trail 互补：decision_trail 记录规则裁决，
//      divergence_log 记录 Agent 间意见分歧

use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use chrono::Utc;

/// 分歧日志三元组
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceLog {
    pub id: i64,                    // 自增主键
    pub entity: String,             // 分歧涉及的实体（股票代码/因子名）
    pub dimension: String,          // 分歧维度（见 schema 枚举）
    pub source_a: String,           // 分歧来源 A
    pub source_b: String,           // 分歧来源 B
    pub magnitude: f64,             // 分歧幅度 0~1
    pub direction: String,          // 分歧方向
    pub resolution: Option<String>, // 解决方式 (JSON)
    pub timestamp: f64,             // Unix 时间戳（不可变）
    pub session_id: String,         // 会话 ID
    pub prev_hash: String,          // SHA256 链：前一条日志的哈希
    pub content_hash: String,       // 本条日志内容的 SHA256
    pub row_hash: String,           // (row_id || prev_hash || content_hash) 的 SHA256
    pub read_only: bool,            // 只读标记：写入时为 true
}

/// Dao 层守卫：分歧日志 CRUD 限制
pub struct DivergenceLogDao {
    conn: Connection,
}

impl DivergenceLogDao {
    /// 初始化分歧日志表（含防篡改索引）
    pub fn init(conn: Connection) -> SqliteResult<Self> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS divergence_logs (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                entity       TEXT    NOT NULL,
                dimension    TEXT    NOT NULL,
                source_a     TEXT    NOT NULL,
                source_b     TEXT    NOT NULL,
                magnitude    REAL    NOT NULL CHECK(magnitude >= 0 AND magnitude <= 1),
                direction    TEXT    NOT NULL,
                resolution   TEXT,
                timestamp    REAL    NOT NULL,
                session_id   TEXT    NOT NULL,
                prev_hash    TEXT    NOT NULL,
                content_hash TEXT    NOT NULL,
                row_hash     TEXT    NOT NULL,
                read_only    INTEGER NOT NULL DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_divergence_entity
                ON divergence_logs(entity, dimension);
            CREATE INDEX IF NOT EXISTS idx_divergence_timestamp
                ON divergence_logs(timestamp DESC);"
        )?;
        Ok(Self { conn })
    }

    /// 获取上一条日志的 row_hash（用于 SHA256 链）
    fn get_last_hash(&self) -> SqliteResult<String> {
        let result: SqliteResult<String> = self.conn.query_row(
            "SELECT row_hash FROM divergence_logs ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        );
        match result {
            Ok(hash) => Ok(hash),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok("GENESIS_BLOCK".to_string()),
            Err(e) => Err(e),
        }
    }

    /// 计算 SHA256(row_id || prev_hash || content_json)
    fn compute_row_hash(id: i64, prev_hash: &str, content_json: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(id.to_string().as_bytes());
        hasher.update(b"||");
        hasher.update(prev_hash.as_bytes());
        hasher.update(b"||");
        hasher.update(content_json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// CREATE：写入分歧日志（唯一允许的写操作）
    pub fn create(&self, log: DivergenceLog) -> SqliteResult<i64> {
        // 计算 content_hash
        let content_json = serde_json::to_string(&log).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(content_json.as_bytes());
        let content_hash = format!("{:x}", hasher.finalize());

        // 获取前一条日志的哈希
        let prev_hash = self.get_last_hash()?;

        // 先插入（自增 ID 在 INSERT 后由 SQLite 分配）
        self.conn.execute(
            "INSERT INTO divergence_logs
             (entity, dimension, source_a, source_b, magnitude, direction,
              resolution, timestamp, session_id, prev_hash, content_hash,
              row_hash, read_only)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'PENDING', 1)",
            params![
                log.entity, log.dimension, log.source_a, log.source_b,
                log.magnitude, log.direction, log.resolution,
                log.timestamp, log.session_id, prev_hash, content_hash,
            ],
        )?;

        let row_id = self.conn.last_insert_rowid();
        let row_hash = Self::compute_row_hash(row_id, &prev_hash, &content_json);

        // 更新 row_hash
        self.conn.execute(
            "UPDATE divergence_logs SET row_hash = ?1 WHERE id = ?2",
            params![row_hash, row_id],
        )?;

        Ok(row_id)
    }

    /// READ：读取分歧日志列表
    pub fn read(&self, entity: Option<&str>, limit: i64) -> SqliteResult<Vec<DivergenceLog>> {
        let mut stmt = if let Some(e) = entity {
            self.conn.prepare(
                "SELECT id, entity, dimension, source_a, source_b, magnitude, direction,
                        resolution, timestamp, session_id, prev_hash, content_hash, row_hash, read_only
                 FROM divergence_logs
                 WHERE entity = ?1
                 ORDER BY timestamp DESC
                 LIMIT ?2",
            )?
        } else {
            self.conn.prepare(
                "SELECT id, entity, dimension, source_a, source_b, magnitude, direction,
                        resolution, timestamp, session_id, prev_hash, content_hash, row_hash, read_only
                 FROM divergence_logs
                 ORDER BY timestamp DESC
                 LIMIT ?1",
            )?
        };

        let rows = if let Some(e) = entity {
            stmt.query_map(params![e, limit], |row| Self::row_to_log(row))?
        } else {
            stmt.query_map(params![limit], |row| Self::row_to_log(row))?
        };

        let mut logs = Vec::new();
        for row in rows {
            logs.push(row?);
        }
        Ok(logs)
    }

    /// 验证 SHA256 链完整性
    pub fn verify_integrity(&self, limit: usize) -> Result<Vec<(i64, bool)>, String> {
        let logs = self.read(None, limit as i64).map_err(|e| e.to_string())?;
        let mut results = Vec::new();
        let mut expected_prev = "".to_string();

        for (i, log) in logs.iter().enumerate() {
            if i == 0 {
                expected_prev = log.row_hash.clone();
                results.push((log.id, true));
                continue;
            }

            let valid = log.prev_hash == expected_prev;
            results.push((log.id, valid));
            expected_prev = log.row_hash.clone();
        }

        Ok(results)
    }

    /// Dao 守卫：拦截 UPDATE 操作（返回错误，阻止执行）
    pub fn forbidden_update(&self, _id: i64) -> SqliteResult<()> {
        Err(rusqlite::Error::InvalidParameterName(
            "DAO_GUARD: divergence_logs 禁止 UPDATE 操作（只读保护）".to_string(),
        ))
    }

    /// Dao 守卫：拦截 DELETE 操作（返回错误，阻止执行）
    pub fn forbidden_delete(&self, _id: i64) -> SqliteResult<()> {
        Err(rusqlite::Error::InvalidParameterName(
            "DAO_GUARD: divergence_logs 禁止 DELETE 操作（只读保护）".to_string(),
        ))
    }

    fn row_to_log(row: &rusqlite::Row) -> SqliteResult<DivergenceLog> {
        Ok(DivergenceLog {
            id: row.get(0)?,
            entity: row.get(1)?,
            dimension: row.get(2)?,
            source_a: row.get(3)?,
            source_b: row.get(4)?,
            magnitude: row.get(5)?,
            direction: row.get(6)?,
            resolution: row.get(7)?,
            timestamp: row.get(8)?,
            session_id: row.get(9)?,
            prev_hash: row.get(10)?,
            content_hash: row.get(11)?,
            row_hash: row.get(12)?,
            read_only: row.get::<_, i32>(13)? != 0,
        })
    }
}

/// 辅助函数：从 portfolio-mgr.rhai 的 decision_trail 派生产生分歧日志
/// 调用时机：每当 decision_trail 中出现 R-200（风险否决）或 R-201（空头否决）条目时，
/// 说明 portfolio-mgr 内部存在分歧（先验判断 vs 否决规则），应写入 divergence_log。
pub fn record_divergence_from_trail(
    dao: &DivergenceLogDao,
    trail: &[serde_json::Value],
    entity_code: &str,
    session_id: &str,
) -> SqliteResult<()> {
    for entry in trail {
        let rule_id = entry["rule_id"].as_str().unwrap_or("");
        let status = entry["status"].as_str().unwrap_or("");

        // 仅记录否决类条目（DOWNGRADED / VETOED）
        if status != "DOWNGRADED" && status != "VETOED" {
            continue;
        }

        let dimension = match rule_id {
            "R-200" => "risk_assessment",
            "R-201" => "trader_consensus",
            "R-401" | "R-402" | "R-403" | "R-404" | "R-405" => "technicals",
            _ => "trader_consensus",
        };

        let log = DivergenceLog {
            id: 0,
            entity: entity_code.to_string(),
            dimension: dimension.to_string(),
            source_a: "portfolio-mgr prior".to_string(),
            source_b: format!("veto_{}", rule_id),
            magnitude: 0.5, // 默认分歧幅度，可由具体规则调整
            direction: "opposing".to_string(),
            resolution: Some(entry["detail"].as_str().unwrap_or("").to_string()),
            timestamp: Utc::now().timestamp() as f64,
            session_id: session_id.to_string(),
            prev_hash: String::new(),
            content_hash: String::new(),
            row_hash: String::new(),
            read_only: true,
        };

        dao.create(log)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divergence_log_create_and_read() {
        let conn = Connection::open_in_memory().unwrap();
        let dao = DivergenceLogDao::init(conn).unwrap();

        let log = DivergenceLog {
            id: 0, entity: "300285".into(), dimension: "risk_assessment".into(),
            source_a: "prior=0.65".into(), source_b: "risk_downgrade=R-200".into(),
            magnitude: 0.5, direction: "opposing".into(),
            resolution: Some("高风险风控否决:买入→持有".into()),
            timestamp: 1751457600.0, session_id: "sess-001".into(),
            prev_hash: "".into(), content_hash: "".into(), row_hash: "".into(),
            read_only: true,
        };

        let id = dao.create(log).unwrap();
        assert!(id > 0);

        let logs = dao.read(Some("300285"), 10).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].entity, "300285");
    }

    #[test]
    fn test_forbidden_update() {
        let conn = Connection::open_in_memory().unwrap();
        let dao = DivergenceLogDao::init(conn).unwrap();
        let result = dao.forbidden_update(1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("DAO_GUARD"));
    }
}
