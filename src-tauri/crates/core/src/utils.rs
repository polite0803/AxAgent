pub fn gen_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

const OUTPUT_LANGUAGE_TAG: &str = "<output-language>";

pub fn language_code_to_name(code: &str) -> &str {
    match code {
        "zh-CN" | "zh-TW" | "zh-Hans" | "zh-Hant" => "Chinese",
        "en-US" | "en-GB" | "en" => "English",
        "ja-JP" | "ja" => "Japanese",
        "ko-KR" | "ko" => "Korean",
        "ru" | "ru-RU" => "Russian",
        "fr" | "fr-FR" => "French",
        "de" | "de-DE" => "German",
        "es" | "es-ES" => "Spanish",
        "pt" | "pt-BR" | "pt-PT" => "Portuguese",
        "it" | "it-IT" => "Italian",
        "ar" | "ar-SA" => "Arabic",
        "th" | "th-TH" => "Thai",
        "vi" | "vi-VN" => "Vietnamese",
        "id" | "id-ID" => "Indonesian",
        other => other,
    }
}

pub fn build_output_language_directive(language_code: &str) -> String {
    let lang_name = language_code_to_name(language_code);
    format!(
        "{tag}\nIMPORTANT: You MUST respond in {lang_name}. All your output, including explanations, tool call reasoning, summaries, and any text directed to the user, must be written in {lang_name}. This is a strict requirement — do not switch to any other language unless the user explicitly asks you to.\n</output-language>",
        tag = OUTPUT_LANGUAGE_TAG,
        lang_name = lang_name,
    )
}

pub fn has_output_language_directive(content: &str) -> bool {
    content.contains(OUTPUT_LANGUAGE_TAG)
}

pub fn append_language_directive(system_prompt: &str, language_code: &str) -> String {
    if language_code.is_empty() || has_output_language_directive(system_prompt) {
        return system_prompt.to_string();
    }
    format!("{}\n\n{}", system_prompt, build_output_language_directive(language_code))
}
