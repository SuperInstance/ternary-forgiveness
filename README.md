# ternary-forgiveness

Explicit forgiveness mechanics for trust systems.

Configurable forgiveness rates, forgiveness timing windows synchronized to RPS (rock-paper-scissors) cycles, relationship repair after defection spirals, and erasure-code embedding detection for trust patterns.

## Features

- **Configurable forgiveness rates**: Per-engine and per-relationship settings
- **RPS cycle synchronization**: Forgiveness enhanced at cycle boundaries
- **Relationship tracking**: Full interaction history with trust trajectories
- **Defection spiral recovery**: Explicit repair mechanics
- **Forgiveness window activation**: Threshold-based forgiveness triggering
- **Compression advantage**: Measure encoding efficiency of forgiving strategies
- **Erasure-code detection**: Identify redundant trust patterns
- **Rate sweep & optimization**: Find optimal forgiveness parameters

## Usage

```rust
use ternary_forgiveness::{ForgivenessEngine, ForgivenessConfig, Action};

let config = ForgivenessConfig {
    forgiveness_rate: 0.05,
    defection_penalty: 0.3,
    cooperation_bonus: 0.1,
    forgiveness_threshold: 3,
    ..Default::default()
};

let mut engine = ForgivenessEngine::new(config);
engine.set_rps_cycle_length(3);

let rel = engine.add_relationship(0.5);

// Interact
engine.process_action(rel, Action::Cooperate);
engine.process_action(rel, Action::Defect);
engine.advance_round();

// Repair after spiral
engine.repair_relationship(rel, 0.4);

// Check state
let r = engine.get_relationship(rel).unwrap();
println!("Trust: {:.3}, State: {:?}", r.trust_score, r.state());
```

## Test Coverage

21 tests covering trust mechanics, forgiveness rates, timing windows, defection spirals, RPS synchronization, erasure detection, compression advantage, and edge cases.

## Known Limitations

- Single-agent perspective only (no multi-agent game theory)
- Forgiveness is passive (time-based), not negotiation-based
- Erasure-code detection uses simple statistical heuristics, not actual coding theory
- No support for asymmetric forgiveness (different rates per direction)
- Compression advantage metric is heuristic, not information-theoretically rigorous
- Trust bounds are hard thresholds, not probabilistic
- No support for third-party reputation or gossip

## License

MIT
