use std::collections::HashSet;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::models::skill::Skill;
use crate::services::skill_store::SkillStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub skill: Skill,
    pub score: f32,
    pub reason: String,
    pub reason_type: String, // "similar_content" | "co_occurrence" | "category_popular"
}

pub struct SkillRecommender<'a> {
    store: &'a SkillStore,
}

impl<'a> SkillRecommender<'a> {
    pub fn new(store: &'a SkillStore) -> Self {
        SkillRecommender { store }
    }

    pub fn recommend(&self, limit: usize) -> Result<Vec<Recommendation>, String> {
        let recent = self.store.get_recent_used_skills(30, 50)?;
        if recent.is_empty() {
            return self.cold_start(limit);
        }

        let used_ids: HashSet<String> = recent.iter().map(|(id, _)| id.clone()).collect();
        let user_tags = self.store.get_all_usage_tags()?;
        let user_tag_set: HashSet<&str> = user_tags.iter().map(|s| s.as_str()).collect();

        let all_skills = self.store.list_skills(None, None, None)?;
        let candidates: Vec<&Skill> = all_skills
            .iter()
            .filter(|s| !used_ids.contains(&s.id))
            .collect();

        let mut scored: Vec<Recommendation> = Vec::new();
        for candidate in candidates {
            let content_score = self.content_similarity(&user_tag_set, &candidate.tags);
            let collab_score = self.collaborative_score(&used_ids, &candidate.id)?;
            let pop_score = 0.0f32;

            let final_score = 0.7 * content_score + 0.2 * collab_score + 0.1 * pop_score;

            if final_score > 0.05 {
                let (reason_type, reason) = if content_score > collab_score {
                    (
                        "similar_content".to_string(),
                        format!("与你常用的 skill 有相似标签: {}", candidate.tags.join(", ")),
                    )
                } else {
                    (
                        "co_occurrence".to_string(),
                        "经常与你使用的 skill 一起出现".to_string(),
                    )
                };

                scored.push(Recommendation {
                    skill: candidate.clone(),
                    score: final_score,
                    reason,
                    reason_type,
                });
            }
        }

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored)
    }

    fn content_similarity(&self, user_tags: &HashSet<&str>, skill_tags: &[String]) -> f32 {
        if user_tags.is_empty() || skill_tags.is_empty() {
            return 0.0;
        }
        let skill_set: HashSet<&str> = skill_tags.iter().map(|s| s.as_str()).collect();
        let intersection = user_tags.iter().filter(|t| skill_set.contains(*t)).count();
        let union = user_tags.len() + skill_set.len() - intersection;
        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    fn collaborative_score(&self, used_ids: &HashSet<String>, candidate_id: &str) -> Result<f32, String> {
        let conn = Connection::open(self.store.db_path()).map_err(|e| e.to_string())?;
        let mut total_co = 0i64;
        for used_id in used_ids {
            let count: i64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(count), 0) FROM co_occurrence WHERE (skill_a = ?1 AND skill_b = ?2) OR (skill_a = ?2 AND skill_b = ?1)",
                    rusqlite::params![used_id.as_str(), candidate_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            total_co += count;
        }
        let s = if total_co > 0 {
            (total_co as f32).ln() / 10.0
        } else {
            0.0
        };
        Ok(s.min(1.0))
    }

    fn cold_start(&self, limit: usize) -> Result<Vec<Recommendation>, String> {
        let all = self.store.list_skills(None, None, None)?;
        let mut recommendations: Vec<Recommendation> = all
            .into_iter()
            .take(limit)
            .map(|s| {
                let cat = s.category.clone().unwrap_or_else(|| "未分类".to_string());
                Recommendation {
                    skill: s,
                    score: 0.1,
                    reason: format!("分类「{}」下的推荐", cat),
                    reason_type: "category_popular".to_string(),
                }
            })
            .collect();
        recommendations.truncate(limit);
        Ok(recommendations)
    }
}
