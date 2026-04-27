use sea_orm::*;

use crate::entity::atomic_skills;
use crate::error::Result;
use crate::utils::now_ts;

pub async fn list_atomic_skills(
    db: &DatabaseConnection,
    category: Option<&str>,
    source: Option<&str>,
    enabled: Option<bool>,
) -> Result<Vec<atomic_skills::Model>> {
    let mut query = atomic_skills::Entity::find();

    if let Some(cat) = category {
        query = query.filter(atomic_skills::Column::Category.eq(cat));
    }
    if let Some(src) = source {
        query = query.filter(atomic_skills::Column::Source.eq(src));
    }
    if let Some(en) = enabled {
        query = query.filter(atomic_skills::Column::Enabled.eq(en));
    }

    let skills = query
        .order_by(atomic_skills::Column::UpdatedAt, Order::Desc)
        .all(db)
        .await?;
    Ok(skills)
}

pub async fn get_atomic_skill(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<atomic_skills::Model>> {
    let skill = atomic_skills::Entity::find_by_id(id).one(db).await?;
    Ok(skill)
}

pub async fn get_atomic_skill_by_name(
    db: &DatabaseConnection,
    name: &str,
) -> Result<Option<atomic_skills::Model>> {
    let skill = atomic_skills::Entity::find()
        .filter(atomic_skills::Column::Name.eq(name))
        .one(db)
        .await?;
    Ok(skill)
}

pub async fn create_atomic_skill(
    db: &DatabaseConnection,
    id: &str,
    name: &str,
    description: &str,
    input_schema: Option<&str>,
    output_schema: Option<&str>,
    entry_type: &str,
    entry_ref: &str,
    category: &str,
    tags: Option<&str>,
    version: &str,
    enabled: bool,
    source: &str,
) -> Result<()> {
    let now = now_ts();
    let model = atomic_skills::ActiveModel {
        id: Set(id.to_string()),
        name: Set(name.to_string()),
        description: Set(description.to_string()),
        input_schema: Set(input_schema.map(|s| s.to_string())),
        output_schema: Set(output_schema.map(|s| s.to_string())),
        entry_type: Set(entry_type.to_string()),
        entry_ref: Set(entry_ref.to_string()),
        category: Set(category.to_string()),
        tags: Set(tags.map(|s| s.to_string())),
        version: Set(version.to_string()),
        enabled: Set(enabled),
        source: Set(source.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    model.insert(db).await?;
    Ok(())
}

pub async fn update_atomic_skill(
    db: &DatabaseConnection,
    id: &str,
    name: Option<String>,
    description: Option<String>,
    input_schema: Option<Option<String>>,
    output_schema: Option<Option<String>>,
    entry_type: Option<String>,
    entry_ref: Option<String>,
    category: Option<String>,
    tags: Option<Option<String>>,
    version: Option<String>,
    enabled: Option<bool>,
    source: Option<String>,
) -> Result<bool> {
    let skill = atomic_skills::Entity::find_by_id(id).one(db).await?;
    if let Some(s) = skill {
        let mut active_model: atomic_skills::ActiveModel = s.into();
        if let Some(v) = name {
            active_model.name = Set(v);
        }
        if let Some(v) = description {
            active_model.description = Set(v);
        }
        if let Some(v) = input_schema {
            active_model.input_schema = Set(v);
        }
        if let Some(v) = output_schema {
            active_model.output_schema = Set(v);
        }
        if let Some(v) = entry_type {
            active_model.entry_type = Set(v);
        }
        if let Some(v) = entry_ref {
            active_model.entry_ref = Set(v);
        }
        if let Some(v) = category {
            active_model.category = Set(v);
        }
        if let Some(v) = tags {
            active_model.tags = Set(v);
        }
        if let Some(v) = version {
            active_model.version = Set(v);
        }
        if let Some(v) = enabled {
            active_model.enabled = Set(v);
        }
        if let Some(v) = source {
            active_model.source = Set(v);
        }
        active_model.updated_at = Set(now_ts());
        active_model.update(db).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn delete_atomic_skill(db: &DatabaseConnection, id: &str) -> Result<bool> {
    let skill = atomic_skills::Entity::find_by_id(id).one(db).await?;
    if let Some(s) = skill {
        s.delete(db).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn toggle_atomic_skill(db: &DatabaseConnection, id: &str, enabled: bool) -> Result<bool> {
    let skill = atomic_skills::Entity::find_by_id(id).one(db).await?;
    if let Some(s) = skill {
        let mut active_model: atomic_skills::ActiveModel = s.into();
        active_model.enabled = Set(enabled);
        active_model.updated_at = Set(now_ts());
        active_model.update(db).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Check semantic uniqueness: same entry_type + entry_ref + input_schema + output_schema
pub async fn check_semantic_uniqueness(
    db: &DatabaseConnection,
    entry_type: &str,
    entry_ref: &str,
    input_schema: Option<&str>,
    output_schema: Option<&str>,
) -> Result<Option<atomic_skills::Model>> {
    let mut query = atomic_skills::Entity::find()
        .filter(atomic_skills::Column::EntryType.eq(entry_type))
        .filter(atomic_skills::Column::EntryRef.eq(entry_ref));

    match input_schema {
        Some(s) => {
            query = query.filter(atomic_skills::Column::InputSchema.eq(s));
        }
        None => {
            query = query.filter(atomic_skills::Column::InputSchema.is_null());
        }
    }

    match output_schema {
        Some(s) => {
            query = query.filter(atomic_skills::Column::OutputSchema.eq(s));
        }
        None => {
            query = query.filter(atomic_skills::Column::OutputSchema.is_null());
        }
    }

    let existing = query.one(db).await?;
    Ok(existing)
}

/// Check name uniqueness
pub async fn check_name_uniqueness(
    db: &DatabaseConnection,
    name: &str,
) -> Result<Option<atomic_skills::Model>> {
    get_atomic_skill_by_name(db, name).await
}

#[derive(Debug, Clone)]
pub struct SkillMatch {
    pub existing_skill: atomic_skills::Model,
    pub similarity_score: f32,
    pub match_reasons: Vec<String>,
}

fn compute_word_set(text: &str) -> std::collections::HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty() && s.len() > 1)
        .map(|s| s.to_string())
        .collect()
}

fn jaccard_similarity(
    set1: &std::collections::HashSet<String>,
    set2: &std::collections::HashSet<String>,
) -> f32 {
    if set1.is_empty() && set2.is_empty() {
        return 1.0;
    }
    if set1.is_empty() || set2.is_empty() {
        return 0.0;
    }
    let intersection = set1.intersection(set2).count();
    let union = set1.union(set2).count();
    intersection as f32 / union as f32
}

pub async fn find_similar_skills(
    db: &DatabaseConnection,
    target_name: &str,
    target_description: &str,
    target_entry_type: &str,
    target_entry_ref: &str,
    target_category: &str,
    min_similarity: f32,
) -> Result<Vec<SkillMatch>> {
    let all_skills = list_atomic_skills(db, None, None, Some(true)).await?;

    let target_words_name = compute_word_set(target_name);
    let target_words_desc = compute_word_set(target_description);

    let mut matches: Vec<SkillMatch> = Vec::new();

    for skill in all_skills {
        let mut reasons = Vec::new();
        let mut score: f32 = 0.0;
        let mut weight_sum: f32 = 0.0;

        let name_words = compute_word_set(&skill.name);
        let desc_words = compute_word_set(&skill.description);

        let name_sim = jaccard_similarity(&target_words_name, &name_words);
        if name_sim > 0.3 {
            weight_sum += 0.4;
            score += name_sim * 0.4;
            if name_sim > 0.5 {
                reasons.push(format!("名称相似度: {:.0}%", name_sim * 100.0));
            }
        }

        let desc_sim = jaccard_similarity(&target_words_desc, &desc_words);
        if desc_sim > 0.2 {
            weight_sum += 0.3;
            score += desc_sim * 0.3;
            if desc_sim > 0.4 {
                reasons.push(format!("描述相似度: {:.0}%", desc_sim * 100.0));
            }
        }

        if skill.entry_type == target_entry_type && skill.entry_ref == target_entry_ref {
            weight_sum += 0.2;
            score += 0.2;
            reasons.push("相同入口类型和引用".to_string());
        }

        if skill.category == target_category && !target_category.is_empty() {
            weight_sum += 0.1;
            score += 0.1;
            reasons.push("相同分类".to_string());
        }

        let final_score = if weight_sum > 0.0 {
            score / weight_sum
        } else {
            0.0
        };

        if final_score >= min_similarity && !reasons.is_empty() {
            matches.push(SkillMatch {
                existing_skill: skill,
                similarity_score: final_score,
                match_reasons: reasons,
            });
        }
    }

    matches.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
    Ok(matches)
}
