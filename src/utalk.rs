use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct SendTextMessageRequest {
    #[serde(rename = "chatId")]
    pub chat_id: String,
    pub content: String,
    #[serde(rename = "messageType")]
    pub message_type: String,
}

pub async fn send_utalk_message(
    api_url: &str,
    token: &str,
    org_id: &str,
    chat_id: &str,
    content: &str,
) -> Result<String, String> {
    if token.is_empty() || org_id.is_empty() {
        return Err("Token ou Organization ID do uTalk não configurados".to_string());
    }

    let client = reqwest::Client::new();
    let url = format!("{}/messages?organizationId={}", api_url.trim_end_matches('/'), org_id);

    let body = json!({
        "ChatId": chat_id,
        "OrganizationId": org_id,
        "Message": content
    });

    println!("📤 Enviando mensagem ao cliente via uTalk API [ChatId: {}]...", chat_id);

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Erro HTTP ao enviar mensagem uTalk: {}", e))?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Sem corpo de resposta".to_string());

    if status.is_success() {
        println!("✅ Mensagem entregue com sucesso no uTalk!");
        Ok(response_text)
    } else {
        println!("❌ Falha no envio uTalk HTTP {}: {}", status, response_text);
        Err(format!("Erro uTalk HTTP {}: {}", status, response_text))
    }
}

pub async fn send_utalk_audio_message(
    api_url: &str,
    token: &str,
    org_id: &str,
    chat_id: &str,
    audio_url: &str,
) -> Result<String, String> {
    if token.is_empty() || org_id.is_empty() {
        return Err("Token ou Organization ID do uTalk não configurados".to_string());
    }

    let client = reqwest::Client::new();
    let url = format!("{}/messages?organizationId={}", api_url.trim_end_matches('/'), org_id);

    // Tenta formato padrão do uTalk para mídias/áudios com redundância de chaves exigidas pela API
    let body = json!({
        "ChatId": chat_id,
        "OrganizationId": org_id,
        "MediaUrl": audio_url,
        "Url": audio_url,
        "File": audio_url,
        "Content": audio_url,
        "MessageType": "Audio"
    });

    println!("🎙️ Enviando NOTA DE VOZ (ÁUDIO MP3) ao cliente via uTalk API [ChatId: {}]...", chat_id);

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Erro HTTP ao enviar áudio uTalk: {}", e))?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Sem corpo de resposta".to_string());

    if status.is_success() {
        println!("✅ Mensagem de voz entregue com sucesso no uTalk!");
        Ok(response_text)
    } else {
        println!("❌ Falha no envio de áudio uTalk HTTP {}: {}", status, response_text);
        Err(format!("Erro uTalk HTTP {}: {}", status, response_text))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UtalkOperator {
    pub id: String,
    pub name: String,
    pub email: String,
    pub active: bool,
    pub is_online: bool,
}

pub async fn fetch_human_operators(
    api_url: &str,
    token: &str,
    org_id: &str,
) -> Result<Vec<UtalkOperator>, String> {
    if token.is_empty() || org_id.is_empty() {
        return Err("Token ou Organization ID do uTalk não configurados".to_string());
    }

    let client = reqwest::Client::new();
    let url = format!("{}/organizations/{}/", api_url.trim_end_matches('/'), org_id);

    let online_ids = fetch_online_members(api_url, token, org_id).await.unwrap_or_default();

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Erro HTTP ao consultar organização uTalk: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Erro HTTP ao consultar organização: {}", response.status()));
    }

    let val: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Erro ao decodificar JSON da organização: {}", e))?;

    let mut operators = Vec::new();
    if let Some(members) = val.get("organizationMembers").and_then(|m| m.as_array()) {
        for m in members {
            let member_type = m.get("_t").and_then(|t| t.as_str()).unwrap_or_default();
            if member_type == "OrganizationHumanAgentReferenceModel" {
                let id = m.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let name = m.get("displayName")
                    .or_else(|| m.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Sem nome")
                    .to_string();
                let email = m.get("emailAddress").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let active = m.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
                let is_online = online_ids.contains(&id);

                if !id.is_empty() {
                    operators.push(UtalkOperator {
                        id,
                        name,
                        email,
                        active,
                        is_online,
                    });
                }
            }
        }
    }

    Ok(operators)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UtalkWebhookConfig {
    pub id: String,
    pub name: String,
    pub url: String,
    pub paused: bool,
    #[serde(rename = "forChannels")]
    pub for_channels: Vec<String>,
    pub events: Vec<String>,
}

pub async fn fetch_utalk_webhooks(
    api_url: &str,
    token: &str,
    org_id: &str,
) -> Result<Vec<UtalkWebhookConfig>, String> {
    if token.is_empty() || org_id.is_empty() {
        return Err("Token ou Organization ID do uTalk não configurados".to_string());
    }

    let client = reqwest::Client::new();
    let url = format!("{}/webhooks?organizationId={}", api_url.trim_end_matches('/'), org_id);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Erro HTTP ao consultar webhooks do uTalk: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Erro HTTP {} ao buscar webhooks do uTalk", response.status()));
    }

    let items: Vec<serde_json::Value> = response
        .json()
        .await
        .map_err(|e| format!("Erro ao decodificar JSON dos webhooks: {}", e))?;

    let mut result = Vec::new();
    for item in items {
        let id = item["id"].as_str().unwrap_or_default().to_string();
        let name = item["name"].as_str().unwrap_or("Sem nome").to_string();
        let url_str = item["url"].as_str().unwrap_or_default().to_string();
        let paused = item["paused"].as_bool().unwrap_or(false);
        let for_channels = item["forChannels"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();
        let events = item["events"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        result.push(UtalkWebhookConfig {
            id,
            name,
            url: url_str,
            paused,
            for_channels,
            events,
        });
    }

    Ok(result)
}

pub async fn fetch_online_members(
    api_url: &str,
    token: &str,
    org_id: &str,
) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/members/online/?organizationId={}", api_url.trim_end_matches('/'), org_id);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Erro HTTP ao consultar membros online: {}", e))?;

    if !response.status().is_success() {
        return Ok(Vec::new());
    }

    let val: serde_json::Value = response.json().await.unwrap_or_default();
    let mut ids = Vec::new();
    if let Some(arr) = val.as_array() {
        for item in arr {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                ids.push(id.to_string());
            }
        }
    }
    Ok(ids)
}

