mod config;
mod db;
mod deepseek;
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
            let content_obj = &payload["Payload"]["Content"];
            let payload_type = payload["Payload"]["Type"].as_str().unwrap_or_default();
            let msg_obj = if payload_type == "Message" || content_obj["MessageType"].is_string() {
                content_obj
            } else {
                &content_obj["LastMessage"]
            };

            let chat_obj = if payload_type == "Chat" { content_obj } else { &content_obj["Chat"] };
            let channel_name = chat_obj["Channel"]["Name"].as_str().or_else(|| content_obj["Channel"]["Name"].as_str()).unwrap_or("Canal Geral");
            let channel_id = chat_obj["Channel"]["Id"].as_str().or_else(|| content_obj["Channel"]["Id"].as_str()).unwrap_or("N/A");
            
            let contact_name = content_obj["Contact"]["Name"].as_str().or_else(|| msg_obj["Chat"]["Contact"]["Name"].as_str()).unwrap_or("Cliente");
            let phone = content_obj["Contact"]["PhoneNumber"].as_str().or_else(|| msg_obj["Chat"]["Contact"]["PhoneNumber"].as_str()).unwrap_or("N/A");
            let text = msg_obj["Content"].as_str().unwrap_or("[Mídia / Áudio / Sistema]");

            let tag_names: Vec<String> = chat_obj["Tags"].as_array()
                .or_else(|| content_obj["Contact"]["Tags"].as_array())
                .or_else(|| content_obj["Tags"].as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.get("name").or_else(|| t.get("Name")).and_then(|n| n.as_str()).map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let tags_info = if tag_names.is_empty() { "Nenhuma".to_string() } else { tag_names.join(", ") };

            let has_human_member = chat_obj["OrganizationMember"].is_object()
                || chat_obj["organizationMember"].is_object()
                || chat_obj["Member"].is_object()
                || content_obj["OrganizationMember"].is_object()
                || content_obj["organizationMember"].is_object()
                || chat_obj["MemberId"].as_str().map(|s| !s.is_empty()).unwrap_or(false)
                || chat_obj["memberId"].as_str().map(|s| !s.is_empty()).unwrap_or(false);

            let attendant_info = if has_human_member { "👤 Atendente Humano Atribuído no uTalk" } else { "✅ Nenhum (Chat Livre)" };

            // Salvar o último payload bruto em arquivo na VPS para inspeção
            if let Ok(pretty_json) = serde_json::to_string_pretty(&payload) {
                let _ = std::fs::write("/tmp/last_webhook.json", &pretty_json);
            }

            println!("\n========================================================");
            println!("📩 MENSAGEM RECEBIDA NO WEBHOOK [uTalk]");
            println!("📡 Canal Receptor  : {} (ID: {})", channel_name, channel_id);
            println!("👤 Cliente / Remet. : {} ({})", contact_name, phone);
            println!("💬 Conteúdo Texto  : \"{}\"", text);
            println!("🏷️ Tags no uTalk    : {}", tags_info);
            println!("👤 Status Atendente: {}", attendant_info);

            // 🎯 TRAVA ESTRITA DE CANAIS SINCRONIZADOS DO UTALK:
            let synced_webhooks_json = state.db.get_synced_webhooks();
            let mut allowed_channels = Vec::new();
            if let Some(arr) = synced_webhooks_json.as_array() {
                for item in arr {
                    let is_paused = item["paused"].as_bool().unwrap_or(false);
                    if !is_paused {
                        if let Some(chans) = item["for_channels"].as_array().or_else(|| item["forChannels"].as_array()) {
                            for ch in chans {
                                if let Some(s) = ch.as_str() {
                                    allowed_channels.push(s.to_string());
                                }
                            }
                        }
                    }
                }
            }

            let channel_allowed = allowed_channels.is_empty() || allowed_channels.contains(&channel_id.to_string());

            let target_chat_id = if payload_type == "Chat" {
                content_obj["Id"].as_str().unwrap_or_default()
            } else {
                msg_obj["Chat"]["Id"].as_str().or_else(|| content_obj["Chat"]["Id"].as_str()).unwrap_or_default()
            };

            let is_vps_transferred = state.db.is_chat_transferred(target_chat_id);

            // Sanitiza o número do remetente para verificação de permissão de teste
            let clean_phone: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
            let is_tester = config_snapshot.test_allowed_phones.iter().any(|p| {
                let clean_p: String = p.chars().filter(|c| c.is_ascii_digit()).collect();
                !clean_p.is_empty() && (clean_phone.ends_with(&clean_p) || clean_p.ends_with(&clean_phone))
            });

            if config_snapshot.bot_enabled {
                if !channel_allowed {
                    println!("⚡ Decisão da IA    : ⏸️ [IGNORADO] Canal '{}' (ID: {}) não está na lista de canais permitidos do Webhook.", channel_name, channel_id);
                } else if config_snapshot.test_mode_enabled && !is_tester {
                    println!("⚡ Decisão da IA    : ⏸️ [MODO DE TESTE ATIVO] Mensagem ignorada pois o remetente '{}' não está na lista VIP de testes (Claus/Lucas).", phone);
                } else if !is_tester && (has_human_member || is_vps_transferred) {
                    println!("⚡ Decisão da IA    : ⏸️ [SILÊNCIOSO] Atendente humano atribuído no uTalk/VPS (HasHumanMember: {}). IA pausada.", has_human_member);
                } else {
                    if is_tester {
                        println!("🧪 Decisão da IA    : 🧪 [MODO TESTE VIP] Atendimento FORÇADO para o testador '{}' (Ignorando travas de atendente do uTalk).", phone);
                    } else {
                        println!("⚡ Decisão da IA    : 🤖 [LIGADO] Processando com DeepSeek...");
                    }
                    let state_clone = state.clone();
                    tokio::spawn(async move {
                        process_incoming_webhook(state_clone, payload).await;
                    });
                }
            } else {
                println!("⚡ Decisão da IA    : ⏸️ [PAUSADO] Robô inativo no Dashboard.");
            }
            println!("========================================================\n");
        } else if !body_str.is_empty() {
            println!("📦 Body Texto sem formatação:\n{}", body_str);
        }
    }

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

    let chat_obj = if payload_type == "Chat" { content_obj } else { &content_obj["Chat"] };
    let created_at_utc = chat_obj["CreatedAtUTC"].as_str().unwrap_or_default();
    let event_at_utc = payload["EventDate"].as_str().or_else(|| msg_obj["CreatedAtUTC"].as_str()).unwrap_or_default();

    let mut is_old_customer = false;
    if !created_at_utc.is_empty() && !event_at_utc.is_empty() {
        if let (Ok(created_date), Ok(event_date)) = (
            chrono::DateTime::parse_from_rfc3339(created_at_utc),
            chrono::DateTime::parse_from_rfc3339(event_at_utc)
        ) {
            let diff = event_date.signed_duration_since(created_date);
            if diff.num_hours() > 24 {
                is_old_customer = true;
            }
        }
    }

    let contact_name = content_obj["Contact"]["Name"]
        .as_str()
        .or_else(|| msg_obj["Chat"]["Contact"]["Name"].as_str())
        .unwrap_or("Cliente");

    if source == "Contact" && !chat_id.is_empty() {
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

                        println!("🎙️ Enviando áudio para o serviço LOCAL do Faster-Whisper (VPS porta 8080)...");
                        let client = reqwest::Client::new();
                        let part = reqwest::multipart::Part::bytes(bytes.to_vec())
                            .file_name("recording.mp3")
                            .mime_str("audio/mp3")
                            .unwrap_or_else(|_| reqwest::multipart::Part::bytes(bytes.to_vec()));
                        let form = reqwest::multipart::Form::new().part("file", part);

                        if let Ok(stt_res) = client.post("http://localhost:8080/transcribe").multipart(form).send().await {
                            if let Ok(json_res) = stt_res.json::<serde_json::Value>().await {
                                let transcribed_text = json_res["text"].as_str().unwrap_or("").trim().to_string();
                                if !transcribed_text.is_empty() {
                                    println!("✅ Transcrição local do Whisper VPS: \"{}\"", transcribed_text);
                                    transcribed_text
                                } else {
                                    "[Áudio enviado pelo cliente]".to_string()
                                }
                            } else {
                                "[Áudio enviado pelo cliente]".to_string()
                            }
                        } else {
                            "[Áudio enviado pelo cliente]".to_string()
                        }
                    }
                    Err(e) => {
                        println!("⚠️ Falha no download direto, buscando via API uTalk: {}", e);
                        if let Ok((mime, b64)) = utalk::fetch_message_audio(
                            &cfg_snapshot.utalk_api_url,
                            &cfg_snapshot.utalk_api_token,
                            &cfg_snapshot.utalk_organization_id,
                            msg_id,
                        ).await {
                            audio_data_tuple = Some((mime, b64.clone()));
                            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&b64) {
                                println!("🎙️ Enviando áudio para o serviço LOCAL do Faster-Whisper (VPS porta 8080)...");
                                let client = reqwest::Client::new();
                                let part = reqwest::multipart::Part::bytes(bytes.clone())
                                    .file_name("recording.mp3")
                                    .mime_str("audio/mp3")
                                    .unwrap_or_else(|_| reqwest::multipart::Part::bytes(bytes));
                                let form = reqwest::multipart::Form::new().part("file", part);

                                if let Ok(stt_res) = client.post("http://localhost:8080/transcribe").multipart(form).send().await {
                                    if let Ok(json_res) = stt_res.json::<serde_json::Value>().await {
                                        let transcribed_text = json_res["text"].as_str().unwrap_or("").trim().to_string();
                                        if !transcribed_text.is_empty() {
                                            println!("✅ Transcrição local do Whisper VPS: \"{}\"", transcribed_text);
                                            transcribed_text
                                        } else {
                                            "[Áudio enviado pelo cliente]".to_string()
                                        }
                                    } else {
                                        "[Áudio enviado pelo cliente]".to_string()
                                    }
                                } else {
                                    "[Áudio enviado pelo cliente]".to_string()
                                }
                            } else {
                                "[Áudio enviado pelo cliente]".to_string()
                            }
                        } else {
                            format!("O cliente '{}' enviou um áudio.", contact_name)
                        }
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
                        audio_data_tuple = Some((mime, b64.clone()));
                        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&b64) {
                            println!("🎙️ Enviando áudio para o serviço LOCAL do Faster-Whisper (VPS porta 8080)...");
                            let client = reqwest::Client::new();
                            let part = reqwest::multipart::Part::bytes(bytes.clone())
                                .file_name("recording.mp3")
                                .mime_str("audio/mp3")
                                .unwrap_or_else(|_| reqwest::multipart::Part::bytes(bytes));
                            let form = reqwest::multipart::Form::new().part("file", part);

                            if let Ok(stt_res) = client.post("http://localhost:8080/transcribe").multipart(form).send().await {
                                if let Ok(json_res) = stt_res.json::<serde_json::Value>().await {
                                    let transcribed_text = json_res["text"].as_str().unwrap_or("").trim().to_string();
                                    if !transcribed_text.is_empty() {
                                        println!("✅ Transcrição local do Whisper VPS: \"{}\"", transcribed_text);
                                        transcribed_text
                                    } else {
                                        "[Áudio enviado pelo cliente]".to_string()
                                    }
                                } else {
                                    "[Áudio enviado pelo cliente]".to_string()
                                }
                            } else {
                                "[Áudio enviado pelo cliente]".to_string()
                            }
                        } else {
                            "[Áudio enviado pelo cliente]".to_string()
                        }
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

        let phone = content_obj["Contact"]["PhoneNumber"].as_str().or_else(|| msg_obj["Chat"]["Contact"]["PhoneNumber"].as_str()).unwrap_or("N/A");

        // 🧪 COMANDO DE RESET AUTOMÁTICO EXCLUSIVO PARA TESTADORES VIP (Claus e Lucas)
        let clean_phone: String = phone.chars().filter(|c: &char| c.is_ascii_digit()).collect();
        let is_tester = cfg_snapshot.test_allowed_phones.iter().any(|p| {
            let clean_p: String = p.chars().filter(|c: &char| c.is_ascii_digit()).collect();
            !clean_p.is_empty() && (clean_phone.ends_with(&clean_p) || clean_p.ends_with(&clean_phone))
        });

        if is_tester && user_prompt.to_lowercase().contains("tubarao_testes") {
            println!("🔄 [MODO TESTE RESET] Comando 'tubarao_testes' recebido de testador VIP ({}). Zerando histórico e reiniciando atendimento...", phone);
            state.db.reset_chat_state(chat_id);
            state.db.set_chat_stage(chat_id, "STAGE_1");
            
            let confirm_msg = "🧪 *[MODO DE TESTE REINICIADO]*\nO seu histórico de testes foi completamente zerado! Olá! Sou o Leandro da equipe da Tubarão Bombas. Qual é a fonte de água que você vai utilizar no seu projeto (ex: poço artesiano, rio, açude)?";
            let _ = utalk::send_utalk_message(&cfg_snapshot.utalk_api_url, &cfg_snapshot.utalk_api_token, &cfg_snapshot.utalk_organization_id, chat_id, confirm_msg).await;
            return;
        }

        // 🎯 LÓGICA DE ESPELHAMENTO DE MÍDIA E ETAPAS (STATE-MACHINE):
        let is_client_audio = msg_type == "Audio";
        let current_stage = state.db.get_chat_stage(chat_id);
        let text_low = user_prompt.to_lowercase();

        let (history_vec, _) = state.db.get_chat_context_for_ai(chat_id, 24);
        let is_first_contact = history_vec.is_empty();

        if is_first_contact {
            if is_old_customer {
                println!("🔄 Cliente antigo detectado [ChatId: {}]. Realizando transbordo imediato (Opção A)...", chat_id);
                let mut candidate_ids = cfg_snapshot.rotation_operator_ids.clone();
                if let Ok(online_ids) = utalk::fetch_online_members(
                    &cfg_snapshot.utalk_api_url,
                    &cfg_snapshot.utalk_api_token,
                    &cfg_snapshot.utalk_organization_id,
                )
                .await
                {
                    candidate_ids.retain(|id| online_ids.contains(id));
                }

                let target_op_name = if let Some(target_operator_id) = state.db.get_next_rotation_operator(&candidate_ids) {
                    let _ = utalk::transfer_chat_to_member(
                        &cfg_snapshot.utalk_api_url,
                        &cfg_snapshot.utalk_api_token,
                        &cfg_snapshot.utalk_organization_id,
                        chat_id,
                        &target_operator_id,
                    ).await;
                    "Equipe de Vendas".to_string()
                } else {
                    "Equipe (Nenhum Online)".to_string()
                };

                let transfer_msg = "Olá! Vi que você já conversou com a gente antes. Para agilizar seu atendimento, estou transferindo você diretamente para um de nossos especialistas. Aguarde um instante!";
                let _ = utalk::send_utalk_message(
                    &cfg_snapshot.utalk_api_url,
                    &cfg_snapshot.utalk_api_token,
                    &cfg_snapshot.utalk_organization_id,
                    chat_id,
                    transfer_msg,
                ).await;

                state.db.record_transfer(chat_id, "transferred", &target_op_name);
                println!("📊 Transferência por CLIENTE ANTIGO gravada: Chat {} -> {}", chat_id, target_op_name);
                return;
            } else {
                println!("🆕 Primeiro contato detectado [ChatId: {}]. IA (DeepSeek) assumindo apresentação inicial...", chat_id);
                state.db.set_chat_stage(chat_id, "STAGE_1");
            }
        }

        // Salva a mensagem do usuário no histórico para manter contexto contínuo
        state.db.save_message(chat_id, "user", &user_prompt);

        // 🤖 PASSAGEM DIRETA DE CONTROLE PARA A INTELIGÊNCIA ARTIFICIAL (DEEPSEEK)
        println!("🤖 [PROCESSANDO COM DEEPSEEK IA] Analisando mensagem do cliente e conduzindo a triagem...");

        match deepseek::generate_deepseek_response(state.db.clone(), &cfg_snapshot, chat_id, &user_prompt).await {
            Ok(mut ai_reply) => {
                println!("✨ DeepSeek gerou resposta:\n{}", ai_reply);
                state.db.save_message(chat_id, "assistant", &ai_reply);

                let should_transfer = cfg_snapshot.rotation_enabled
                    && !cfg_snapshot.rotation_trigger_keyword.is_empty()
                    && ai_reply.contains(&cfg_snapshot.rotation_trigger_keyword);

                if should_transfer {
                    println!("🔄 Gatilho de Rodízio detectado na resposta do Gemini!");
                    ai_reply = ai_reply.replace(&cfg_snapshot.rotation_trigger_keyword, "").trim().to_string();
                }

                if !ai_reply.is_empty() {
                    let mut selected_audio_url = String::new();

                    // Procura a tag [AUDIO_KEY: chave] sugerida pelo DeepSeek no catálogo de áudios pré-gravados
                    if let Some(pos) = ai_reply.find("[AUDIO_KEY:") {
                        if let Some(end_pos) = ai_reply[pos..].find(']') {
                            let audio_key = ai_reply[pos + 11..pos + end_pos].trim().to_string();
                            let cleaned_text = ai_reply[..pos].to_string() + &ai_reply[pos + end_pos + 1..];
                            ai_reply = cleaned_text.trim().to_string();

                            let audio_items = state.db.get_all_audio_bank_items();
                            if let Some(arr) = audio_items.as_array() {
                                for item in arr {
                                    if item["key"].as_str().unwrap_or_default() == audio_key {
                                        selected_audio_url = item["audio_url"].as_str().unwrap_or_default().to_string();
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    let mut sent_audio = false;
                    if !selected_audio_url.is_empty() {
                        let full_audio_url = if selected_audio_url.starts_with("http://") || selected_audio_url.starts_with("https://") {
                            selected_audio_url
                        } else {
                            format!("https://tubaraoia.lysia.tech{}", selected_audio_url)
                        };

                        if is_client_audio {
                            println!("🎙️ Enviando áudio pré-gravado do Catálogo ao cliente: {}...", full_audio_url);
                            let _ = utalk::send_utalk_audio_message(
                                &cfg_snapshot.utalk_api_url,
                                &cfg_snapshot.utalk_api_token,
                                &cfg_snapshot.utalk_organization_id,
                                chat_id,
                                &full_audio_url,
                            ).await;
                            sent_audio = true;
                        }
                    }

                    // Se não enviou apenas o áudio, ou se a mensagem tem resumo de transferência, envia o texto
                    if !sent_audio || should_transfer {
                        let _ = utalk::send_utalk_message(
                            &cfg_snapshot.utalk_api_url,
                            &cfg_snapshot.utalk_api_token,
                            &cfg_snapshot.utalk_organization_id,
                            chat_id,
                            &ai_reply,
                        ).await;
                    }
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

async fn sync_webhooks_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<Value>) {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            let cfg = state.db.get_config();
            match utalk::fetch_utalk_webhooks(
                &cfg.utalk_api_url,
                &cfg.utalk_api_token,
                &cfg.utalk_organization_id,
            )
            .await
            {
                Ok(webhooks) => {
                    let json_val = serde_json::to_value(&webhooks).unwrap_or_default();
                    state.db.save_synced_webhooks(&json_val);
                    println!("🔄 Webhooks do uTalk sincronizados com sucesso! (Total: {})", webhooks.len());
                    return (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "success": true,
                            "webhooks": webhooks
                        })),
                    );
                }
                Err(err) => {
                    println!("❌ Erro ao sincronizar webhooks do uTalk: {}", err);
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({ "error": err })),
                    );
                }
            }
        }
    }
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "Não autorizado" })))
}

