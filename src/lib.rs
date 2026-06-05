#![forbid(unsafe_code)]

//! # ternary-forgiveness
//!
//! Explicit forgiveness mechanics for trust systems.
//!
//! Configurable forgiveness rates, timing windows synchronized to RPS cycles,
//! relationship repair, and erasure-code embedding detection.

/// Trust state for a relationship
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrustState {
    /// Fully trusted
    Trusted,
    /// Partially trusted
    Partial,
    /// Not trusted
    Untrusted,
}

impl TrustState {
    pub fn from_score(score: f64) -> Self {
        if score >= 0.7 { TrustState::Trusted }
        else if score >= 0.3 { TrustState::Partial }
        else { TrustState::Untrusted }
    }

    pub fn score(&self) -> f64 {
        match self {
            TrustState::Trusted => 1.0,
            TrustState::Partial => 0.5,
            TrustState::Untrusted => 0.0,
        }
    }
}

/// Configuration for forgiveness mechanics
#[derive(Debug, Clone)]
pub struct ForgivenessConfig {
    /// Base forgiveness rate (how quickly trust recovers per step)
    pub forgiveness_rate: f64,
    /// Maximum trust score
    pub max_trust: f64,
    /// Minimum trust score
    pub min_trust: f64,
    /// Trust lost on defection
    pub defection_penalty: f64,
    /// Trust gained on cooperation
    pub cooperation_bonus: f64,
    /// Forgiveness timing window (in RPS cycles)
    pub forgiveness_window: usize,
    /// Number of consecutive cooperations needed for forgiveness to kick in
    pub forgiveness_threshold: usize,
    /// Whether forgiveness is enabled
    pub enabled: bool,
}

impl Default for ForgivenessConfig {
    fn default() -> Self {
        Self {
            forgiveness_rate: 0.05,
            max_trust: 1.0,
            min_trust: 0.0,
            defection_penalty: 0.3,
            cooperation_bonus: 0.1,
            forgiveness_window: 5,
            forgiveness_threshold: 3,
            enabled: true,
        }
    }
}

/// Action in a round
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Cooperate,
    Defect,
}

/// Record of an interaction
#[derive(Debug, Clone)]
pub struct Interaction {
    pub round: usize,
    pub action: Action,
    pub trust_before: f64,
    pub trust_after: f64,
}

/// A relationship between two agents
#[derive(Debug, Clone)]
pub struct Relationship {
    pub trust_score: f64,
    pub history: Vec<Interaction>,
    pub consecutive_cooperations: usize,
    pub consecutive_defections: usize,
    pub total_cooperations: usize,
    pub total_defections: usize,
    pub last_forgiveness_round: Option<usize>,
    pub in_forgiveness_window: bool,
}

impl Relationship {
    pub fn new(initial_trust: f64) -> Self {
        Self {
            trust_score: initial_trust,
            history: Vec::new(),
            consecutive_cooperations: 0,
            consecutive_defections: 0,
            total_cooperations: 0,
            total_defections: 0,
            last_forgiveness_round: None,
            in_forgiveness_window: false,
        }
    }

    pub fn state(&self) -> TrustState {
        TrustState::from_score(self.trust_score)
    }

    /// Total interactions
    pub fn total_interactions(&self) -> usize {
        self.total_cooperations + self.total_defections
    }

    /// Cooperation ratio
    pub fn cooperation_ratio(&self) -> f64 {
        if self.total_interactions() == 0 { return 1.0; }
        self.total_cooperations as f64 / self.total_interactions() as f64
    }
}

/// The forgiveness engine
pub struct ForgivenessEngine {
    config: ForgivenessConfig,
    relationships: Vec<Relationship>,
    current_round: usize,
    rps_cycle_length: usize,
}

impl ForgivenessEngine {
    pub fn new(config: ForgivenessConfig) -> Self {
        Self {
            config,
            relationships: Vec::new(),
            current_round: 0,
            rps_cycle_length: 3,
        }
    }

    /// Set the RPS cycle length for timing synchronization
    pub fn set_rps_cycle_length(&mut self, length: usize) {
        self.rps_cycle_length = length;
    }

