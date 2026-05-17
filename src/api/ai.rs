use axum::{Json, response::IntoResponse, http::StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AiAssistRequest {
    pub grammar: String,
    pub request_type: String, // "explain_error" | "fix_ambiguity" | "suggest_ll1_transform"
    pub error_message: Option<String>,
    pub input_string: Option<String>,
    pub parser_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AiAssistResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transformed_grammar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

fn build_prompt(req: &AiAssistRequest) -> String {
    match req.request_type.as_str() {
        "explain_error" => {
            let error = req.error_message.as_deref().unwrap_or("error desconocido");
            let input = req.input_string.as_deref().unwrap_or("");
            let parser = req.parser_type.as_deref().unwrap_or("desconocido");
            format!(
                "Eres un experto en compiladores y teoría de lenguajes formales. \
                Explica en español, de forma clara y educativa (máximo 3 párrafos), \
                por qué falló el siguiente análisis sintáctico:\n\n\
                Gramática:\n{}\n\n\
                Cadena de entrada: \"{}\"\n\
                Algoritmo de parsing: {}\n\
                Mensaje de error del parser: \"{}\"\n\n\
                Explica: (1) qué significa el error, (2) por qué ocurre con esta gramática/entrada, \
                (3) qué token o producción causó el problema.",
                req.grammar, input, parser, error
            )
        }
        "fix_ambiguity" => {
            format!(
                "Eres un experto en compiladores. Analiza esta gramática libre de contexto y \
                devuelve tu respuesta en español con este formato EXACTO:\n\n\
                DIAGNÓSTICO: [explica en 1-2 oraciones qué problema tiene la gramática]\n\n\
                PROBLEMAS:\n- [problema 1]\n- [problema 2]\n\
                (lista todos los problemas: ambigüedad, recursión izquierda, etc.)\n\n\
                GRAMÁTICA CORREGIDA:\n[escribe la gramática corregida en formato: NT → producción]\n\n\
                EXPLICACIÓN: [explica qué transformaciones hiciste y por qué]\n\n\
                Gramática a analizar:\n{}",
                req.grammar
            )
        }
        "suggest_ll1_transform" => {
            format!(
                "Eres un experto en compiladores. Transforma esta gramática para que sea LL(1). \
                Devuelve tu respuesta en español con este formato EXACTO:\n\n\
                ANÁLISIS: [explica por qué la gramática actual NO es LL(1), si es que no lo es]\n\n\
                PASOS:\n1. [paso 1: ej. Eliminar recursión izquierda en X → ...]\n2. [paso 2: ej. Factorizar Y → ...]\n\
                (lista cada transformación necesaria)\n\n\
                GRAMÁTICA LL(1):\n[escribe la gramática transformada, una producción por línea en formato: NT → producción]\n\n\
                VERIFICACIÓN: [explica brevemente por qué la gramática resultante SÍ es LL(1)]\n\n\
                Gramática original:\n{}",
                req.grammar
            )
        }
        _ => format!("Analiza esta gramática libre de contexto y da recomendaciones en español:\n{}", req.grammar)
    }
}

fn parse_ai_response(request_type: &str, text: String) -> AiAssistResponse {
    match request_type {
        "explain_error" => AiAssistResponse {
            status: "success".to_string(),
            explanation: Some(text),
            suggestions: None,
            transformed_grammar: None,
            message: None,
        },
        "fix_ambiguity" => {
            // Extract GRAMÁTICA CORREGIDA section
            let transformed = extract_section(&text, "GRAMÁTICA CORREGIDA:");
            // Build suggestions list from PROBLEMAS section
            let problems_text = extract_section(&text, "PROBLEMAS:");
            let suggestions: Vec<String> = problems_text
                .lines()
                .filter(|l| l.trim_start().starts_with('-'))
                .map(|l| l.trim_start_matches('-').trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            AiAssistResponse {
                status: "success".to_string(),
                explanation: Some(text),
                suggestions: if suggestions.is_empty() { None } else { Some(suggestions) },
                transformed_grammar: if transformed.is_empty() { None } else { Some(transformed) },
                message: None,
            }
        }
        "suggest_ll1_transform" => {
            let transformed = extract_section(&text, "GRAMÁTICA LL(1):");
            AiAssistResponse {
                status: "success".to_string(),
                explanation: Some(text),
                suggestions: None,
                transformed_grammar: if transformed.is_empty() { None } else { Some(transformed) },
                message: None,
            }
        }
        _ => AiAssistResponse {
            status: "success".to_string(),
            explanation: Some(text),
            suggestions: None,
            transformed_grammar: None,
            message: None,
        }
    }
}

fn extract_section(text: &str, header: &str) -> String {
    if let Some(start) = text.find(header) {
        let after = &text[start + header.len()..];
        // Find next all-caps header or end of text
        let headers = ["DIAGNÓSTICO:", "PROBLEMAS:", "GRAMÁTICA CORREGIDA:", "EXPLICACIÓN:",
                        "ANÁLISIS:", "PASOS:", "GRAMÁTICA LL(1):", "VERIFICACIÓN:"];
        let mut end = after.len();
        for h in &headers {
            if let Some(pos) = after.find(h) {
                if pos > 0 && pos < end {
                    end = pos;
                }
            }
        }
        after[..end].trim().to_string()
    } else {
        String::new()
    }
}

pub async fn ai_assist(Json(payload): Json<AiAssistRequest>) -> impl IntoResponse {
    let api_key = match std::env::var("GROQ_API_KEY") {
        Ok(k) => k,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(AiAssistResponse {
            status: "error".to_string(),
            explanation: None,
            suggestions: None,
            transformed_grammar: None,
            message: Some("GROQ_API_KEY not configured in server environment".to_string()),
        })),
    };

    let prompt = build_prompt(&payload);

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": "llama-3.3-70b-versatile",
        "max_tokens": 1024,
        "messages": [{ "role": "user", "content": prompt }]
    });

    let response = match client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(AiAssistResponse {
            status: "error".to_string(),
            explanation: None,
            suggestions: None,
            transformed_grammar: None,
            message: Some(format!("Failed to reach Groq API: {}", e)),
        })),
    };

    let json: serde_json::Value = match response.json().await {
        Ok(j) => j,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(AiAssistResponse {
            status: "error".to_string(),
            explanation: None,
            suggestions: None,
            transformed_grammar: None,
            message: Some(format!("Failed to parse Groq API response: {}", e)),
        })),
    };

    let text = match json["choices"][0]["message"]["content"].as_str() {
        Some(t) => t.to_string(),
        None => {
            let error_msg = json["error"]["message"].as_str().unwrap_or("Empty response from Groq").to_string();
            return (StatusCode::OK, Json(AiAssistResponse {
                status: "error".to_string(),
                explanation: None,
                suggestions: None,
                transformed_grammar: None,
                message: Some(error_msg),
            }));
        }
    };

    (StatusCode::OK, Json(parse_ai_response(&payload.request_type, text)))
}
