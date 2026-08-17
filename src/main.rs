mod config;
mod db;
mod gemini;
mod ui;
mod utalk;

use base64::Engine;
use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, Method, StatusCode},
    response::Html,
    routing::{any, get},
    Json, Router,
};
use serde::Deserialize;
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
    if state.db.verify_user(&req.username, &req.password) {
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
    let content_obj = &payload["Payload"]["Content"];
    let payload_type = payload["Payload"]["Type"].as_str().unwrap_or_default();

    let msg_obj = if payload_type == "Message" || content_obj["MessageType"].is_string() {
        content_obj
    } else {
        &content_obj["LastMessage"]
    };

    let msg_id = msg_obj["Id"].as_str().unwrap_or_default();
    let source = msg_obj["Source"].as_str().unwrap_or_default();
    let msg_type = msg_obj["MessageType"].as_str().unwrap_or_default();
    let content = msg_obj["Content"].as_str().unwrap_or_default();

    let chat_id = if payload_type == "Chat" {
        content_obj["Id"].as_str().unwrap_or_default()
    } else {
        msg_obj["Chat"]["Id"].as_str().or_else(|| content_obj["Chat"]["Id"].as_str()).unwrap_or_default()
    };

    let contact_name = content_obj["Contact"]["Name"]
        .as_str()
        .or_else(|| msg_obj["Chat"]["Contact"]["Name"].as_str())
        .unwrap_or("Cliente");

    let is_audio = msg_type == "Audio";
    if (source == "Contact" || is_audio) && !chat_id.is_empty() {
        // 1. Checa se o chat já foi transferido no banco local/VPS
        if state.db.is_chat_transferred(chat_id) {
            println!("⏸️ Chat {} já foi transferido para o atendimento humano. IA pausada para este contato.", chat_id);
            return;
        }

        // 2. Checa se o próprio payload da Umbler/uTalk indica que já existe um atendente humano atribuído a esta conversa
        let chat_obj = if payload_type == "Chat" { content_obj } else { &content_obj["Chat"] };
        let has_human_member = chat_obj["OrganizationMember"].is_object()
            || chat_obj["organizationMember"].is_object()
            || chat_obj["Member"].is_object()
            || content_obj["OrganizationMember"].is_object()
            || content_obj["organizationMember"].is_object()
            || chat_obj["MemberId"].as_str().map(|s| !s.is_empty()).unwrap_or(false)
            || chat_obj["memberId"].as_str().map(|s| !s.is_empty()).unwrap_or(false);

        if has_human_member {
            println!("⏸️ Chat {} já possui atendente humano atribuído no uTalk/Umbler. IA pausada automaticamente.", chat_id);
            let _ = state.db.record_transfer(chat_id, "uTalk-Attendant", "Atendente uTalk");
            return;
        }

        println!("🤖 Processando mensagem de '{}' [ChatId: {}, Type: {}, MsgId: {}]", contact_name, chat_id, msg_type, msg_id);

        let cfg_snapshot = state.db.get_config();
        let mut audio_data_tuple: Option<(String, String)> = None;

        let user_prompt = if msg_type == "Text" {
            content.to_string()
        } else if msg_type == "Audio" {
            // Tenta obter a URL direta do payload primeiro, ou faz GET na API do uTalk
            let direct_audio_url = msg_obj["file"]["url"]
                .as_str()
                .or_else(|| msg_obj["File"]["Url"].as_str())
                .or_else(|| msg_obj["media"]["url"].as_str());

            if let Some(audio_url) = direct_audio_url {
                println!("🔊 Baixando áudio diretamente da URL do payload: {}...", audio_url);
                match reqwest::get(audio_url).await {
                    Ok(resp) => {
                        let bytes = resp.bytes().await.unwrap_or_default();
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                        audio_data_tuple = Some(("audio/mp3".to_string(), b64));
                        "[Áudio enviado pelo cliente]".to_string()
                    }
                    Err(e) => {
                        println!("⚠️ Falha no download direto, buscando via API uTalk: {}", e);
                        utalk::fetch_message_audio(
                            &cfg_snapshot.utalk_api_url,
                            &cfg_snapshot.utalk_api_token,
                            &cfg_snapshot.utalk_organization_id,
                            msg_id,
                        ).await.map(|(mime, b64)| {
                            audio_data_tuple = Some((mime, b64));
                            "[Áudio enviado pelo cliente]".to_string()
                        }).unwrap_or_else(|err| {
                            println!("⚠️ Não foi possível baixar mídia de áudio do uTalk: {}", err);
                            format!("O cliente '{}' enviou um áudio.", contact_name)
                        })
                    }
                }
            } else {
                match utalk::fetch_message_audio(
                    &cfg_snapshot.utalk_api_url,
                    &cfg_snapshot.utalk_api_token,
                    &cfg_snapshot.utalk_organization_id,
                    msg_id,
                )
                .await
                {
                    Ok((mime, b64)) => {
                        audio_data_tuple = Some((mime, b64));
                        "[Áudio enviado pelo cliente]".to_string()
                    }
                    Err(err) => {
                        println!("⚠️ Não foi possível baixar mídia de áudio do uTalk: {}", err);
                        format!("O cliente '{}' enviou um áudio.", contact_name)
                    }
                }
            }
        } else if msg_type == "Image" {
            format!("O cliente '{}' enviou uma imagem. Atenda-o cordialmente.", contact_name)
        } else {
            content.to_string()
        };

        if user_prompt.trim().is_empty() && audio_data_tuple.is_none() {
            return;
        }

        let audio_ref = audio_data_tuple.as_ref().map(|(m, b)| (m.as_str(), b.as_str()));

        match gemini::generate_gemini_response(state.db.clone(), &cfg_snapshot, chat_id, &user_prompt, audio_ref).await {
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

                    // Filtra apenas os operadores que estão ONLINE no uTalk no momento
                    if let Ok(online_ids) = utalk::fetch_online_members(
                        &cfg_snapshot.utalk_api_url,
                        &cfg_snapshot.utalk_api_token,
                        &cfg_snapshot.utalk_organization_id,
                    )
                    .await
                    {
                        candidate_ids.retain(|id| online_ids.contains(id));
                    }

                    // DEVE OBRIGATORIAMENTE registrar a transferência para PAUSAR a IA no chat local
                    let target_op_name = if let Some(target_operator_id) = state.db.get_next_rotation_operator(&candidate_ids) {
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

                        let _ = utalk::transfer_chat_to_member(
                            &cfg_snapshot.utalk_api_url,
                            &cfg_snapshot.utalk_api_token,
                            &cfg_snapshot.utalk_organization_id,
                            chat_id,
                            &target_operator_id,
                        ).await;

                        op_name
                    } else {
                        "Equipe de Vendas".to_string()
                    };

                    // Grava obrigatoriamente a transferencia para pausar a IA localmente
                    state.db.record_transfer(chat_id, "transferred", &target_op_name);

                    // Anexa as etiquetas no uTalk (procura por tag com o nome do atendente e/ou 'FAZER ORÇAMENTO')
                    let client = reqwest::Client::new();
                    let tags_url = format!("{}/tags/?organizationId={}", cfg_snapshot.utalk_api_url.trim_end_matches('/'), cfg_snapshot.utalk_organization_id);
                    let mut selected_tag_ids = vec!["aQC8MBYhPaycNeGd".to_string()]; // Tag padrão: FAZER ORÇAMENTO

                    if let Ok(res) = client.get(&tags_url).header("Authorization", format!("Bearer {}", cfg_snapshot.utalk_api_token)).send().await {
                        if let Ok(data) = res.json::<serde_json::Value>().await {
                            let items_empty = vec![];
                            let tags_arr = data.get("items").and_then(|v| v.as_array()).unwrap_or(&items_empty);
                            for t in tags_arr {
                                if let (Some(tid), Some(tname)) = (t.get("id").and_then(|v| v.as_str()), t.get("name").and_then(|v| v.as_str())) {
                                    if tname.to_lowercase().contains(&target_op_name.to_lowercase()) {
                                        selected_tag_ids.push(tid.to_string());
                                        println!("🏷️ Encontrada etiqueta para o atendente '{}': '{}' (ID: {})", target_op_name, tname, tid);
                                    }
                                }
                            }
                        }
                    }

                    let chat_tag_url = format!("{}/chats/{}/?organizationId={}", cfg_snapshot.utalk_api_url.trim_end_matches('/'), chat_id, cfg_snapshot.utalk_organization_id);
                    let tags_payload: Vec<serde_json::Value> = selected_tag_ids.iter().map(|id| serde_json::json!({ "id": id })).collect();
                    let _ = client.put(&chat_tag_url)
                        .header("Authorization", format!("Bearer {}", cfg_snapshot.utalk_api_token))
                        .json(&serde_json::json!({ "tags": tags_payload }))
                        .send().await;
                    println!("🏷️ Chat {} pausado para a IA e etiquetado no uTalk para '{}'!", chat_id, target_op_name);
                }
            }
            Err(err) => {
                println!("❌ Erro ao gerar resposta do Gemini: {}", err);
            }
        }
    }
}