    /// Add a new relationship
    pub fn add_relationship(&mut self, initial_trust: f64) -> usize {
        let idx = self.relationships.len();
        self.relationships.push(Relationship::new(initial_trust));
        idx
    }

    /// Get a relationship by index
    pub fn get_relationship(&self, idx: usize) -> Option<&Relationship> {
        self.relationships.get(idx)
    }

    /// Get all relationships
    pub fn relationships(&self) -> &[Relationship] {
        &self.relationships
    }

    /// Process an action in a relationship
    pub fn process_action(&mut self, rel_idx: usize, action: Action) -> f64 {
        if rel_idx >= self.relationships.len() {
            return 0.0;
        }

        let rel = &mut self.relationships[rel_idx];
        let trust_before = rel.trust_score;

        match action {
            Action::Cooperate => {
                rel.consecutive_cooperations += 1;
                rel.consecutive_defections = 0;
                rel.total_cooperations += 1;

                // Apply cooperation bonus
                let bonus = if self.config.enabled
                    && rel.consecutive_cooperations >= self.config.forgiveness_threshold {
                    // Enhanced bonus during forgiveness
                    self.config.cooperation_bonus * (1.0 + self.config.forgiveness_rate)
                } else {
                    self.config.cooperation_bonus
                };
                rel.trust_score = (rel.trust_score + bonus).min(self.config.max_trust);

                // Check if we're entering forgiveness window
                if rel.consecutive_cooperations >= self.config.forgiveness_threshold {
                    rel.in_forgiveness_window = true;
                    rel.last_forgiveness_round = Some(self.current_round);
                }
            }
            Action::Defect => {
                rel.consecutive_defections += 1;
                rel.consecutive_cooperations = 0;
                rel.total_defections += 1;
                rel.in_forgiveness_window = false;

                rel.trust_score = (rel.trust_score - self.config.defection_penalty).max(self.config.min_trust);
            }
        }

        rel.history.push(Interaction {
            round: self.current_round,
            action,
            trust_before,
            trust_after: rel.trust_score,
        });

        rel.trust_score
    }

    /// Apply passive forgiveness (time-based recovery)
    pub fn apply_forgiveness(&mut self) {
        if !self.config.enabled {
            return;
        }

        for rel in &mut self.relationships {
            if rel.trust_score < self.config.max_trust {
                // Check if we're in a forgiveness window
                let in_window = rel.in_forgiveness_window
                    || rel.consecutive_cooperations >= 2;

                // Check RPS cycle alignment
                let on_cycle = self.current_round % self.rps_cycle_length == 0;

                let rate = if in_window && on_cycle {
                    self.config.forgiveness_rate * 2.0 // Enhanced on cycle
                } else if in_window {
                    self.config.forgiveness_rate
                } else {
                    self.config.forgiveness_rate * 0.5 // Background forgiveness
                };

                rel.trust_score = (rel.trust_score + rate).min(self.config.max_trust);
            }
        }
    }

    /// Advance to the next round
    pub fn advance_round(&mut self) {
        self.current_round += 1;
        self.apply_forgiveness();
    }

    /// Get current round
    pub fn current_round(&self) -> usize {
        self.current_round
    }

    /// Check if a relationship is in active forgiveness mode
    pub fn is_forgiving(&self, rel_idx: usize) -> bool {
        self.relationships.get(rel_idx)
            .map(|r| r.in_forgiveness_window)
            .unwrap_or(false)
    }

    /// Repair a relationship after a defection spiral
    pub fn repair_relationship(&mut self, rel_idx: usize, repair_boost: f64) -> f64 {
        if rel_idx >= self.relationships.len() {
            return 0.0;
        }
        let rel = &mut self.relationships[rel_idx];
        rel.trust_score = (rel.trust_score + repair_boost).min(self.config.max_trust);
        rel.consecutive_defections = 0;
        rel.consecutive_cooperations = 1;
        rel.in_forgiveness_window = true;
        rel.last_forgiveness_round = Some(self.current_round);
        rel.trust_score
    }

