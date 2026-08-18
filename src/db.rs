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

            CREATE TABLE IF NOT EXISTS admin_users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL,
                password TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS chat_stages (
                chat_id TEXT PRIMARY KEY,
                current_stage TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS stage_config (
                stage_key TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                text_message TEXT NOT NULL,
                audio_url TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS audio_bank (
                key TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                text_message TEXT NOT NULL,
                audio_url TEXT NOT NULL
            );
            ",
        )
        .map_err(|e| format!("Erro ao inicializar tabelas SQLite/FTS5: {}", e))?;

        // Inicializar usuario admin padrao se tabela vazia
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM admin_users", [], |row| row.get(0))
            .unwrap_or(0);

        if count == 0 {
            let _ = conn.execute(
                "INSERT INTO admin_users (username, password, created_at) VALUES ('admin', 'admin123', datetime('now'))",
                [],
            );
        }

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

    pub fn reset_chat_state(&self, chat_id: &str) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute("DELETE FROM messages WHERE chat_id = ?1", params![chat_id]);
        let _ = conn.execute("DELETE FROM messages_fts WHERE chat_id = ?1", params![chat_id]);
        let _ = conn.execute("DELETE FROM transfers WHERE chat_id = ?1", params![chat_id]);
        let _ = conn.execute("DELETE FROM sessions WHERE chat_id = ?1 OR token = ?1", params![chat_id]);
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

    pub fn get_session_user(&self, token: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        let now_iso = Utc::now().to_rfc3339();

        let mut stmt = conn.prepare("SELECT username FROM sessions WHERE token = ?1 AND expires_at > ?2").ok()?;
        stmt.query_row(params![token, now_iso], |row| row.get::<_, String>(0)).ok()
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

    pub fn save_synced_webhooks(&self, json_val: &Value) {
        let conn = self.conn.lock().unwrap();
        let val_str = serde_json::to_string(json_val).unwrap_or_default();
        let _ = conn.execute(
            "INSERT INTO settings (key, value) VALUES ('synced_utalk_webhooks', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1",
            params![val_str],
        );
    }

    pub fn get_synced_webhooks(&self) -> Value {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare("SELECT value FROM settings WHERE key = 'synced_utalk_webhooks'") {
            Ok(s) => s,
            Err(_) => return Value::Array(vec![]),
        };
        if let Ok(val_str) = stmt.query_row([], |row| row.get::<_, String>(0)) {
            serde_json::from_str(&val_str).unwrap_or(Value::Array(vec![]))
        } else {
            Value::Array(vec![])
        }
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

    pub fn verify_user(&self, username: &str, password: &str) -> bool {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare("SELECT password FROM admin_users WHERE username = ?1") {
            Ok(s) => s,
            Err(_) => return false,
        };

        if let Ok(db_password) = stmt.query_row([username], |row| row.get::<_, String>(0)) {
            db_password == password
        } else {
            // Fallback para configuracao em memoria
            let cfg = self.get_config();
            username == cfg.admin_username && password == cfg.admin_password
        }
    }

    pub fn list_admin_users(&self) -> Vec<serde_json::Value> {
        let conn = self.conn.lock().unwrap();
        let mut users = Vec::new();
        if let Ok(mut stmt) = conn.prepare("SELECT id, username, created_at FROM admin_users ORDER BY id ASC") {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, i64>(0)?,
                    "username": row.get::<_, String>(1)?,
                    "created_at": row.get::<_, String>(2)?
                }))
            }) {
                for r in rows.flatten() {
                    users.push(r);
                }
            }
        }
        users
    }

    pub fn add_admin_user(&self, username: &str, password: &str) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO admin_users (username, password, created_at) VALUES (?1, ?2, datetime('now'))",
            [username, password],
        )
        .map_err(|e| format!("Erro ao adicionar administrador: {}", e))?;
        Ok(conn.last_insert_rowid())
    }

    pub fn delete_admin_user_with_check(&self, id: i64, current_logged_user: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();

        let target_username: String = match conn.query_row(
            "SELECT username FROM admin_users WHERE id = ?1",
            [id],
            |row| row.get(0),
        ) {
            Ok(u) => u,
            Err(_) => return Err("Administrador não encontrado.".to_string()),
        };

        if target_username.to_lowercase() == current_logged_user.to_lowercase() {
            return Err("Você não pode excluir o seu próprio usuário enquanto estiver logado.".to_string());
        }

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM admin_users", [], |row| row.get(0))
            .unwrap_or(0);

        if count <= 1 {
            return Err("Não é possível excluir o único administrador do sistema.".to_string());
        }

        conn.execute("DELETE FROM admin_users WHERE id = ?1", [id])
            .map_err(|e| format!("Erro ao remover administrador: {}", e))?;
        Ok(())
    }

    pub fn change_user_password(&self, username: &str, new_password: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE admin_users SET password = ?1 WHERE username = ?2",
            [new_password, username],
        )
        .map_err(|e| format!("Erro ao alterar senha: {}", e))?;
        Ok(())
    }

    pub fn get_chat_stage(&self, chat_id: &str) -> String {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT current_stage FROM chat_stages WHERE chat_id = ?1",
            [chat_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "STAGE_1".to_string())
    }

    pub fn set_chat_stage(&self, chat_id: &str, stage: &str) {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let _ = conn.execute(
            "INSERT INTO chat_stages (chat_id, current_stage, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(chat_id) DO UPDATE SET current_stage = excluded.current_stage, updated_at = excluded.updated_at",
            params![chat_id, stage, now],
        );
    }

    pub fn get_stage_config(&self, stage_key: &str) -> Option<Value> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT stage_key, title, text_message, audio_url FROM stage_config WHERE stage_key = ?1",
            [stage_key],
            |row| {
                Ok(json!({
                    "stage_key": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "text_message": row.get::<_, String>(2)?,
                    "audio_url": row.get::<_, String>(3)?,
                }))
            },
        )
        .ok()
    }

    pub fn save_stage_config(&self, stage_key: &str, title: &str, text_message: &str, audio_url: &str) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "INSERT INTO stage_config (stage_key, title, text_message, audio_url) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(stage_key) DO UPDATE SET title = excluded.title, text_message = excluded.text_message, audio_url = excluded.audio_url",
            params![stage_key, title, text_message, audio_url],
        );
    }

    pub fn get_all_stage_configs(&self) -> Value {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare("SELECT stage_key, title, text_message, audio_url FROM stage_config") {
            Ok(s) => s,
            Err(_) => return json!([]),
        };

        let rows = stmt.query_map([], |row| {
            Ok(json!({
                "stage_key": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "text_message": row.get::<_, String>(2)?,
                "audio_url": row.get::<_, String>(3)?,
            }))
        });

        let mut list = Vec::new();
        if let Ok(iter) = rows {
            for item in iter.flatten() {
                list.push(item);
            }
        }

        let defaults = vec![
            (
                "STAGE_1",
                "1. Apresentação e Fonte de Água",
                "Olá, bom dia! Sou o Leandro da equipe da Tubarão Bombas. É um prazer falar com você. Estou aqui para ajudar a encontrar a bomba solar ideal para o seu projeto. Para começarmos, você poderia me dizer qual é a fonte de água que você vai utilizar, por exemplo, poço artesiano, rio, represa ou cacimba?",
                "/assets/saudacao_bom_dia.mp3",
            ),
            (
                "STAGE_2",
                "2. Profundidade/Altura e Distância",
                "Entendido! E você saberia me informar qual é a profundidade ou altura da água e a distância até a caixa d água ou reservatório?",
                "/assets/profundidade_distancia.mp3",
            ),
            (
                "STAGE_3",
                "3. Vazão em Litros (Dia ou Hora)",
                "Perfeito! E quantos litros de água você precisa abastecer por dia ou por hora?",
                "/assets/vazao_agua.mp3",
            ),
            (
                "STAGE_TRANSFER",
                "4. Resumo e Encaminhamento ao Humano",
                "Excelente! Já compilei todas as informações do seu projeto e estou encaminhando agora mesmo para a nossa equipe de especialistas. Um de nossos atendentes vai te chamar em instantes para finalizar o seu orçamento. Muito obrigado!",
                "/assets/encaminhamento_resumo.mp3",
            ),
        ];

        for (key, title, txt, audio) in defaults {
            let _ = conn.execute(
                "INSERT INTO stage_config (stage_key, title, text_message, audio_url) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(stage_key) DO UPDATE SET 
                 audio_url = CASE WHEN stage_config.audio_url IS NULL OR stage_config.audio_url = '' THEN excluded.audio_url ELSE stage_config.audio_url END",
                params![key, title, txt, audio],
            );
        }

        let mut stmt = match conn.prepare("SELECT stage_key, title, text_message, audio_url FROM stage_config ORDER BY stage_key") {
            Ok(s) => s,
            Err(_) => return json!([]),
        };

        let rows = stmt.query_map([], |row| {
            Ok(json!({
                "stage_key": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "text_message": row.get::<_, String>(2)?,
                "audio_url": row.get::<_, String>(3)?,
            }))
        });

        let mut list = Vec::new();
        if let Ok(iter) = rows {
            for item in iter.flatten() {
                list.push(item);
            }
        }

        json!(list)
    }

    pub fn get_all_audio_bank_items(&self) -> Value {
        let conn = self.conn.lock().unwrap();

        let defaults = vec![
            (
                "saudacao_bom_dia",
                "Saudação Bom Dia e Fonte de Água",
                "Usado no início do atendimento (período da manhã) para dar bom dia e perguntar qual é a fonte de água.",
                "Olá, bom dia! Sou o Leandro da equipe da Tubarão Bombas. É um prazer falar com você. Estou aqui para ajudar a encontrar a bomba solar ideal para o seu projeto. Para começarmos, você poderia me dizer qual é a fonte de água que você vai utilizar, por exemplo, poço artesiano, rio, represa ou cacimba?",
                "/assets/saudacao_bom_dia.mp3",
            ),
            (
                "saudacao_boa_tarde",
                "Saudação Boa Tarde e Fonte de Água",
                "Usado no início do atendimento (período da tarde) para dar boa tarde e perguntar qual é a fonte de água.",
                "Olá, boa tarde! Sou o Leandro da equipe da Tubarão Bombas. É um prazer falar com você. Estou aqui para ajudar a encontrar a bomba solar ideal para o seu projeto. Para começarmos, você poderia me dizer qual é a fonte de água que você vai utilizar, por exemplo, poço artesiano, rio, represa ou cacimba?",
                "/assets/saudacao_boa_tarde.mp3",
            ),
            (
                "saudacao_boa_noite",
                "Saudação Boa Noite e Fonte de Água",
                "Usado no início do atendimento (período da noite) para dar boa noite e perguntar qual é a fonte de água.",
                "Olá, boa noite! Sou o Leandro da equipe da Tubarão Bombas. É um prazer falar com você. Estou aqui para ajudar a encontrar a bomba solar ideal para o seu projeto. Para começarmos, você poderia me dizer qual é a fonte de água que você vai utilizar, por exemplo, poço artesiano, rio, represa ou cacimba?",
                "/assets/saudacao_boa_noite.mp3",
            ),
            (
                "poco_cacimba_detalhes",
                "Poço Artesiano ou Cacimba - Profundidade e Distância",
                "Usado especificamente quando o cliente informa que a fonte é poço artesiano ou cacimba para perguntar a profundidade do poço e a distância.",
                "Entendido! Para poço artesiano ou cacimba, você saberia me informar qual é a profundidade do poço e a distância até a caixa d água ou reservatório?",
                "/assets/poco_cacimba_detalhes.mp3",
            ),
            (
                "rio_represa_detalhes",
                "Rio, Represa ou Açude - Altura e Distância",
                "Usado especificamente quando a fonte de água é rio, represa ou açude para perguntar a altura de subida e a distância.",
                "Entendido! Para captação em rio, represa ou açude, você saberia me informar qual é a altura de subida do terreno e a distância até a caixa d água ou reservatório?",
                "/assets/rio_represa_detalhes.mp3",
            ),
            (
                "profundidade_distancia",
                "Profundidade / Altura da Água e Distância (Geral)",
                "Usado como alternativa geral para perguntar a profundidade/altura da água e a distância até o reservatório.",
                "Entendido! E você saberia me informar qual é a profundidade ou altura da água e a distância até a caixa d água ou reservatório?",
                "/assets/profundidade_distancia.mp3",
            ),
            (
                "vazao_agua",
                "Vazão em Litros por Dia ou por Hora",
                "Usado após o cliente informar a profundidade e a distância para perguntar a vazão em litros por dia ou hora.",
                "Perfeito! E quantos litros de água você precisa abastecer por dia ou por hora?",
                "/assets/vazao_agua.mp3",
            ),
            (
                "encaminhamento_resumo",
                "Encaminhamento e Transferência ao Especialista",
                "Usado ao concluir a coleta de dados, informando que os dados foram compilados e encaminhados para a equipe de especialistas.",
                "Excelente! Já compilei todas as informações do seu projeto e estou encaminhando agora mesmo para a nossa equipe de especialistas. Um de nossos atendentes vai te chamar em instantes para finalizar o seu orçamento. Muito obrigado!",
                "/assets/encaminhamento_resumo.mp3",
            ),
        ];

        for (key, title, desc, txt, audio) in defaults {
            let _ = conn.execute(
                "INSERT INTO audio_bank (key, title, description, text_message, audio_url) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(key) DO UPDATE SET 
                 description = excluded.description,
                 text_message = excluded.text_message,
                 audio_url = CASE WHEN audio_bank.audio_url IS NULL OR audio_bank.audio_url = '' THEN excluded.audio_url ELSE audio_bank.audio_url END",
                params![key, title, desc, txt, audio],
            );
        }

        let mut stmt = match conn.prepare("SELECT key, title, description, text_message, audio_url FROM audio_bank ORDER BY key") {
            Ok(s) => s,
            Err(_) => return json!([]),
        };

        let rows = stmt.query_map([], |row| {
            Ok(json!({
                "key": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "description": row.get::<_, String>(2)?,
                "text_message": row.get::<_, String>(3)?,
                "audio_url": row.get::<_, String>(4)?,
            }))
        });

        let mut list = Vec::new();
        if let Ok(iter) = rows {
            for item in iter.flatten() {
                list.push(item);
            }
        }

        json!(list)
    }

    pub fn save_audio_bank_item(&self, item: &Value) -> bool {
        let conn = self.conn.lock().unwrap();
        let key = item["key"].as_str().unwrap_or_default();
        let title = item["title"].as_str().unwrap_or_default();
        let description = item["description"].as_str().unwrap_or_default();
        let text_message = item["text_message"].as_str().unwrap_or_default();
        let audio_url = item["audio_url"].as_str().unwrap_or_default();

        if key.is_empty() { return false; }

        let res = conn.execute(
            "INSERT INTO audio_bank (key, title, description, text_message, audio_url) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(key) DO UPDATE SET title = ?2, description = ?3, text_message = ?4, audio_url = ?5",
            params![key, title, description, text_message, audio_url],
        );
        res.is_ok()
    }
}

pub type SharedDatabase = Arc<Database>;
