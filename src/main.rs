mod config;
mod db;
mod gemini;
mod ui;
mod utalk;

use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, Method, StatusCode},
    response::Html,
    routing::{any, get},
    Json, Router,
};
use config::AppConfig;
use db::{Database, SharedDatabase};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    db: SharedDatabase,
}

#[derive(serde::Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(serde::Serialize)]
struct LoginResponse {
    success: bool,
    token: Option<String>,
    message: String,
}

fn extract_token(headers: &HeaderMap) -> Option<String> {
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                return Some(auth_str[7..].trim().to_string());
            }
        }
    }
    if let Some(cookie_header) = headers.get("cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                if parts.len() == 2 && parts[0] == "session_token" {
                    return Some(parts[1].to_string());
                }
            }
        }
    }
    None
}

async fn render_dashboard() -> Html<&'static str> {
    Html(ui::get_dashboard_html())
}

async fn get_logo_asset() -> (HeaderMap, &'static [u8]) {
    let mut headers = HeaderMap::new();
    if let Ok(v) = "image/png".parse() {
        headers.insert("content-type", v);
    }
    if let Ok(v) = "public, max-age=86400".parse() {
        headers.insert("cache-control", v);
    }
    (headers, include_bytes!("../assets/logo.png"))
}

async fn get_banner_asset() -> (HeaderMap, &'static [u8]) {
    let mut headers = HeaderMap::new();
    if let Ok(v) = "image/jpeg".parse() {
        headers.insert("content-type", v);
    }
    if let Ok(v) = "public, max-age=86400".parse() {
        headers.insert("cache-control", v);
    }
    (headers, include_bytes!("../assets/banner.jpg"))
}

async fn login_handler(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> (StatusCode, HeaderMap, Json<LoginResponse>) {
    let cfg = state.db.get_config();
    if req.username == cfg.admin_username && req.password == cfg.admin_password {
        let token = state.db.create_session(&req.username, 24);
        let mut headers = HeaderMap::new();
        let cookie_val = format!("session_token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400", token);
        if let Ok(hdr_val) = cookie_val.parse() {
            headers.insert("set-cookie", hdr_val);
        }
        println!("🔐 Login bem-sucedido para o usuário: {}", req.username);
        (
            StatusCode::OK,
            headers,
            Json(LoginResponse {
                success: true,
                token: Some(token),
                message: "Login efetuado com sucesso!".to_string(),
            }),
        )
    } else {
        println!("⚠️ Tentativa de login incorreta para o usuário: {}", req.username);
        (
            StatusCode::UNAUTHORIZED,
            HeaderMap::new(),
            Json(LoginResponse {
                success: false,
                token: None,
                message: "Usuário ou senha incorretos.".to_string(),
            }),
        )
    }
}

async fn logout_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, HeaderMap, &'static str) {
    if let Some(token) = extract_token(&headers) {
        state.db.delete_session(&token);
    }
    let mut resp_headers = HeaderMap::new();
    if let Ok(hdr_val) = "session_token=; Path=/; HttpOnly; Max-Age=0".parse() {
        resp_headers.insert("set-cookie", hdr_val);
    }
    (StatusCode::OK, resp_headers, "Desconectado")
}

async fn get_config_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AppConfig>, StatusCode> {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            let cfg = state.db.get_config();
            return Ok(Json(cfg));
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

async fn save_config_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(new_config): Json<AppConfig>,
) -> StatusCode {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            state.db.save_config(&new_config);
            println!("💾 Novas configurações salvas no SQLite!");
            return StatusCode::OK;
        }
    }
    StatusCode::UNAUTHORIZED
}

