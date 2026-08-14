use crate::config::{AppConfig, ExternalApiIntegration};
use crate::db::SharedDatabase;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionCallArgs {
    pub integration_id: String,
    pub http_method: String,
    pub endpoint: String,
    pub params_json: Option<String>,
    pub payload_json: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchConversationsArgs {
    pub query: String,
    pub chat_id: Option<String>,
}

pub async fn execute_external_api_call(
    config: &AppConfig,
    args: &FunctionCallArgs,
) -> Result<String, String> {
    println!("\n⚡ [TOOL CALL] Executando request_external_api:");
    println!("   Integração: {}", args.integration_id);
    println!("   Método HTTP: {}", args.http_method);
    println!("   Endpoint: {}", args.endpoint);

    let integration = config
        .external_apis
        .iter()
        .find(|api| {
            api.enabled
                && (api.id.to_lowercase() == args.integration_id.to_lowercase()
                    || api.name.to_lowercase().contains(&args.integration_id.to_lowercase()))
        })
        .or_else(|| config.external_apis.iter().find(|api| api.enabled));

    let api_info = match integration {
        Some(api) => api,
        None => {
            let err = format!("Erro: Integração '{}' não foi encontrada ou está desativada no painel.", args.integration_id);
            println!("❌ {}", err);
            return Ok(json!({ "error": err }).to_string());
        }
    };

    let target_url = if args.endpoint.starts_with("http://") || args.endpoint.starts_with("https://") {
        args.endpoint.clone()
    } else {
        let base = api_info.base_url.trim_end_matches('/');
        let path = if args.endpoint.starts_with('/') {
            args.endpoint.clone()
        } else {
            format!("/{}", args.endpoint)
        };
        format!("{}{}", base, path)
    };

    let client = reqwest::Client::new();
    let method = match args.http_method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "PATCH" => reqwest::Method::PATCH,
        _ => reqwest::Method::GET,
    };

    let mut req = client.request(method, &target_url);

    for header in &api_info.headers {
        if !header.key.trim().is_empty() {
            req = req.header(&header.key, &header.value);
        }
    }

    if let Some(params_str) = &args.params_json {
        if let Ok(params_map) = serde_json::from_str::<HashMap<String, String>>(params_str) {
            req = req.query(&params_map);
        }
    }

    if let Some(payload_str) = &args.payload_json {
        if let Ok(json_body) = serde_json::from_str::<Value>(payload_str) {
            req = req.json(&json_body);
        } else if !payload_str.is_empty() {
            req = req.body(payload_str.clone());
        }
    }

    println!("🌐 Disparando requisição HTTP -> {}", target_url);

    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            println!("📥 Resposta HTTP {}: {}", status, body_text);
            Ok(json!({
                "http_status": status.as_u16(),
                "response": body_text
            }).to_string())
        }
        Err(e) => {
            let err_msg = format!("Falha na chamada HTTP da API Externa: {}", e);
            println!("❌ {}", err_msg);
            Ok(json!({ "error": err_msg }).to_string())
        }
    }
}

