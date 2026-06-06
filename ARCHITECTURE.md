# Architecture — ternary-forgiveness

> *Internal design and data flow.*

## Overview

This crate implements ternary {-1, 0, +1} semantics for the `forgiveness` domain.
It is one of ~280 ternary crates in the SuperInstance fleet, all sharing Z₃ arithmetic
from [ternary-core](https://github.com/SuperInstance/ternary-core).

## Core Types

- **`ForgivenessConfig`**
- **`Interaction`**
- **`Relationship`**
- **`ForgivenessEngine`**
- **`ErasureDetector`**
- **`ErasureReport`**

## Key Functions

- `from_score()`
- `score()`
- `new()`
- `state()`
- `total_interactions()`
- `cooperation_ratio()`
- `new()`
- `set_rps_cycle_length()`

## Ternary Mapping

| Value | Meaning |
|-------|---------|
| +1 | Active / positive |
| 0  | Neutral |
| -1 | Inactive / negative |

## Source Structure

1 Rust source file(s) in `src/`.
Language: Rust

## Cross-Repo References

- [ternary-core](https://github.com/SuperInstance/ternary-core) — shared Z₃ traits
- [ternary-types](https://github.com/SuperInstance/ternary-types) — type-level encodings
- [Full SuperInstance fleet](https://github.com/orgs/SuperInstance/repositories?q=ternary)
