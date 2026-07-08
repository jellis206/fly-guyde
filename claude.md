# Claude Instructions for Fly-Guyde Project

## Project Overview
This is a CS 452 class project building a natural language interface to a fly fishing product database. The project uses a Rust TUI application to query a SQLite database of 2,117 fly fishing products from Fly Fish Food using OpenAI's GPT API.

## Key Context Files
- **fly-guyde-context.md** (in $HOME): Comprehensive project context with all details, decisions, and progress
- **schema.sql**: Database schema (4 tables: products, variants, tags, product_tags)
- **flies.db**: Populated SQLite database with all product data

## Technology Stack
- **Language**: Rust
- **TUI Framework**: ratatui + crossterm
- **Database**: SQLite (rusqlite crate)
- **AI Integration**: OpenAI API via async-openai crate
- **Config**: .env file for OPENAI_API_KEY (gitignored)

## Core Requirements
1. Natural language to SQL query interface
2. Implement and compare **three prompting strategies** from paper: https://arxiv.org/abs/2305.11853
3. Document at least 6 examples (including 1+ success, 1+ failure)
4. Create schema visualization/ERD
5. Keep it simple and focused on the assignment requirements

## Architecture Guidelines
- **TUI Layout**: Input field → SQL display → Results table → Strategy selector
- **Query Flow**: User input → OpenAI (generate SQL) → SQLite query → OpenAI (format response) → Display
- **Error Handling**: Show SQL errors to user, log for debugging
- **Performance**: Keep it fast, cache schema context for OpenAI

## Database Schema (Quick Reference)
```
products (id, handle, title, product_type, vendor, description, price_min, price_max, available, image_url, dates)
├── variants (id, product_id, title, sku, price, available, inventory_quantity, option1-3)
├── product_tags (product_id, tag_id)
└── tags (id, name)
```

**View**: `product_details` - denormalized view with aggregated tags

## Development Priorities
1. **Keep it simple**: Don't over-engineer, this is a class project
2. **Focus on requirements**: Three prompting strategies are critical
3. **Document everything**: Examples, failures, strategy comparisons
4. **Make it usable**: TUI should be intuitive and responsive
5. **Budget conscious**: Keep OpenAI API costs low (estimated $0.29-$5 total)

## Common Queries to Support
- "Show me all dry flies under $5"
- "What streamers does Umpqua make?"
- "Find flies for euro nymphing"
- "Show the most expensive flies"
- "What flies are available from MFC?"
- "List all vendors and their product counts"

## Files Created So Far
- ✅ seed_db.py - Single script: scrape → clean → populate DB
- ✅ schema.sql - Database schema
- ✅ flies.db - Populated database (2,112 products, 5,094 variants, 206 tags)

## Files Still Needed
- ⏳ Cargo.toml - Rust dependencies
- ⏳ .env - OpenAI API key
- ⏳ .gitignore - Ignore .env, target/
- ⏳ src/main.rs - TUI entry point
- ⏳ src/db.rs - Database module
- ⏳ src/openai.rs - OpenAI integration + prompting strategies
- ⏳ src/ui.rs - Ratatui interface
- ⏳ README.md - Setup and usage instructions (mostly done)
- ⏳ EXAMPLES.md - 6+ query examples with results
- ⏳ schema.png - ERD visualization

## Coding Style Preferences
- **Rust**: Idiomatic Rust, use Result<T, E> for error handling
- **Comments**: Only where logic isn't self-evident
- **Modules**: Keep focused and single-responsibility
- **Testing**: Manual testing is fine for this project scope
- **Dependencies**: Prefer well-maintained crates

## Prompting Strategies Implementation Notes
Read the paper first: https://arxiv.org/abs/2305.11853

The three strategies will differ in how we prompt GPT to generate SQL:
1. Need to understand the differences from the paper
2. Implement as separate functions or enum variants
3. Allow user to toggle between them in TUI
4. Track and compare accuracy/quality of results
5. Document findings for class submission

## Cost Management
- Use GPT-3.5-turbo for development/testing
- Consider GPT-4 only for final testing if needed
- Keep prompts concise but include full schema
- Cache responses where possible
- Target: Stay under $5 total

## Class Submission Checklist
- [ ] Working code (GitHub repo)
- [ ] Schema picture/ERD
- [ ] 1-sentence project description
- [ ] Sample successful query + SQL + response
- [ ] Sample failed query + SQL + response
- [ ] File with 6+ additional examples
- [ ] Documentation comparing prompting strategies
- [ ] Each person has run with their own API key

## Quick Start Commands
```bash
# Seed database (one-time, or to refresh)
python3 seed_db.py

# Build & run
cargo build --release
echo "OPENAI_API_KEY=sk-..." > .env
cargo run --release
```

## Things to Avoid
- Don't over-engineer abstractions
- Don't add features beyond requirements
- Don't create unnecessary documentation
- Don't optimize prematurely
- Don't implement features "for later" - YAGNI
- Don't add cron/scheduled updates yet (nice-to-have)

## When Working on This Project
1. Check fly-guyde-context.md for detailed status
2. Focus on next incomplete todo item
3. Keep commits small and focused
4. Test each component before moving on
5. Document prompting strategy results as you go
6. Update context doc if anything significant changes

## OpenAI Integration Tips
- Include full schema in system prompt
- Provide example queries in few-shot format
- Ask for SQL in code blocks for easy parsing
- Request explanations for transparency
- Handle malformed SQL gracefully
- Show generated SQL to user for debugging
- SQL generation uses `gpt-4o`
- Summarization uses `gpt-4o-mini` (cheap, fast)

## UI/UX Principles
- Show what's happening (loading states)
- Display generated SQL for transparency
- Make errors helpful, not cryptic
- Allow easy strategy switching
- Keep interface uncluttered
- Keyboard shortcuts for power users

## Success Criteria
The project is done when:
1. TUI can accept natural language queries
2. All three prompting strategies are implemented
3. Queries generate valid SQL and return results
4. At least 6 examples documented (with 1+ failure)
5. Schema visualization created
6. Code is on GitHub with README
7. Prompting strategies are compared/documented

## Notes
- User already has OpenAI API key in environment
- User prefers Rust + TUI over Python/web
- Database is read-only (no updates through app)
- Project name is a pun: "fly-guyde" (fly guide)
- Keep it fun and educational!