async fn get_operators_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<utalk::UtalkOperator>>, StatusCode> {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            let cfg = state.db.get_config();
            match utalk::fetch_human_operators(
                &cfg.utalk_api_url,
                &cfg.utalk_api_token,
                &cfg.utalk_organization_id,
            )
            .await
            {
                Ok(ops) => return Ok(Json(ops)),
                Err(err) => {
                    println!("❌ Erro ao buscar operadores no uTalk: {}", err);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

async fn get_stats_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            let stats = state.db.get_dashboard_stats();
            return Ok(Json(stats));
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

async fn handle_webhook(
    State(state): State<AppState>,
    method: Method,
    _headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    bytes: Bytes,
) -> (StatusCode, &'static str) {
    println!("\n========================================================");
    println!("📩 NOVO EVENTO DE WEBHOOK RECEBIDO [{}]", method);
    println!("========================================================");

    if !params.is_empty() {
        println!("📍 Query Params: {:?}", params);
    }

    let config_snapshot = state.db.get_config();

    if let Ok(body_str) = std::str::from_utf8(&bytes) {
        let parsed: Result<Value, _> = serde_json::from_str(body_str);
        if let Ok(payload) = parsed {
            if let Ok(pretty) = serde_json::to_string_pretty(&payload) {
                println!("📦 Payload JSON:\n{}", pretty);
            }

            if config_snapshot.bot_enabled {
                let state_clone = state.clone();
                tokio::spawn(async move {
                    process_incoming_webhook(state_clone, payload).await;
                });
            } else {
                println!("⏸️ Robô está PAUSADO no Dashboard. Evento ignorado.");
            }
        } else if !body_str.is_empty() {
            println!("📦 Body Texto:\n{}", body_str);
        }
    }

    println!("========================================================\n");
    (StatusCode::OK, "OK")
}

async fn process_incoming_webhook(state: AppState, payload: Value) {
    let last_msg = &payload["Payload"]["Content"]["LastMessage"];
    let source = last_msg["Source"].as_str().unwrap_or_default();
    let msg_type = last_msg["MessageType"].as_str().unwrap_or_default();
    let content = last_msg["Content"].as_str().unwrap_or_default();

    let chat_id = payload["Payload"]["Content"]["Id"]
        .as_str()
        .or_else(|| last_msg["Chat"]["Id"].as_str())
        .unwrap_or_default();

    let contact_name = payload["Payload"]["Content"]["Contact"]["Name"]
        .as_str()
        .unwrap_or("Cliente");

    if source == "Contact" && !chat_id.is_empty() {
        println!("🤖 Processando mensagem de '{}' [ChatId: {}]", contact_name, chat_id);

        let user_prompt = if msg_type == "Text" {
            content.to_string()
        } else if msg_type == "Audio" {
            format!("O cliente '{}' enviou um áudio. Atenda-o cordialmente.", contact_name)
        } else if msg_type == "Image" {
            format!("O cliente '{}' enviou uma imagem. Atenda-o cordialmente.", contact_name)
        } else {
            content.to_string()
        };

        if user_prompt.trim().is_empty() {
            return;
        }

        let cfg_snapshot = state.db.get_config();

        match gemini::generate_gemini_response(state.db.clone(), &cfg_snapshot, chat_id, &user_prompt).await {
            Ok(mut ai_reply) => {
                println!("✨ Gemini gerou resposta:\n{}", ai_reply);

                let should_transfer = cfg_snapshot.rotation_enabled
                    && !cfg_snapshot.rotation_trigger_keyword.is_empty()
                    && ai_reply.contains(&cfg_snapshot.rotation_trigger_keyword);

                if should_transfer {
                    println!("🔄 Gatilho de Rodízio detectado na resposta do Gemini!");
                    ai_reply = ai_reply.replace(&cfg_snapshot.rotation_trigger_keyword, "").trim().to_string();
                }

                // Envia a mensagem do bot se houver texto restante
                if !ai_reply.is_empty() {
                    let _ = utalk::send_utalk_message(
                        &cfg_snapshot.utalk_api_url,
                        &cfg_snapshot.utalk_api_token,
                        &cfg_snapshot.utalk_organization_id,
                        chat_id,
                        &ai_reply,
                    )
                    .await;
                }

                // Executa a transferência de rodízio se ativada
                if should_transfer {
                    let mut candidate_ids = cfg_snapshot.rotation_operator_ids.clone();

                    // Se a estratégia for apenas operadores online, filtra via uTalk API
                    if cfg_snapshot.rotation_strategy == "online_only" {
                        if let Ok(online_ids) = utalk::fetch_online_members(
                            &cfg_snapshot.utalk_api_url,
                            &cfg_snapshot.utalk_api_token,
                            &cfg_snapshot.utalk_organization_id,
                        )
                        .await
                        {
                            candidate_ids.retain(|id| online_ids.contains(id));
                        }
                    }

                    if let Some(target_operator_id) = state.db.get_next_rotation_operator(&candidate_ids) {
                        // Busca o nome do operador para registro de métrica
                        let op_name = match utalk::fetch_human_operators(
                            &cfg_snapshot.utalk_api_url,
                            &cfg_snapshot.utalk_api_token,
                            &cfg_snapshot.utalk_organization_id,
                        )
                        .await
                        {
                            Ok(ops) => ops
                                .into_iter()
                                .find(|o| o.id == target_operator_id)
                                .map(|o| o.name)
                                .unwrap_or_else(|| target_operator_id.clone()),
                            Err(_) => target_operator_id.clone(),
                        };

                        if let Ok(_) = utalk::transfer_chat_to_member(
                            &cfg_snapshot.utalk_api_url,
                            &cfg_snapshot.utalk_api_token,
                            &cfg_snapshot.utalk_organization_id,
                            chat_id,
                            &target_operator_id,
                        )
                        .await
                        {
                            state.db.record_transfer(chat_id, &target_operator_id, &op_name);
                        }
                    } else {
                        println!("⚠️ Nenhum operador disponível na fila do rodízio para transferência.");
                    }
                }
            }
            Err(err) => {
                println!("❌ Erro ao gerar resposta do Gemini: {}", err);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Inicializa o banco de dados embarcado SQLite com busca FTS5 e timestamps
    let db = Database::open("chat_ai_bot.db").expect("Falha ao inicializar SQLite FTS5");
    let state = AppState {
        db: Arc::new(db),
    };

    let app = Router::new()
        .route("/", get(render_dashboard))
        .route("/assets/logo.png", get(get_logo_asset))
        .route("/assets/banner.png", get(get_banner_asset))
        .route("/assets/banner.jpg", get(get_banner_asset))
        .route("/api/login", axum::routing::post(login_handler))
        .route("/api/logout", axum::routing::post(logout_handler))
        .route("/api/config", get(get_config_handler).post(save_config_handler))
        .route("/api/operators", get(get_operators_handler))
        .route("/api/stats", get(get_stats_handler))
        .route("/webhook", any(handle_webhook))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 uTalk AI Bot Studio rodando com Rodízio de Atendentes + Dashboard em Abas!");
    println!("📊 Dashboard Web de Controle: http://localhost:3000/");
    println!("📍 Endpoint do Webhook: http://localhost:3000/webhook");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