pub async fn generate_gemini_response(
    db: SharedDatabase,
    config: &AppConfig,
    chat_id: &str,
    user_message: &str,
) -> Result<String, String> {
    if config.gemini_api_key.is_empty() {
        return Err("GEMINI_API_KEY não foi configurada.".to_string());
    }

    // Carrega o histórico persistido do chat analisando se houve pausa longa (> 24h)
    let (mut contents, hours_elapsed) = db.get_chat_context_for_ai(chat_id, 24);

    // Salva a nova mensagem do usuário no banco SQLite com timestamp
    db.save_message(chat_id, "user", user_message);
    contents.push(json!({
        "role": "user",
        "parts": [{ "text": user_message }]
    }));

    if let Some(hours) = hours_elapsed {
        println!("⏱️ Tempo decorrido desde última mensagem de [{}]: {} horas", chat_id, hours);
    }

    // Constrói a documentação de todas as APIs externas para o System Prompt
    let mut apis_doc = String::new();
    for api in &config.external_apis {
        if api.enabled {
            apis_doc.push_str(&format!(
                "\n- **ID/Nome da API:** `{}` ({})\n  - **Base URL:** {}\n  - **Documentação & Endpoints:** {}\n",
                api.id, api.name, api.base_url, api.documentation
            ));
        }
    }

    let full_system_prompt = format!(
        "{}\n\n### ANÁLISE DE TEMPO & EVITANDO DUPLICAÇÃO:\n- Sempre analise as mensagens anteriores. Se o cliente respondeu rapidamente (poucas horas), continue a conversa atual.\n- Se houver nota de intervalo longo (dias), considere o contexto anterior mas reinicie o atendimento de forma cordial.\n- Utilize `search_past_conversations` se precisar pesquisar assuntos/orçamentos antigos discutidos no passado.\n\n### APIS EXTERNAS DISPONÍVEIS VIA `request_external_api`:\n{}",
        config.system_prompt,
        if apis_doc.is_empty() { "Nenhuma API externa cadastrada." } else { &apis_doc }
    );

    let tools_declaration = json!([{
        "function_declarations": [
            {
                "name": "request_external_api",
                "description": "Executa chamadas HTTP para APIs externas cadastradas no sistema para consultar estoque, cadastrar dados, buscar pedidos, etc.",
                "parameters": {
                    "type": "OBJECT",
                    "properties": {
                        "integration_id": {
                            "type": "STRING",
                            "description": "ID ou Nome da integração cadastrada a ser chamada"
                        },
                        "http_method": {
                            "type": "STRING",
                            "description": "Verbo HTTP: GET, POST, PUT, DELETE, PATCH"
                        },
                        "endpoint": {
                            "type": "STRING",
                            "description": "Caminho do endpoint"
                        },
                        "params_json": {
                            "type": "STRING",
                            "description": "Opcional: JSON string com Query Parameters"
                        },
                        "payload_json": {
                            "type": "STRING",
                            "description": "Opcional: JSON string com o corpo/payload da requisição"
                        }
                    },
                    "required": ["integration_id", "http_method", "endpoint"]
                }
            },
            {
                "name": "search_past_conversations",
                "description": "Realiza busca textual (Full-Text Search) no histórico de conversas passadas para relembrar orçamentos, produtos ou detalhes mencionados anteriormente.",
                "parameters": {
                    "type": "OBJECT",
                    "properties": {
                        "query": {
                            "type": "STRING",
                            "description": "Palavra-chave ou texto a pesquisar nas mensagens anteriores"
                        },
                        "chat_id": {
                            "type": "STRING",
                            "description": "Opcional: ID do chat específico para filtrar a pesquisa"
                        }
                    },
                    "required": ["query"]
                }
            }
        ]
    }]);

    let gemini_url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
        config.gemini_api_key
    );

    let client = reqwest::Client::new();

    let payload = json!({
        "system_instruction": {
            "parts": [{ "text": full_system_prompt }]
        },
        "contents": contents,
        "tools": tools_declaration
    });

    println!("🤖 Solicitando resposta ao Gemini (Chat: {})...", chat_id);
    let response = client
        .post(&gemini_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Erro HTTP Gemini: {}", e))?;

    let res_json: Value = response
        .json()
        .await
        .map_err(|e| format!("Erro ao ler JSON do Gemini: {}", e))?;

    if let Some(candidate) = res_json["candidates"].get(0) {
        if let Some(parts) = candidate["content"]["parts"].as_array() {
            for part in parts {
                if let Some(func_call) = part.get("functionCall") {
                    let name = func_call["name"].as_str().unwrap_or_default();
                    
                    // TOOL 1: Requisicao HTTP Externa
                    if name == "request_external_api" {
                        let args_val = &func_call["args"];
                        let args: FunctionCallArgs = serde_json::from_value(args_val.clone())
                            .map_err(|e| format!("Erro ao parsear argumentos da tool call: {}", e))?;

                        let tool_result = execute_external_api_call(config, &args).await?;

                        contents.push(candidate["content"].clone());
                        contents.push(json!({
                            "role": "user",
                            "parts": [{
                                "functionResponse": {
                                    "name": "request_external_api",
                                    "response": {
                                        "name": "request_external_api",
                                        "content": tool_result
                                    }
                                }
                            }]
                        }));

                        let second_payload = json!({
                            "system_instruction": { "parts": [{ "text": full_system_prompt }] },
                            "contents": contents
                        });

                        let second_res = client.post(&gemini_url).json(&second_payload).send().await
                            .map_err(|e| format!("Erro HTTP Gemini (2ª chamada): {}", e))?;
                        let second_json: Value = second_res.json().await
                            .map_err(|e| format!("Erro ao ler JSON final do Gemini: {}", e))?;

                        if let Some(final_text) = second_json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                            db.save_message(chat_id, "model", final_text);
                            return Ok(final_text.to_string());
                        }
                    } 
                    // TOOL 2: Busca Textual em Histórico (FTS5)
                    else if name == "search_past_conversations" {
                        let args_val = &func_call["args"];
                        let args: SearchConversationsArgs = serde_json::from_value(args_val.clone())
                            .map_err(|e| format!("Erro ao parsear argumentos da busca: {}", e))?;

                        let results = db.search_full_text(&args.query, args.chat_id.as_deref());
                        let search_json = json!({ "results": results }).to_string();

                        contents.push(candidate["content"].clone());
                        contents.push(json!({
                            "role": "user",
                            "parts": [{
                                "functionResponse": {
                                    "name": "search_past_conversations",
                                    "response": {
                                        "name": "search_past_conversations",
                                        "content": search_json
                                    }
                                }
                            }]
                        }));

                        let second_payload = json!({
                            "system_instruction": { "parts": [{ "text": full_system_prompt }] },
                            "contents": contents
                        });

                        let second_res = client.post(&gemini_url).json(&second_payload).send().await
                            .map_err(|e| format!("Erro HTTP Gemini (segunda chamada FTS): {}", e))?;
                        let second_json: Value = second_res.json().await
                            .map_err(|e| format!("Erro ao ler JSON final do Gemini: {}", e))?;

                        if let Some(final_text) = second_json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                            db.save_message(chat_id, "model", final_text);
                            return Ok(final_text.to_string());
                        }
                    }
                }

                if let Some(text) = part["text"].as_str() {
                    db.save_message(chat_id, "model", text);
                    return Ok(text.to_string());
                }
            }
        }
    }

    Err(format!("Resposta inesperada do Gemini: {:?}", res_json))
}
