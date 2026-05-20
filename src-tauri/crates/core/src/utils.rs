use std::process::Command as StdCommand;

/// 创建不弹出控制台窗口的进程命令（Windows 专用）
#[cfg(windows)]
pub fn cmd(program: &str) -> StdCommand {
    use std::os::windows::process::CommandExt;
    let mut c = StdCommand::new(program);
    c.creation_flags(0x08000000); // CREATE_NO_WINDOW
    c
}

#[cfg(not(windows))]
pub fn cmd(program: &str) -> StdCommand {
    StdCommand::new(program)
}

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
    let thinking_emphasis = if lang_name == "Chinese" {
        "\nCRITICAL: Your internal reasoning process (thinking) must ALSO be in Chinese. When you use <think> tags or any thinking/reasoning mode, write ALL your thoughts, analysis, and problem-solving steps in Chinese. Never switch to English for thinking — use Chinese throughout your entire cognitive process."
    } else {
        ""
    };
    format!(
        "{tag}\nIMPORTANT: You MUST respond entirely in {lang_name}. All your output, including explanations, tool call reasoning, summaries, and any text directed to the user, must be written in {lang_name}. This is a strict requirement — do not switch to any other language unless the user explicitly asks you to.{thinking_emphasis}\n</output-language>",
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
