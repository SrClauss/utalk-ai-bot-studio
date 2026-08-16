use crate::config::AppConfig;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone)]
pub struct StoredMessage {
    pub id: i64,
    pub chat_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl Database {
    pub fn open(path: &str) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| format!("Erro ao abrir SQLite: {}", e))?;

        // Ativa modo WAL e FTS5 para alto desempenho e busca textual rápida
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sessions (
                token TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chat_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_messages_chat_time ON messages(chat_id, created_at);

            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                chat_id,
                content,
                content='messages',
                content_rowid='id'
            );

            CREATE TABLE IF NOT EXISTS transfers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chat_id TEXT NOT NULL,
                member_id TEXT NOT NULL,
                member_name TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_transfers_member ON transfers(member_id);
            ",
        )
        .map_err(|e| format!("Erro ao inicializar tabelas SQLite/FTS5: {}", e))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn get_config(&self) -> AppConfig {
        let has_config = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT value FROM settings WHERE key = 'app_config'")
                .ok();

            if let Some(mut stmt) = stmt {
                if let Ok(value_str) = stmt.query_row([], |row| row.get::<_, String>(0)) {
                    if let Ok(config) = serde_json::from_str::<AppConfig>(&value_str) {
                        return config;
                    }
                }
            }
        };

        let default_cfg = AppConfig::default();
        self.save_config(&default_cfg);
        default_cfg
    }

    pub fn save_config(&self, config: &AppConfig) {
        let conn = self.conn.lock().unwrap();
        if let Ok(json_str) = serde_json::to_string_pretty(config) {
            let _ = conn.execute(
                "INSERT INTO settings (key, value) VALUES ('app_config', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = ?1",
                params![json_str],
            );
            println!("💾 Configurações salvas no SQLite!");
        }
    }

    // --- MENSAGENS E ANÁLISE DE TEMPO DA CONVERSA ---
    pub fn save_message(&self, chat_id: &str, role: &str, content: &str) -> DateTime<Utc> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        let now_iso = now.to_rfc3339();

        let _ = conn.execute(
            "INSERT INTO messages (chat_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![chat_id, role, content, now_iso],
        );

        now
    }

    /// Analisa o tempo decorrido desde a última mensagem e retorna o histórico para o Gemini
    /// Se houver um intervalo maior que `session_gap_hours` (ex: 24h ou 10 dias), o sistema insere uma nota explicativa.
    pub fn get_chat_context_for_ai(&self, chat_id: &str, session_gap_hours: i64) -> (Vec<Value>, Option<i64>) {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT role, content, created_at FROM messages WHERE chat_id = ?1 ORDER BY id DESC LIMIT 20")
            .unwrap();

        let mut rows = stmt
            .query_map(params![chat_id], |row| {
                let role: String = row.get(0)?;
                let content: String = row.get(1)?;
                let created_at: String = row.get(2)?;
                Ok((role, content, created_at))
            })
            .unwrap();

        let mut raw_msgs = vec![];
        while let Some(Ok(row)) = rows.next() {
            raw_msgs.push(row);
        }

        raw_msgs.reverse(); // Coloca em ordem cronológica (mais antiga para mais recente)

        let mut contents = vec![];
        let mut hours_since_last: Option<i64> = None;

        if !raw_msgs.is_empty() {
            let now = Utc::now();
            let last_msg_time = DateTime::parse_from_rfc3339(&raw_msgs.last().unwrap().2)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or(now);

            let diff = now.signed_duration_since(last_msg_time);
            let hours = diff.num_hours();
            hours_since_last = Some(hours);

            // Se o intervalo entre a última mensagem e agora for grande (ex: > 24 horas)
            if hours >= session_gap_hours {
                let days = diff.num_days();
                let time_note = format!(
                    "[AVISO DO SISTEMA: Se passaram {} dias ({} horas) desde a última conversa com este cliente. Trate como um novo atendimento/contato, mantendo cordialidade.]",
                    days, hours
                );

                contents.push(json!({
                    "role": "user",
                    "parts": [{ "text": time_note }]
                }));
            }

            for (role, text, _) in raw_msgs {
                contents.push(json!({
                    "role": role,
                    "parts": [{ "text": text }]
                }));
            }
        }

        (contents, hours_since_last)
    }

    pub fn get_all_chats_summary(&self) -> Vec<Value> {
        let conn = self.conn.lock().unwrap();
        let mut results = vec![];

        let sql = "SELECT chat_id, COUNT(*) as msg_count, MAX(created_at) as last_msg FROM messages GROUP BY chat_id ORDER BY last_msg DESC";
        if let Ok(mut stmt) = conn.prepare(sql) {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok(json!({
                    "chat_id": row.get::<_, String>(0)?,
                    "msg_count": row.get::<_, i64>(1)?,
                    "last_msg": row.get::<_, String>(2)?
                }))
            }) {
                for r in rows.flatten() {
                    results.push(r);
                }
            }
        }
        results
    }

    pub fn delete_chat_messages(&self, chat_id: &str) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute("DELETE FROM messages WHERE chat_id = ?1", params![chat_id]);
        let _ = conn.execute("DELETE FROM messages_fts WHERE chat_id = ?1", params![chat_id]);
        let _ = conn.execute("DELETE FROM transfers WHERE chat_id = ?1", params![chat_id]);
        println!("🗑️ Histórico de chat e status de transferência deletados do BD local: {}", chat_id);
    }

    pub fn is_chat_transferred(&self, chat_id: &str) -> bool {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare("SELECT COUNT(*) FROM transfers WHERE chat_id = ?1") {
            Ok(s) => s,
            Err(_) => return false,
        };
        let count: i64 = stmt.query_row(params![chat_id], |row| row.get(0)).unwrap_or(0);
        count > 0
    }

    // --- BUSCA TEXTUAL COMPLETA (FULL-TEXT SEARCH FTS5) ---
    pub fn search_full_text(&self, query_text: &str, target_chat_id: Option<&str>) -> Vec<Value> {
        let conn = self.conn.lock().unwrap();

        let sql = if target_chat_id.is_some() {
            "SELECT m.chat_id, m.role, m.content, m.created_at
             FROM messages_fts fts
             JOIN messages m ON fts.rowid = m.id
             WHERE fts.content MATCH ?1 AND m.chat_id = ?2
             ORDER BY m.id DESC LIMIT 10"
        } else {
            "SELECT m.chat_id, m.role, m.content, m.created_at
             FROM messages_fts fts
             JOIN messages m ON fts.rowid = m.id
             WHERE fts.content MATCH ?1
             ORDER BY m.id DESC LIMIT 10"
        };

        let mut results = vec![];

        if let Some(c_id) = target_chat_id {
            if let Ok(mut stmt) = conn.prepare(sql) {
                if let Ok(rows) = stmt.query_map(params![query_text, c_id], |row| {
                    Ok(json!({
                        "chat_id": row.get::<_, String>(0)?,
                        "role": row.get::<_, String>(1)?,
                        "content": row.get::<_, String>(2)?,
                        "timestamp": row.get::<_, String>(3)?
                    }))
                }) {
                    for r in rows.flatten() {
                        results.push(r);
                    }
                }
            }
        } else if let Ok(mut stmt) = conn.prepare(sql) {
            if let Ok(rows) = stmt.query_map(params![query_text], |row| {
                Ok(json!({
                    "chat_id": row.get::<_, String>(0)?,
                    "role": row.get::<_, String>(1)?,
                    "content": row.get::<_, String>(2)?,
                    "timestamp": row.get::<_, String>(3)?
                }))
            }) {
                for r in rows.flatten() {
                    results.push(r);
                }
            }
        }

        println!("🔍 Busca textual FTS5 por '{}': {} resultados encontrados", query_text, results.len());
        results
    }

    // --- GERENCIAMENTO DE SESSÕES & TOKENS ---
    pub fn create_session(&self, username: &str, duration_hours: i64) -> String {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        let expires = now + chrono::Duration::hours(duration_hours);
        
        let token = format!(
            "tok_{:x}_{:x}",
            now.timestamp_nanos_opt().unwrap_or_default(),
            chrono::Utc::now().timestamp_micros() % 1_000_000
        );

        let _ = conn.execute(
            "INSERT INTO sessions (token, username, created_at, expires_at) VALUES (?1, ?2, ?3, ?4)",
            params![token, username, now.to_rfc3339(), expires.to_rfc3339()],
        );

        token
    }

    pub fn validate_session(&self, token: &str) -> bool {
        let conn = self.conn.lock().unwrap();
        let now_iso = Utc::now().to_rfc3339();

        let mut stmt = match conn.prepare("SELECT username FROM sessions WHERE token = ?1 AND expires_at > ?2") {
            Ok(s) => s,
            Err(_) => return false,
        };

        stmt.exists(params![token, now_iso]).unwrap_or(false)
    }

    pub fn delete_session(&self, token: &str) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute("DELETE FROM sessions WHERE token = ?1", params![token]);
    }

    // --- SISTEMA DE RODÍZIO & MÉTRICAS ---
    pub fn record_transfer(&self, chat_id: &str, member_id: &str, member_name: &str) {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let _ = conn.execute(
            "INSERT INTO transfers (chat_id, member_id, member_name, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![chat_id, member_id, member_name, now],
        );
        println!("📊 Transferência gravada: Chat {} -> {} ({})", chat_id, member_name, member_id);
    }

    pub fn get_next_rotation_operator(&self, operator_ids: &[String]) -> Option<String> {
        if operator_ids.is_empty() {
            return None;
        }

        let conn = self.conn.lock().unwrap();
        
        // Pega o índice do último atendente acionado
        let last_idx: usize = {
            let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = 'rotation_last_index'").ok();
            if let Some(mut stmt) = stmt {
                if let Ok(val_str) = stmt.query_row([], |row| row.get::<_, String>(0)) {
                    val_str.parse().unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            }
        };

        let next_idx = (last_idx + 1) % operator_ids.len();
        let selected_id = operator_ids[next_idx].clone();

        let _ = conn.execute(
            "INSERT INTO settings (key, value) VALUES ('rotation_last_index', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1",
            params![next_idx.to_string()],
        );

        Some(selected_id)
    }

    pub fn get_dashboard_stats(&self) -> Value {
        let conn = self.conn.lock().unwrap();

        let total_messages: i64 = conn
            .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
            .unwrap_or(0);

        let total_transfers: i64 = conn
            .query_row("SELECT COUNT(*) FROM transfers", [], |row| row.get(0))
            .unwrap_or(0);

        let mut transfers_by_operator = Vec::new();
        if let Ok(mut stmt) = conn.prepare("SELECT member_id, member_name, COUNT(*) as cnt FROM transfers GROUP BY member_id ORDER BY cnt DESC") {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok(json!({
                    "member_id": row.get::<_, String>(0)?,
                    "member_name": row.get::<_, String>(1)?,
                    "count": row.get::<_, i64>(2)?
                }))
            }) {
                for r in rows.flatten() {
                    transfers_by_operator.push(r);
                }
            }
        }

        let mut recent_transfers = Vec::new();
        if let Ok(mut stmt) = conn.prepare("SELECT chat_id, member_id, member_name, created_at FROM transfers ORDER BY id DESC LIMIT 10") {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok(json!({
                    "chat_id": row.get::<_, String>(0)?,
                    "member_id": row.get::<_, String>(1)?,
                    "member_name": row.get::<_, String>(2)?,
                    "created_at": row.get::<_, String>(3)?
                }))
            }) {
                for r in rows.flatten() {
                    recent_transfers.push(r);
                }
            }
        }

        json!({
            "total_messages": total_messages,
            "total_transfers": total_transfers,
            "transfers_by_operator": transfers_by_operator,
            "recent_transfers": recent_transfers
        })
    }
}

pub type SharedDatabase = Arc<Database>;
