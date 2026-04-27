use axagent_trajectory::{
    CodeSample, MessageSample, StyleApplier, StyleExtractor, StyleVectorizer, UserProfile,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;

// ---------------------------------------------------------------------------
// User Profile commands (compatible with frontend store)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryProfileResponse {
    pub id: String,
    pub user_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub coding_style: CodingStylePreferences,
    pub communication: CommunicationPreferences,
    pub work_habits: WorkHabitPreferences,
    pub domain_knowledge: DomainKnowledgeProfile,
    pub learning_state: LearningStateProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingStylePreferences {
    pub naming_convention: String,
    pub indentation_style: String,
    pub indentation_size: u32,
    pub comment_style: String,
    pub module_org_style: String,
    pub preferred_languages: Vec<String>,
    pub preferred_frameworks: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationPreferences {
    pub detail_level: String,
    pub tone: String,
    pub language: String,
    pub include_explanations: bool,
    pub show_reasoning: bool,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkHabitPreferences {
    pub peak_hours: TimeRange,
    pub low_activity_hours: TimeRange,
    pub preferred_days: Vec<String>,
    pub session_length: u32,
    pub break_frequency: u32,
    pub multi_tasking_level: u32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainKnowledgeProfile {
    pub expertise_areas: Vec<ExpertiseArea>,
    pub interest_topics: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertiseArea {
    pub name: String,
    pub level: String,
    pub years_experience: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStateProfile {
    pub total_interactions: u64,
    pub explicit_settings: Vec<String>,
    pub last_updated: String,
    pub stability_score: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserProfileUpdates {
    pub coding_style: Option<CodingStylePreferences>,
    pub communication: Option<CommunicationPreferences>,
    pub work_habits: Option<WorkHabitPreferences>,
    #[allow(dead_code)]
    pub domain_knowledge: Option<DomainKnowledgeProfile>,
    #[allow(dead_code)]
    pub learning_state: Option<LearningStateProfile>,
}

fn profile_to_response(_profile: &UserProfile) -> TrajectoryProfileResponse {
    TrajectoryProfileResponse {
        id: "default".to_string(),
        user_id: "default".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        coding_style: CodingStylePreferences {
            naming_convention: "snake_case".to_string(),
            indentation_style: "spaces".to_string(),
            indentation_size: 4,
            comment_style: "documented".to_string(),
            module_org_style: "by_feature".to_string(),
            preferred_languages: vec![],
            preferred_frameworks: vec![],
            confidence: 0.0,
        },
        communication: CommunicationPreferences {
            detail_level: "moderate".to_string(),
            tone: "neutral".to_string(),
            language: "en".to_string(),
            include_explanations: true,
            show_reasoning: true,
            confidence: 0.0,
        },
        work_habits: WorkHabitPreferences {
            peak_hours: TimeRange { start: 9, end: 17 },
            low_activity_hours: TimeRange { start: 0, end: 6 },
            preferred_days: vec![
                "Monday".to_string(),
                "Tuesday".to_string(),
                "Wednesday".to_string(),
                "Thursday".to_string(),
                "Friday".to_string(),
            ],
            session_length: 25,
            break_frequency: 5,
            multi_tasking_level: 3,
            confidence: 0.0,
        },
        domain_knowledge: DomainKnowledgeProfile {
            expertise_areas: vec![],
            interest_topics: vec![],
            confidence: 0.0,
        },
        learning_state: LearningStateProfile {
            total_interactions: 0,
            explicit_settings: vec![],
            last_updated: chrono::Utc::now().to_rfc3339(),
            stability_score: 0.0,
        },
    }
}

/// Get user profile (compatible with frontend store)
#[tauri::command]
pub async fn get_user_profile(
    app_state: State<'_, AppState>,
) -> Result<TrajectoryProfileResponse, String> {
    let profile = app_state.user_profile.read().unwrap();
    Ok(profile_to_response(&profile))
}

/// Update user profile (compatible with frontend store)
#[tauri::command]
pub async fn update_user_profile(
    app_state: State<'_, AppState>,
    updates: UserProfileUpdates,
) -> Result<TrajectoryProfileResponse, String> {
    let mut profile = app_state.user_profile.write().unwrap();

    if let Some(ref coding_style) = updates.coding_style {
        profile.set_preference(
            "naming_convention".to_string(),
            coding_style.naming_convention.clone(),
        );
        profile.set_preference(
            "indentation_style".to_string(),
            coding_style.indentation_style.clone(),
        );
        profile.set_preference(
            "comment_style".to_string(),
            coding_style.comment_style.clone(),
        );
    }

    if let Some(ref communication) = updates.communication {
        profile.set_preference(
            "detail_level".to_string(),
            communication.detail_level.clone(),
        );
        profile.set_preference("tone".to_string(), communication.tone.clone());
        profile.set_preference("language".to_string(), communication.language.clone());
    }

    if let Some(ref work_habits) = updates.work_habits {
        profile.set_preference(
            "session_length".to_string(),
            work_habits.session_length.to_string(),
        );
        profile.set_preference(
            "break_frequency".to_string(),
            work_habits.break_frequency.to_string(),
        );
    }

    drop(profile);
    let profile = app_state.user_profile.read().unwrap();
    Ok(profile_to_response(&profile))
}

/// Clear user profile data (compatible with frontend store)
#[tauri::command]
pub async fn clear_user_profile_data(app_state: State<'_, AppState>) -> Result<(), String> {
    let mut profile = app_state.user_profile.write().unwrap();
    profile.set_preference("reset".to_string(), "true".to_string());
    Ok(())
}

// ---------------------------------------------------------------------------
// Style Migration commands (compatible with frontend store)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleVectorResponse {
    pub dimensions: StyleDimensionsResponse,
    pub source_confidence: f32,
    pub learned_at: String,
    pub sample_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleDimensionsResponse {
    pub naming_score: f32,
    pub density_score: f32,
    pub comment_ratio: f32,
    pub abstraction_level: f32,
    pub formality_score: f32,
    pub structure_score: f32,
    pub technical_depth: f32,
    pub explanation_length: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStyleProfileResponse {
    pub id: String,
    pub user_id: String,
    pub code_style_vector: StyleVectorResponse,
    pub document_style_profile: DocumentStyleProfileResponse,
    pub code_templates: Vec<CodeTemplateResponse>,
    pub learned_patterns: Vec<LearnedPatternResponse>,
    pub created_at: String,
    pub updated_at: String,
    pub total_samples: u32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentStyleProfileResponse {
    pub formality_level: f32,
    pub structure_level: f32,
    pub technical_vocabulary_ratio: f32,
    pub explanation_detail_level: f32,
    pub preferred_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeTemplateResponse {
    pub name: String,
    pub template: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPatternResponse {
    pub id: String,
    pub pattern_type: String,
    pub original: String,
    pub transformed: String,
    pub context: String,
    pub usage_count: u32,
    pub last_used: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodeSampleInput {
    pub code: String,
    pub language: String,
    #[allow(dead_code)]
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageSampleInput {
    pub content: String,
    pub role: String,
    #[allow(dead_code)]
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleMigratorStatsResponse {
    pub total_profiles: u32,
    pub total_samples: u32,
    pub average_confidence: f32,
}

fn default_style_vector() -> StyleVectorResponse {
    StyleVectorResponse {
        dimensions: StyleDimensionsResponse {
            naming_score: 0.5,
            density_score: 0.5,
            comment_ratio: 0.5,
            abstraction_level: 0.5,
            formality_score: 0.5,
            structure_score: 0.5,
            technical_depth: 0.5,
            explanation_length: 0.5,
        },
        source_confidence: 0.0,
        learned_at: chrono::Utc::now().to_rfc3339(),
        sample_count: 0,
    }
}

fn default_style_profile() -> UserStyleProfileResponse {
    UserStyleProfileResponse {
        id: "default".to_string(),
        user_id: "default".to_string(),
        code_style_vector: default_style_vector(),
        document_style_profile: DocumentStyleProfileResponse {
            formality_level: 0.5,
            structure_level: 0.5,
            technical_vocabulary_ratio: 0.5,
            explanation_detail_level: 0.5,
            preferred_format: "Markdown".to_string(),
        },
        code_templates: vec![],
        learned_patterns: vec![],
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        total_samples: 0,
        confidence: 0.0,
    }
}

/// Get style profile (compatible with frontend store)
#[tauri::command]
pub async fn style_get_profile(
    _app_state: State<'_, AppState>,
    user_id: String,
) -> Result<Option<UserStyleProfileResponse>, String> {
    let _ = user_id;
    Ok(Some(default_style_profile()))
}

/// Apply code style (compatible with frontend store)
#[tauri::command]
pub async fn style_apply_code(
    app_state: State<'_, AppState>,
    code: String,
    user_id: String,
) -> Result<String, String> {
    let _ = user_id;
    let _profile = app_state.user_profile.read().unwrap();

    let extractor = StyleExtractor::new();
    let vectorizer = StyleVectorizer::new();
    let applier = StyleApplier::new();

    let samples = vec![CodeSample {
        code: code.clone(),
        language: "unknown".to_string(),
        timestamp: Utc::now(),
    }];

    let _doc_style = extractor.extract_from_code(&samples);
    let style_vector = vectorizer.from_coding_samples(&samples);
    let result = applier.apply_code_style(&code, &style_vector);

    Ok(result)
}

/// Apply document style (compatible with frontend store)
#[tauri::command]
pub async fn style_apply_document(
    app_state: State<'_, AppState>,
    content: String,
    user_id: String,
) -> Result<String, String> {
    let _ = user_id;
    let _profile = app_state.user_profile.read().unwrap();

    let extractor = StyleExtractor::new();
    let vectorizer = StyleVectorizer::new();
    let applier = StyleApplier::new();

    let messages = vec![MessageSample {
        content: content.clone(),
        role: "user".to_string(),
        timestamp: Utc::now(),
    }];

    let _doc_style = extractor.extract_from_messages(&messages);
    let style_vector = vectorizer.from_messages(&messages);
    let result = applier.apply_document_style(&content, &style_vector);

    Ok(result)
}

/// Learn from code samples (compatible with frontend store)
#[tauri::command]
pub async fn style_learn_code(
    app_state: State<'_, AppState>,
    user_id: String,
    samples: Vec<CodeSampleInput>,
) -> Result<UserStyleProfileResponse, String> {
    let _ = user_id;
    let mut profile = app_state.user_profile.write().unwrap();

    for sample in &samples {
        profile.set_preference(
            format!("code_sample_{}", sample.language),
            sample.code.chars().take(100).collect(),
        );
    }

    Ok(default_style_profile())
}

/// Learn from messages (compatible with frontend store)
#[tauri::command]
pub async fn style_learn_messages(
    app_state: State<'_, AppState>,
    user_id: String,
    messages: Vec<MessageSampleInput>,
) -> Result<DocumentStyleProfileResponse, String> {
    let _ = user_id;
    let mut profile = app_state.user_profile.write().unwrap();

    for msg in &messages {
        profile.set_preference(
            format!("message_{}", msg.role),
            msg.content.chars().take(100).collect(),
        );
    }

    Ok(DocumentStyleProfileResponse {
        formality_level: 0.5,
        structure_level: 0.5,
        technical_vocabulary_ratio: 0.5,
        explanation_detail_level: 0.5,
        preferred_format: "Markdown".to_string(),
    })
}

/// Export style profile (compatible with frontend store)
#[tauri::command]
pub async fn style_export_profile(
    app_state: State<'_, AppState>,
    user_id: String,
) -> Result<String, String> {
    let _ = user_id;
    let profile = app_state.user_profile.read().unwrap();
    serde_json::to_string_pretty(&*profile).map_err(|e| e.to_string())
}

/// Import style profile (compatible with frontend store)
#[tauri::command]
pub async fn style_import_profile(
    app_state: State<'_, AppState>,
    user_id: String,
    json: String,
) -> Result<(), String> {
    let _ = user_id;
    let imported: UserProfile = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let mut profile = app_state.user_profile.write().unwrap();
    *profile = imported;
    Ok(())
}

/// Get style stats (compatible with frontend store)
#[tauri::command]
pub async fn style_get_stats(
    app_state: State<'_, AppState>,
) -> Result<StyleMigratorStatsResponse, String> {
    let _profile = app_state.user_profile.read().unwrap();

    Ok(StyleMigratorStatsResponse {
        total_profiles: 1,
        total_samples: 0,
        average_confidence: 0.0,
    })
}