async fn get_chats_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            let chats = state.db.get_all_chats_summary();
            return Ok(Json(serde_json::json!(chats)));
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

async fn delete_chat_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(chat_id): axum::extract::Path<String>,
) -> Result<StatusCode, StatusCode> {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            state.db.delete_chat_messages(&chat_id);
            return Ok(StatusCode::OK);
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

async fn get_admin_users_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            let users = state.db.list_admin_users();
            return Ok(Json(serde_json::json!(users)));
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

#[derive(Deserialize)]
struct AddUserRequest {
    username: String,
    password: String,
}

async fn add_admin_user_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AddUserRequest>,
) -> (StatusCode, Json<Value>) {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            if req.username.trim().is_empty() || req.password.trim().is_empty() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "Usuário e senha são obrigatórios." })),
                );
            }
            match state.db.add_admin_user(req.username.trim(), req.password.trim()) {
                Ok(id) => return (StatusCode::OK, Json(serde_json::json!({ "success": true, "id": id }))),
                Err(err) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": err }))),
            }
        }
    }
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "Não autorizado" })))
}

async fn delete_admin_user_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> (StatusCode, Json<Value>) {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            let logged_user = state.db.get_session_user(&token).unwrap_or_default();
            match state.db.delete_admin_user_with_check(id, &logged_user) {
                Ok(_) => return (StatusCode::OK, Json(serde_json::json!({ "success": true }))),
                Err(err) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": err }))),
            }
        }
    }
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "Não autorizado" })))
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    new_password: String,
}

async fn change_password_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ChangePasswordRequest>,
) -> (StatusCode, Json<Value>) {
    if let Some(token) = extract_token(&headers) {
        if let Some(username) = state.db.get_session_user(&token) {
            if req.new_password.trim().is_empty() {
                return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Nova senha não pode ser vazia." })));
            }
            match state.db.change_user_password(&username, req.new_password.trim()) {
                Ok(_) => return (StatusCode::OK, Json(serde_json::json!({ "success": true }))),
                Err(err) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": err }))),
            }
        }
    }
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "Não autorizado" })))
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
        .route("/api/chats", get(get_chats_handler))
        .route("/api/chats/:chat_id", axum::routing::delete(delete_chat_handler))
        .route("/api/users", get(get_admin_users_handler).post(add_admin_user_handler))
        .route("/api/users/:id", axum::routing::delete(delete_admin_user_handler))
        .route("/api/change-password", axum::routing::post(change_password_handler))
        .route("/webhook", any(handle_webhook))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 uTalk AI Bot Studio rodando com Rodízio de Atendentes + Dashboard em Abas!");
    println!("📊 Dashboard Web de Controle: http://localhost:3000/");
    println!("📍 Endpoint do Webhook: http://localhost:3000/webhook");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