async fn get_synced_webhooks_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            let val = state.db.get_synced_webhooks();
            return Ok(Json(val));
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

async fn get_stages_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            let stages = state.db.get_all_stage_configs();
            return Ok(Json(stages));
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

#[derive(Deserialize)]
struct SaveStageRequest {
    stage_key: String,
    title: String,
    text_message: String,
    audio_url: String,
}

async fn save_stage_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SaveStageRequest>,
) -> (StatusCode, Json<Value>) {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            state.db.save_stage_config(&req.stage_key, &req.title, &req.text_message, &req.audio_url);
            return (StatusCode::OK, Json(serde_json::json!({ "success": true })));
        }
    }
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "Não autorizado" })))
}

async fn get_assets_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let full_path = format!("assets/{}", path);
    if let Ok(bytes) = std::fs::read(&full_path) {
        let mime = if path.ends_with(".mp3") {
            "audio/mpeg"
        } else if path.ends_with(".png") {
            "image/png"
        } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
            "image/jpeg"
        } else {
            "application/octet-stream"
        };
        return (
            [(axum::http::header::CONTENT_TYPE, mime)],
            bytes,
        ).into_response();
    }
    (StatusCode::NOT_FOUND, "Arquivo não encontrado").into_response()
}

async fn upload_audio_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: axum::extract::Multipart,
) -> (StatusCode, Json<Value>) {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            while let Ok(Some(field)) = multipart.next_field().await {
                let name = field.name().unwrap_or("file").to_string();
                if name == "file" || name == "audio" {
                    let original_name = field.file_name().unwrap_or("audio.mp3").to_string();
                    let ext = if original_name.ends_with(".wav") { ".wav" } else { ".mp3" };
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    original_name.hash(&mut hasher);
                    let time_now = chrono::Utc::now().timestamp_millis();
                    let filename = format!("upload_{}_{}{}", time_now, hasher.finish(), ext);
                    let save_path = format!("assets/uploads/{}", filename);
                    let _ = std::fs::create_dir_all("assets/uploads");

                    if let Ok(bytes) = field.bytes().await {
                        if std::fs::write(&save_path, bytes).is_ok() {
                            let rel_url = format!("/assets/uploads/{}", filename);
                            println!("📁 Upload de áudio salvo em: {}", save_path);
                            return (StatusCode::OK, Json(serde_json::json!({ "success": true, "url": rel_url })));
                        }
                    }
                }
            }
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Nenhum arquivo enviado" })));
        }
    }
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "Não autorizado" })))
}