    /// Compute the "compression advantage" of a forgiving genome
    /// (forgiving strategies can be encoded more compactly)
    pub fn compression_advantage(&self) -> f64 {
        let forgiving_count = self.relationships.iter()
            .filter(|r| r.in_forgiveness_window || r.consecutive_cooperations > 0)
            .count();
        if self.relationships.is_empty() {
            return 0.0;
        }
        let ratio = forgiving_count as f64 / self.relationships.len() as f64;
        // Compression advantage: forgiving strategies need fewer bits to encode
        // because they don't need retaliation tracking
        ratio * (1.0 - ratio).ln_1p().abs()
    }
}

/// Erasure code embedding detector
/// Detects if trust patterns embed erasure-code-like redundancy
pub struct ErasureDetector;

impl ErasureDetector {
    /// Check if a set of trust scores exhibits erasure-code-like patterns
    /// (high redundancy, ability to reconstruct from partial data)
    pub fn detect_embedding(scores: &[f64], threshold: f64) -> ErasureReport {
        if scores.len() < 3 {
            return ErasureReport {
                is_embedded: false,
                redundancy: 0.0,
                recoverable_fragments: 0,
                confidence: 0.0,
            };
        }

        let n = scores.len() as f64;
        let mean: f64 = scores.iter().sum::<f64>() / n;
        let variance: f64 = scores.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / n;

        // High redundancy = low variance relative to mean
        let redundancy = if mean.abs() > 1e-10 {
            1.0 - (variance.sqrt() / mean.abs()).min(1.0)
        } else {
            0.0
        };

        // Count recoverable fragments: values close to mean
        let recoverable = scores.iter()
            .filter(|&&s| (s - mean).abs() < threshold)
            .count();

        // Check for parity-like patterns
        let has_parity = scores.len() >= 3 && {
            let third = scores.len() / 3;
            let chunk_means: Vec<f64> = scores.chunks(third)
                .map(|chunk| chunk.iter().sum::<f64>() / chunk.len() as f64)
                .collect();
            chunk_means.iter().all(|&cm| (cm - mean).abs() < threshold * 2.0)
        };

        let is_embedded = redundancy > 0.8 && recoverable > scores.len() / 2 && has_parity;
        let confidence = if is_embedded { redundancy * 0.9 } else { redundancy * 0.3 };

        ErasureReport {
            is_embedded,
            redundancy,
            recoverable_fragments: recoverable,
            confidence,
        }
    }
}

/// Report from erasure code detection
#[derive(Debug, Clone)]
pub struct ErasureReport {
    pub is_embedded: bool,
    pub redundancy: f64,
    pub recoverable_fragments: usize,
    pub confidence: f64,
}

/// Sweep forgiveness rates and find optimal
pub fn forgiveness_rate_sweep(
    interactions: &[Action],
    rates: &[f64],
) -> Vec<(f64, f64)> {
    rates.iter().map(|&rate| {
        let mut config = ForgivenessConfig::default();
        config.forgiveness_rate = rate;
        let mut engine = ForgivenessEngine::new(config);
        let rel = engine.add_relationship(0.5);

        for &action in interactions {
            engine.process_action(rel, action);
            engine.advance_round();
        }

        let final_trust = engine.get_relationship(rel).unwrap().trust_score;
        (rate, final_trust)
    }).collect()
}

