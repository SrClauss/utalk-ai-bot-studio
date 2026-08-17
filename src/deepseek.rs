use crate::config::AppConfig;
use crate::db::SharedDatabase;
use serde_json::{json, Value};

pub async fn generate_deepseek_response(
    db: SharedDatabase,
    config: &AppConfig,
    chat_id: &str,
    user_message: &str,
) -> Result<String, String> {
    let api_key = "sk-92f4266dd5d14b2884f3d7c35bb81911";
    let url = "https://api.deepseek.com/v1/chat/completions";

    let (history_vec, _) = db.get_chat_context_for_ai(chat_id, 24);

    let mut messages = Vec::new();

    let system_prompt = format!(
        "{}\n\n### REGRA MANDATÓRIA DE ATENDIMENTO TUBARÃO BOMBAS:\n- Você é o Leandro, consultor técnico da Tubarão Bombas.\n- Atenda o cliente de forma cortês, neutra, formal e objetiva (tom consultivo, sem entonação de locutor/político).\n- Não invente preços ou dados irreais. Ao concluir a coleta de dados de poço/vazão ou se o cliente solicitar um orçamento final/projeto especial, inclua OBRIGATORIAMENTE a tag `[TRANSFERIR]` no final da resposta.",
        config.system_prompt
    );

    messages.push(json!({
        "role": "system",
        "content": system_prompt
    }));

    for item in history_vec {
        if let Some(role) = item["role"].as_str() {
            if let Some(parts) = item["parts"].as_array() {
                for p in parts {
                    if let Some(txt) = p["text"].as_str() {
                        let ds_role = if role == "model" || role == "assistant" { "assistant" } else { "user" };
                        messages.push(json!({
                            "role": ds_role,
                            "content": txt
                        }));
                    }
                }
            }
        }
    }

    messages.push(json!({
        "role": "user",
        "content": user_message
    }));

    let payload = json!({
        "model": "deepseek-chat",
        "messages": messages,
        "temperature": 0.7
    });

    let client = reqwest::Client::new();
    let res = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Erro HTTP ao chamar DeepSeek: {}", e))?;

    let res_json: Value = res
        .json()
        .await
        .map_err(|e| format!("Erro ao parsear JSON do DeepSeek: {}", e))?;

    if let Some(choices) = res_json["choices"].as_array() {
        if let Some(first) = choices.first() {
            if let Some(content) = first["message"]["content"].as_str() {
                return Ok(content.trim().to_string());
            }
        }
    }

    if let Some(err_msg) = res_json["error"]["message"].as_str() {
        return Err(format!("Erro da API DeepSeek: {}", err_msg));
    }

    Err("Resposta vazia da API DeepSeek".to_string())
}