pub async fn transfer_chat_to_member(
    api_url: &str,
    token: &str,
    org_id: &str,
    chat_id: &str,
    member_id: &str,
) -> Result<String, String> {
    if token.is_empty() || org_id.is_empty() {
        return Err("Token ou Organization ID do uTalk não configurados".to_string());
    }

    let client = reqwest::Client::new();
    let url = format!("{}/chats/{}/?organizationId={}", api_url.trim_end_matches('/'), chat_id, org_id);

    let body = json!({
        "memberId": member_id
    });

    println!("🔄 Transferindo chat {} no uTalk para o membro {}...", chat_id, member_id);

    let response = client
        .put(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Erro HTTP ao transferir chat uTalk: {}", e))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();

    if status.is_success() {
        println!("✅ Chat {} transferido com sucesso no uTalk!", chat_id);
        Ok(text)
    } else {
        println!("❌ Erro ao transferir chat {} no uTalk [Status {}]: {}", chat_id, status, text);
        Err(format!("Erro ao transferir chat uTalk HTTP {}: {}", status, text))
    }
}

pub async fn fetch_message_audio(
    api_url: &str,
    token: &str,
    org_id: &str,
    msg_id: &str,
) -> Result<(String, String), String> {
    if token.is_empty() || org_id.is_empty() || msg_id.is_empty() {
        return Err("Dados incompletos para buscar áudio".to_string());
    }

    let client = reqwest::Client::new();
    let url = format!("{}/messages/{}?organizationId={}", api_url.trim_end_matches('/'), msg_id, org_id);

    println!("📥 Buscando metadados do áudio no uTalk [MsgId: {}]...", msg_id);

    for attempt in 1..=4 {
        let res = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Erro HTTP uTalk GET message: {}", e))?;

        if res.status().is_success() {
            let msg_json: serde_json::Value = res.json().await.map_err(|e| format!("Erro ao ler JSON da mensagem: {}", e))?;

            let media_url = msg_json["file"]["url"]
                .as_str()
                .or_else(|| msg_json["File"]["Url"].as_str())
                .or_else(|| msg_json["media"]["url"].as_str())
                .or_else(|| msg_json["Media"]["Url"].as_str())
                .or_else(|| msg_json["mediaUrl"].as_str())
                .or_else(|| msg_json["MediaUrl"].as_str());

            let content_type = msg_json["file"]["contentType"]
                .as_str()
                .or_else(|| msg_json["File"]["ContentType"].as_str())
                .or_else(|| msg_json["media"]["contentType"].as_str())
                .or_else(|| msg_json["Media"]["ContentType"].as_str())
                .unwrap_or("audio/mp3")
                .to_string();

            if let Some(u) = media_url {
                if !u.is_empty() {
                    println!("🔊 Baixando arquivo de áudio de {} (Tentativa {})...", u, attempt);
                    let audio_res = client.get(u).send().await.map_err(|e| format!("Erro ao baixar áudio: {}", e))?;
                    let bytes = audio_res.bytes().await.map_err(|e| format!("Erro nos bytes do áudio: {}", e))?;
                    use base64::Engine;
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    println!("✅ Áudio baixado ({:.1} KB) e codificado em Base64 para transcrição!", bytes.len() as f64 / 1024.0);
                    return Ok((content_type, b64));
                }
            }
        }

        if attempt < 4 {
            println!("⏳ Áudio em processamento no uTalk (Tentativa {}/4). Aguardando 1.5s...", attempt);
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        }
    }

    Err("URL da mídia de áudio não encontrada na mensagem do uTalk após 4 tentativas".to_string())
}

pub async fn fetch_last_attendant_from_utalk(
    api_url: &str,
    token: &str,
    org_id: &str,
    phone: &str,
) -> Result<Option<String>, String> {
    if token.is_empty() || org_id.is_empty() || phone.is_empty() {
        return Ok(None);
    }

    let clean_phone: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    let phone_param = format!("%2B{}", clean_phone);

    let client = reqwest::Client::new();
    let url = format!("{}/chats?organizationId={}&phoneNumber={}", api_url.trim_end_matches('/'), org_id, phone_param);

    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Erro HTTP ao buscar chats do uTalk: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Erro HTTP {} ao buscar chats", res.status()));
    }

    let val: serde_json::Value = res.json().await.map_err(|e| format!("Erro no JSON dos chats: {}", e))?;

    let mut all_histories: Vec<(String, String)> = Vec::new(); // (enteredAt, memberId)

    if let Some(items) = val.get("items").and_then(|v| v.as_array()) {
        for chat in items {
            // A API da Umbler PODE ignorar o parâmetro phoneNumber e retornar chats de outros clientes.
            // Precisamos filtrar estritamente aqui para garantir que o chat pertence a este telefone.
            let chat_phone = chat
                .get("contact")
                .and_then(|c| c.get("phoneNumber"))
                .and_then(|p| p.as_str())
                .unwrap_or("");
                
            if chat_phone != phone_param.replace("%2B", "+") && chat_phone != phone {
                continue;
            }

            // Busca o histórico de atendentes de TODOS os canais/chats desse número específico
            if let Some(history) = chat.get("organizationMemberHistory").and_then(|h| h.as_array()) {
                for entry in history {
                    let entered_at = entry.get("enteredAt").and_then(|e| e.as_str()).unwrap_or("").to_string();
                    let member_id = entry.get("memberId").and_then(|id| id.as_str()).unwrap_or("").to_string();
                    
                    if !entered_at.is_empty() && !member_id.is_empty() {
                        all_histories.push((entered_at, member_id));
                    }
                }
            }
        }
    }

    // Se houver históricos, ordena por 'enteredAt' decrescente (o mais recente primeiro)
    // Isso garante que o cliente fique amarrado ao ÚLTIMO atendente com quem ele negociou,
    // impedindo que ele pule de galho em galho buscando preços em canais diferentes no momento.
    if !all_histories.is_empty() {
        all_histories.sort_by(|a, b| b.0.cmp(&a.0));
        let most_recent_member_id = all_histories[0].1.clone();
        return Ok(Some(most_recent_member_id));
    }

    Ok(None)
}

