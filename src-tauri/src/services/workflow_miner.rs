use crate::models::skill::{
    InvocationChain, WorkflowStatus, WorkflowStep, WorkflowTemplate,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub struct WorkflowMiner {
    pub min_support: u32,
    pub min_length: u32,
    pub max_length: u32,
}

impl WorkflowMiner {
    pub fn new() -> Self {
        WorkflowMiner {
            min_support: 2,
            min_length: 2,
            max_length: 8,
        }
    }

    pub fn mine(&self, chains: &[InvocationChain]) -> Vec<WorkflowTemplate> {
        let mut subseq_sessions: HashMap<Vec<String>, Vec<String>> = HashMap::new();

        for chain in chains {
            let seq = &chain.skill_sequence;
            for window_size in self.min_length..=self.max_length.min(seq.len() as u32) {
                for window in seq.windows(window_size as usize) {
                    let sub: Vec<String> = window.to_vec();
                    subseq_sessions
                        .entry(sub)
                        .or_default()
                        .push(chain.session_id.clone());
                }
            }
        }

        let frequent: Vec<(Vec<String>, Vec<String>)> = subseq_sessions
            .into_iter()
            .filter(|(_, sessions)| sessions.len() >= self.min_support as usize)
            .collect();

        let maximal = self.filter_maximal(&frequent);

        let mut workflows: Vec<WorkflowTemplate> = maximal
            .into_iter()
            .map(|(seq, sessions)| {
                let frequency = sessions.len() as u32;
                let confidence = self.compute_confidence(&seq, chains);
                let steps = self.build_steps(&seq);
                let name = self.generate_name(&seq);
                let description = format!("自动发现的工作流模式，出现 {} 次", frequency);

                let id_input = seq.join("→");
                let id = Self::sha256_hex(id_input.as_bytes());

                WorkflowTemplate {
                    id,
                    name,
                    description,
                    steps,
                    frequency,
                    confidence,
                    source_sessions: sessions,
                    category: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    status: WorkflowStatus::Discovered,
                }
            })
            .collect();

        workflows.sort_by(|a, b| {
            let score_a = a.frequency as f32 * a.confidence;
            let score_b = b.frequency as f32 * b.confidence;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        workflows
    }

    fn compute_confidence(&self, seq: &[String], chains: &[InvocationChain]) -> f32 {
        if seq.is_empty() {
            return 0.0;
        }
        let first = &seq[0];
        let total_first = chains
            .iter()
            .filter(|c| c.skill_sequence.contains(first))
            .count();
        let seq_count = chains
            .iter()
            .filter(|c| Self::contains_subsequence(&c.skill_sequence, seq))
            .count();
        if total_first == 0 {
            0.0
        } else {
            seq_count as f32 / total_first as f32
        }
    }

    fn build_steps(&self, seq: &[String]) -> Vec<WorkflowStep> {
        seq.iter()
            .enumerate()
            .map(|(i, name)| WorkflowStep {
                order: i as u32,
                skill_name: name.clone(),
                skill_id: None,
                is_optional: false,
                avg_position: i as f32,
                co_occurrence_ratio: 1.0,
            })
            .collect()
    }

    fn generate_name(&self, seq: &[String]) -> String {
        if seq.len() <= 3 {
            seq.join(" → ")
        } else {
            format!(
                "{} → ... → {} ({}步)",
                seq[0],
                seq[seq.len() - 1],
                seq.len()
            )
        }
    }

    fn filter_maximal(
        &self,
        frequent: &[(Vec<String>, Vec<String>)],
    ) -> Vec<(Vec<String>, Vec<String>)> {
        frequent
            .iter()
            .filter(|(seq, _)| {
                !frequent.iter().any(|(other, _)| {
                    other.len() > seq.len() && Self::contains_subsequence(other, seq)
                })
            })
            .cloned()
            .collect()
    }

    fn contains_subsequence(haystack: &[String], needle: &[String]) -> bool {
        if needle.len() > haystack.len() {
            return false;
        }
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    fn sha256_hex(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}
