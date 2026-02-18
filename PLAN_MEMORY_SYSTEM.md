# Phase 1: Memory System Overhaul

## Overview
Port spacebot's advanced memory features into StarkBot: vector embeddings, hybrid search (FTS5 + vector + graph via RRF), memory associations, importance decay, and background association discovery.

## New Files (11)
1. `stark-backend/src/memory/mod.rs` — Module root
2. `stark-backend/src/memory/embeddings.rs` — EmbeddingGenerator trait + OpenAI impl
3. `stark-backend/src/memory/vector_search.rs` — Brute-force cosine similarity
4. `stark-backend/src/memory/hybrid_search.rs` — RRF combining FTS5 + vector + graph
5. `stark-backend/src/memory/associations.rs` — AssociationType enum + helpers
6. `stark-backend/src/memory/decay.rs` — Importance decay + pruning
7. `stark-backend/src/memory/association_loop.rs` — Background auto-discovery
8. `stark-backend/src/db/tables/memory_embeddings.rs` — DB ops for embeddings
9. `stark-backend/src/db/tables/memory_associations.rs` — DB ops for associations
10. `stark-backend/src/tools/builtin/memory_associate.rs` — Agent tool: create associations
11. `stark-backend/src/tools/builtin/memory_graph.rs` — Agent tool: traverse graph

## Modified Files (11)
1. `stark-backend/src/db/sqlite.rs` — New tables + migration
2. `stark-backend/src/db/tables/mod.rs` — Register new modules
3. `stark-backend/src/main.rs` — Add `mod memory;`, spawn loops
4. `stark-backend/src/tools/types.rs` — Add `hybrid_search` to ToolContext
5. `stark-backend/src/tools/builtin/mod.rs` — Register new tools
6. `stark-backend/src/tools/mod.rs` — Register tools in registry
7. `stark-backend/src/tools/builtin/qmd_memory_search.rs` — Add `mode` param
8. `stark-backend/src/controllers/memory.rs` — Add 7 endpoints
9. `stark-backend/src/channels/dispatcher/mod.rs` — Propagate HybridSearchEngine
10. `stark-backend/Cargo.toml` — No new deps needed (using reqwest for OpenAI)

## Architecture
- **Vector search**: Brute-force cosine similarity (fast for <10k memories)
- **Embeddings**: OpenAI API via reqwest (already a dependency)
- **RRF formula**: `score = sum(1/(60+rank))` across FTS5, vector, graph
- **Association loop**: Background tokio task every 5 min
- **Decay**: Half-life 30 days, access boost +1, prune below importance 2