pub async fn sync_all_historical_chats(
    api_url: &str,
    token: &str,
    org_id: &str,
    db: Arc<crate::db::Database>,
) -> Result<usize, String> {
    if token.is_empty() || org_id.is_empty() {
        return Err("Token ou Organization ID do uTalk não configurados".to_string());
    }

    let client = reqwest::Client::new();
    let mut skip = 0;
    let limit = 100;
    let mut total_chats_processed = 0;
    let mut seen_chat_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut total_records_to_save: Vec<(String, String, String, String, String)> = Vec::new();

    println!("🚀 [CARGA HISTÓRICA] Iniciando varredura com trava de deduplicação anti-loop...");

    loop {
        let url = format!("{}/chats?organizationId={}&limit={}&skip={}", api_url.trim_end_matches('/'), org_id, limit, skip);
        let res = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Erro HTTP ao paginar chats uTalk: {}", e))?;

        if !res.status().is_success() {
            break;
        }

        let val: serde_json::Value = res.json().await.unwrap_or_default();
        let items = match val.get("items").and_then(|v| v.as_array()) {
            Some(arr) if !arr.is_empty() => arr,
            _ => break,
        };

        let mut new_chats_in_this_batch = 0;

        for chat in items {
            let chat_id = chat.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if chat_id.is_empty() { continue; }

            if !seen_chat_ids.insert(chat_id.to_string()) {
                // Já processamos este chat anteriormente
                continue;
            }

            new_chats_in_this_batch += 1;

            let phone = chat
                .get("contact")
                .and_then(|c| c.get("phoneNumber"))
                .and_then(|p| p.as_str())
                .unwrap_or("");

            if phone.is_empty() {
                continue;
            }

            // Histórico de atendentes por data
            if let Some(history) = chat.get("organizationMemberHistory").and_then(|h| h.as_array()) {
                for entry in history {
                    let entered_at = entry.get("enteredAt").and_then(|e| e.as_str()).unwrap_or("").to_string();
                    let member_id = entry.get("memberId").and_then(|id| id.as_str()).unwrap_or("").to_string();
                    
                    if !entered_at.is_empty() && !member_id.is_empty() {
                        total_records_to_save.push((
                            phone.to_string(),
                            chat_id.to_string(),
                            member_id,
                            "Atendente uTalk".to_string(),
                            entered_at,
                        ));
                    }
                }
            } else if let Some(last_member) = chat.get("lastOrganizationMember").and_then(|m| m.get("id")).and_then(|i| i.as_str()) {
                if !last_member.is_empty() {
                    total_records_to_save.push((
                        phone.to_string(),
                        chat_id.to_string(),
                        last_member.to_string(),
                        "Atendente uTalk".to_string(),
                        "1970-01-01T00:00:00Z".to_string(),
                    ));
                }
            }
        }

        total_chats_processed += new_chats_in_this_batch;
        println!("📦 [CARGA HISTÓRICA] Processados {} novos chats únicos (Total acumulado: {})", new_chats_in_this_batch, total_chats_processed);

        // Se a API não devolveu NENHUM chat novo neste lote, significa que a paginação terminou/repetiu!
        if new_chats_in_this_batch == 0 {
            println!("🛑 [CARGA HISTÓRICA] Fim do histórico alcançado! (Nenhum chat novo encontrado neste lote).");
            break;
        }

        skip += limit;
    }

    if !total_records_to_save.is_empty() {
        let saved_count = db.bulk_save_customer_attendants(&total_records_to_save);
        println!("✅ [CARGA HISTÓRICA COMPLETA] {} registros de clientes importados/indexados com sucesso no SQLite!", saved_count);
        return Ok(saved_count);
    }

    Ok(0)
}