/// Find optimal forgiveness timing window
pub fn optimize_timing_window(
    interactions: &[Action],
    windows: &[usize],
) -> Vec<(usize, f64)> {
    windows.iter().map(|&window| {
        let mut config = ForgivenessConfig::default();
        config.forgiveness_window = window;
        let mut engine = ForgivenessEngine::new(config);
        let rel = engine.add_relationship(0.5);

        for &action in interactions {
            engine.process_action(rel, action);
            engine.advance_round();
        }

        let final_trust = engine.get_relationship(rel).unwrap().trust_score;
        (window, final_trust)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_state_from_score() {
        assert_eq!(TrustState::from_score(0.9), TrustState::Trusted);
        assert_eq!(TrustState::from_score(0.5), TrustState::Partial);
        assert_eq!(TrustState::from_score(0.1), TrustState::Untrusted);
    }

    #[test]
    fn test_relationship_new() {
        let rel = Relationship::new(0.5);
        assert!((rel.trust_score - 0.5).abs() < 1e-10);
        assert_eq!(rel.total_interactions(), 0);
    }

    #[test]
    fn test_cooperation_increases_trust() {
        let mut engine = ForgivenessEngine::new(ForgivenessConfig::default());
        let rel = engine.add_relationship(0.5);
        let new_trust = engine.process_action(rel, Action::Cooperate);
        assert!(new_trust > 0.5);
    }

    #[test]
    fn test_defection_decreases_trust() {
        let mut engine = ForgivenessEngine::new(ForgivenessConfig::default());
        let rel = engine.add_relationship(0.5);
        let new_trust = engine.process_action(rel, Action::Defect);
        assert!(new_trust < 0.5);
    }

    #[test]
    fn test_forgiveness_rate_sweep() {
        let interactions = vec![
            Action::Cooperate, Action::Cooperate, Action::Defect,
            Action::Cooperate, Action::Cooperate, Action::Cooperate,
        ];
        let rates = vec![0.01, 0.05, 0.1, 0.2];
        let results = forgiveness_rate_sweep(&interactions, &rates);
        assert_eq!(results.len(), 4);
        // Higher forgiveness rate should generally lead to higher final trust
        assert!(results.last().unwrap().1 >= results.first().unwrap().1);
    }

    #[test]
    fn test_timing_window_optimization() {
        let interactions = vec![
            Action::Cooperate, Action::Defect, Action::Cooperate,
            Action::Cooperate, Action::Cooperate, Action::Cooperate,
        ];
        let windows = vec![1, 3, 5, 10];
        let results = optimize_timing_window(&interactions, &windows);
        assert_eq!(results.len(), 4);
    }

    #[test]
    fn test_repair_after_defection_spiral() {
        let mut engine = ForgivenessEngine::new(ForgivenessConfig::default());
        let rel = engine.add_relationship(0.8);

        // Defection spiral
        for _ in 0..5 {
            engine.process_action(rel, Action::Defect);
        }
        let after_spiral = engine.get_relationship(rel).unwrap().trust_score;
        assert!(after_spiral < 0.3, "Trust should be low after spiral: {}", after_spiral);

        // Repair
        let repaired = engine.repair_relationship(rel, 0.5);
        assert!(repaired > after_spiral);
    }

    #[test]
    fn test_compression_advantage_empty() {
        let engine = ForgivenessEngine::new(ForgivenessConfig::default());
        assert_eq!(engine.compression_advantage(), 0.0);
    }

    #[test]
    fn test_compression_advantage_forgiving() {
        let mut engine = ForgivenessEngine::new(ForgivenessConfig::default());
        let rel = engine.add_relationship(0.5);
        for _ in 0..5 {
            engine.process_action(rel, Action::Cooperate);
            engine.advance_round();
        }
        let adv = engine.compression_advantage();
        assert!(adv >= 0.0);
    }

    #[test]
    fn test_passive_forgiveness() {
        let mut engine = ForgivenessEngine::new(ForgivenessConfig {
            forgiveness_rate: 0.1,
            ..Default::default()
        });
        let rel = engine.add_relationship(0.1);
        engine.advance_round();
        let trust = engine.get_relationship(rel).unwrap().trust_score;
        assert!(trust > 0.1, "Passive forgiveness should increase trust: {}", trust);
    }

    #[test]
    fn test_rps_cycle_sync() {
        let mut engine = ForgivenessEngine::new(ForgivenessConfig {
            forgiveness_rate: 0.1,
            ..Default::default()
        });
        engine.set_rps_cycle_length(3);
        let rel = engine.add_relationship(0.1);

        // Advance multiple rounds
        let mut trusts = vec![];
        for _ in 0..10 {
            engine.advance_round();
            trusts.push(engine.get_relationship(rel).unwrap().trust_score);
        }

        // Trust should increase monotonically with forgiveness
        for i in 1..trusts.len() {
            assert!(trusts[i] >= trusts[i - 1] || trusts[i - 1] >= 1.0);
        }
    }

    #[test]
    fn test_forgiveness_disabled() {
        let mut config = ForgivenessConfig::default();
        config.enabled = false;
        config.forgiveness_rate = 0.1;
        let mut engine = ForgivenessEngine::new(config);
        let rel = engine.add_relationship(0.1);
        engine.advance_round();
        let trust = engine.get_relationship(rel).unwrap().trust_score;
        assert!((trust - 0.1).abs() < 1e-10, "No forgiveness when disabled");
    }

    #[test]
    fn test_consecutive_tracking() {
        let mut engine = ForgivenessEngine::new(ForgivenessConfig::default());
        let rel = engine.add_relationship(0.5);

        engine.process_action(rel, Action::Cooperate);
        engine.process_action(rel, Action::Cooperate);
        assert_eq!(engine.get_relationship(rel).unwrap().consecutive_cooperations, 2);

        engine.process_action(rel, Action::Defect);
        assert_eq!(engine.get_relationship(rel).unwrap().consecutive_cooperations, 0);
        assert_eq!(engine.get_relationship(rel).unwrap().consecutive_defections, 1);
    }

    #[test]
    fn test_forgiveness_window_activation() {
        let config = ForgivenessConfig {
            forgiveness_threshold: 3,
            ..Default::default()
        };
        let mut engine = ForgivenessEngine::new(config);
        let rel = engine.add_relationship(0.5);

        engine.process_action(rel, Action::Cooperate);
        assert!(!engine.is_forgiving(rel));
        engine.process_action(rel, Action::Cooperate);
        assert!(!engine.is_forgiving(rel));
        engine.process_action(rel, Action::Cooperate);
        assert!(engine.is_forgiving(rel));
    }

    #[test]
    fn test_trust_bounded() {
        let mut engine = ForgivenessEngine::new(ForgivenessConfig {
            max_trust: 1.0,
            cooperation_bonus: 0.5,
            ..Default::default()
        });
        let rel = engine.add_relationship(0.9);
        let t = engine.process_action(rel, Action::Cooperate);
        assert!(t <= 1.0, "Trust should not exceed max: {}", t);
    }

    #[test]
    fn test_trust_bounded_below() {
        let mut engine = ForgivenessEngine::new(ForgivenessConfig {
            min_trust: 0.0,
            defection_penalty: 1.0,
            ..Default::default()
        });
        let rel = engine.add_relationship(0.1);
        let t = engine.process_action(rel, Action::Defect);
        assert!(t >= 0.0, "Trust should not go below min: {}", t);
    }

    #[test]
    fn test_erasure_detection_high_redundancy() {
        let scores = vec![0.95, 0.96, 0.94, 0.95, 0.97, 0.95];
        let report = ErasureDetector::detect_embedding(&scores, 0.1);
        assert!(report.redundancy > 0.5);
        assert_eq!(report.recoverable_fragments, 6);
    }

    #[test]
    fn test_erasure_detection_low_redundancy() {
        let scores = vec![0.1, 0.9, 0.3, 0.7, 0.2, 0.8];
        let report = ErasureDetector::detect_embedding(&scores, 0.1);
        assert!(report.redundancy < 0.5);
    }

    #[test]
    fn test_erasure_detection_too_few() {
        let scores = vec![0.5];
        let report = ErasureDetector::detect_embedding(&scores, 0.1);
        assert!(!report.is_embedded);
    }

    #[test]
    fn test_interaction_history() {
        let mut engine = ForgivenessEngine::new(ForgivenessConfig::default());
        let rel = engine.add_relationship(0.5);
        engine.process_action(rel, Action::Cooperate);
        engine.process_action(rel, Action::Defect);
        let r = engine.get_relationship(rel).unwrap();
        assert_eq!(r.history.len(), 2);
        assert_eq!(r.history[0].action, Action::Cooperate);
        assert_eq!(r.history[1].action, Action::Defect);
    }

    #[test]
    fn test_cooperation_ratio() {
        let mut engine = ForgivenessEngine::new(ForgivenessConfig::default());
        let rel = engine.add_relationship(0.5);
        engine.process_action(rel, Action::Cooperate);
        engine.process_action(rel, Action::Cooperate);
        engine.process_action(rel, Action::Defect);
        let r = engine.get_relationship(rel).unwrap();
        assert!((r.cooperation_ratio() - 2.0/3.0).abs() < 1e-10);
    }
}