async fn get_audio_bank_handler(State(state): State<AppState>) -> Json<Value> {
    Json(state.db.get_all_audio_bank_items())
}

async fn save_audio_bank_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(item): Json<Value>,
) -> (StatusCode, Json<Value>) {
    if let Some(token) = extract_token(&headers) {
        if state.db.validate_session(&token) {
            if state.db.save_audio_bank_item(&item) {
                return (StatusCode::OK, Json(serde_json::json!({ "success": true })));
            }
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Falha ao salvar item no Banco de Áudios" })));
        }
    }
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "Não autorizado" })))
}

#[derive(Deserialize)]
struct SimulateChatRequest {
    chat_id: String,
    msg_type: String,
    content: String,
}

async fn simulate_chat_handler(
    State(state): State<AppState>,
    _headers: HeaderMap,
    Json(req): Json<SimulateChatRequest>,
) -> (StatusCode, Json<Value>) {
    let chat_id = if req.chat_id.trim().is_empty() { "simulacao_demo" } else { req.chat_id.trim() };
    let is_client_audio = req.msg_type == "Audio";
    let user_prompt = req.content.clone();
    let cfg_snapshot = state.db.get_config();

    println!("🧪 [SIMULADOR DE CHAT] Nova mensagem recebida. Chat ID: '{}', Mídia: '{}', Mensagem: '{}'", chat_id, req.msg_type, user_prompt);

    // Salva a mensagem do usuario no historico
    state.db.save_message(chat_id, "user", &user_prompt);

    match deepseek::generate_deepseek_response(state.db.clone(), &cfg_snapshot, chat_id, &user_prompt).await {
        Ok(mut ai_reply) => {
            let is_transfer = ai_reply.contains("[TRANSFERIR]");
            if is_transfer {
                ai_reply = ai_reply.replace("[TRANSFERIR]", "").trim().to_string();
            }

            let mut selected_audio_url = String::new();

            // Extrai a chave de audio selecionada pela IA DeepSeek
            if let Some(pos) = ai_reply.find("[AUDIO_KEY:") {
                if let Some(end_pos) = ai_reply[pos..].find(']') {
                    let audio_key = ai_reply[pos + 11..pos + end_pos].trim().to_string();
                    let cleaned_text = ai_reply[..pos].to_string() + &ai_reply[pos + end_pos + 1..];
                    ai_reply = cleaned_text.trim().to_string();

                    let audio_items = state.db.get_all_audio_bank_items();
                    if let Some(arr) = audio_items.as_array() {
                        for item in arr {
                            if item["key"].as_str().unwrap_or_default() == audio_key {
                                selected_audio_url = item["audio_url"].as_str().unwrap_or_default().to_string();
                                break;
                            }
                        }
                    }
                }
            }

            let has_matching_audio = !selected_audio_url.is_empty();

            let reply_type = if is_client_audio && has_matching_audio {
                "Audio"
            } else {
                "Text"
            };

            let reply_audio_url = if reply_type == "Audio" {
                selected_audio_url
            } else {
                String::new()
            };

            state.db.save_message(chat_id, "assistant", &ai_reply);

            let transfer_info = if is_transfer {
                let project_data = deepseek::extract_project_summary(state.db.clone(), &cfg_snapshot, chat_id).await;
                serde_json::json!({
                    "is_transferred": true,
                    "operator": "Leandro Humberto (Rodízio de Atendentes)",
                    "summary": "DeepSeek concluiu a triagem autônoma do projeto solar. Atendimento pausado para o robô e encaminhado ao especialista humano.",
                    "details": project_data
                })
            } else {
                serde_json::json!({ "is_transferred": false })
            };

            return (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "chat_id": chat_id,
                "reply_type": reply_type,
                "reply_text": ai_reply,
                "reply_audio_url": reply_audio_url,
                "was_ai_intervention": true,
                "transfer_info": transfer_info
            })));
        }
        Err(err) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": err })));
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
        .route("/assets/*path", get(get_assets_handler))
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
        .route("/api/webhooks/synced", get(get_synced_webhooks_handler))
        .route("/api/webhooks/sync", axum::routing::post(sync_webhooks_handler))
        .route("/api/stages", get(get_stages_handler).post(save_stage_handler))
        .route("/api/audio-bank", get(get_audio_bank_handler).post(save_audio_bank_handler))
        .route("/api/upload-audio", axum::routing::post(upload_audio_handler))
        .route("/api/simulate", axum::routing::post(simulate_chat_handler))
        .route("/webhook", any(handle_webhook))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 uTalk AI Bot Studio rodando com Rodízio de Atendentes + Dashboard em Abas!");
    println!("📊 Dashboard Web de Controle: http://localhost:3000/");
    println!("📍 Endpoint do Webhook: http://localhost:3000/webhook");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
