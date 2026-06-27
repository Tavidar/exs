# CLAUDE.md

## Project Identity

This is a new standalone AI business assistant project. It is not the old Exsul application.

The old repository `https://github.com/Schreiry/Exsul` is used only as a donor for the working core: Rust/Tauri backend, SQLite database layer, migrations, inventory/product/card data logic, image/file storage, import/export, backup/restore, batch scripts and command patterns.

Do not recreate the old Exsul interface. Do not preserve old visual layouts. Do not rebuild the old dashboard/navigation/pages unless explicitly asked.

## Product Vision

The application is a desktop AI assistant for entrepreneurs, especially Georgian small and medium business owners.

It helps users:

* manage products and product cards;
* search products by name, description, parameters and photographs;
* analyze product photos;
* import and export business data;
* use AI to understand business data;
* later connect to Georgian public services, statistics sources, competitor data and external APIs;
* generate high-quality Georgian business text and localization.

The interface is not a traditional admin panel and not a normal chat. The main screen is the entire working context.

## Core UI Concept

The UI is a “void interface”: a minimal, deep, soft graphite contextual space.

Rules:

1. No traditional windows.
2. No heavy sidebars.
3. No normal chat bubbles.
4. No unnecessary modals.
5. No old Exsul screens.
6. The whole screen is the input/context surface.
7. Text, AI responses, search results, cards and database views appear as states of one continuous scene.
8. Product cards appear directly inside the main screen when relevant.
9. User input and AI output are visually distinct but part of one shared language.
10. The interface should feel deep, smooth, quiet, expensive and precise.

Visual direction:

* soft graphite grey;
* cold blue/green undertone;
* subtle glassmorphism;
* shader-like gradients;
* parallax depth;
* restrained particles;
* smooth physics-based transitions;
* high readability;
* low visual noise.

Performance has priority over decoration. If an effect risks freezes, reduce it or make it optional.

## Repository Branch Model

Expected branches:

* `core` — Rust/Tauri core, database, migrations, files, import/export, backup.
* `business` — business scenarios, analytics, external APIs, Georgian entrepreneur integrations.
* `interface` — new UI, visual system, animation, Svelte components.
* `ai` — AI Gateway, providers, prompts, vision, embeddings, Georgian localization.
* `dev` — integration branch.

Never mix unrelated branch responsibilities.

Core must not depend on UI.
UI must not directly mutate SQLite.
AI must not bypass typed services.
Business modules must not hardcode provider-specific AI calls.



# BUSINESS LAUNCH ASSISTANT

The platform must support entrepreneurs before the business exists.

The system must act as an intelligent business registration assistant for Georgia.

Its goal is to guide a future entrepreneur through the complete business creation process.

The assistant must not assume knowledge.

The assistant must collect all required information through structured interviews.

The assistant must ask questions one-by-one.

The assistant must explain why each question matters.

The assistant must determine:

- business activity;
- expected turnover;
- expected annual revenue;
- expected expenses;
- number of employees;
- legal structure;
- ownership structure;
- physical premises requirements;
- property ownership;
- business location;
- import/export activities;
- online/offline operation;
- VAT relevance;
- small business eligibility;
- micro business eligibility;
- future scaling expectations.

## Current Technical Foundation

Use the Exsul codebase as reference for:

* Tauri 2 application structure;
* Rust backend commands;
* SQLite/rusqlite database connection;
* migration runner;
* backup/recovery;
* import/export;
* image file saving;
* product/inventory commands;
* batch scripts;
* logging and startup safety.

Expected stack:

* Tauri 2
* Rust
* Svelte 5
* SvelteKit
* TypeScript
* Vite
* SQLite/rusqlite
* Serde
* Zod or equivalent frontend validation
* OpenAI API as primary AI provider
* Gemini API as fallback/vision/structured-output option
* Claude API optional
* JSON Schema for AI structured output
* FTS5 and optional vector search

## AI Architecture

Implement an AI Gateway. Do not call provider APIs directly from UI components.

Suggested structure:

```text
src-tauri/src/ai/
  mod.rs
  provider.rs
  router.rs
  types.rs
  prompts.rs
  openai.rs
  gemini.rs
  claude.rs
  vision.rs
  localization.rs
  embeddings.rs
```

The AI Gateway must:

1. Support provider abstraction.
2. Support OpenAI first.
3. Support Gemini fallback.
4. Optionally support Claude.
5. Accept text and image input.
6. Return structured JSON when UI needs structured results.
7. Never expose API keys to frontend.
8. Store secrets securely.
9. Use timeouts.
10. Return controlled errors.
11. Never send the whole database to AI.
12. Send only relevant products/images/context.
13. Support Georgian localization quality checks.

## Georgian Language Requirements

Georgian output must be high quality.

Do not perform literal translation.
Do meaning-based localization.

Rules:

* preserve intent;
* preserve business tone;
* avoid Russian calques;
* use natural Georgian phrasing;
* use Georgian business terminology;
* self-review Georgian output before showing it;
* when generating Georgian product descriptions, produce clean, natural, culturally appropriate text;
* when creating aliases for product search, include Georgian, Russian and English variants where useful.

## Product Search Requirements

The search system must match products by:

* product name;
* product description;
* category;
* parameters;
* price/stock/metadata;
* image caption;
* AI-generated tags;
* aliases;
* visual attributes;
* optional embeddings.

Example query: “джип”.

The system should find relevant products even if the product is named differently but the photo or description represents a jeep, toy car, SUV, vehicle, model, transport item or Georgian equivalent.

Search output should be structured:

```json
{
  "mode": "product_search",
  "query": "...",
  "results": [
    {
      "item_id": "...",
      "title": "...",
      "reason": "...",
      "confidence": 0.0,
      "matched_by": ["name", "description", "image", "ai_tags"]
    }
  ]
}
```

## Database Rules

Do not destroy existing user data.

When adding AI features, add new migrations. Prefer idempotent migrations.

Recommended AI metadata table:

```sql
CREATE TABLE IF NOT EXISTS ai_item_metadata (
    item_id TEXT PRIMARY KEY REFERENCES items(id) ON DELETE CASCADE,
    image_caption_ru TEXT,
    image_caption_ka TEXT,
    image_caption_en TEXT,
    tags_json TEXT NOT NULL DEFAULT '[]',
    visual_attributes_json TEXT NOT NULL DEFAULT '{}',
    aliases_json TEXT NOT NULL DEFAULT '[]',
    embedding_model TEXT,
    embedding_updated_at TEXT,
    ai_updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
```

Recommended FTS table:

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS item_search_fts USING fts5(
    item_id UNINDEXED,
    name,
    description,
    category,
    ai_tags,
    ai_caption,
    aliases
);
```

If the old schema has different table names, inspect it first and adapt instead of blindly pasting SQL.

## UI Implementation Rules

Use Svelte 5 and TypeScript.

Create a minimal scene-based UI:

```text
src/lib/scene/
src/lib/ai/
src/lib/tauri/
src/lib/components/void/
src/routes/+page.svelte
```

Suggested UI states:

* `idle`
* `typing`
* `thinking`
* `searching`
* `showing_results`
* `showing_product`
* `importing`
* `exporting`
* `error`

Main screen behavior:

1. User types anywhere in the central context field.
2. Text appears with smooth type-follow animation.
3. AI/search starts after command or Enter.
4. Input text becomes compact context.
5. Product cards emerge from the void.
6. Results can be filtered/refined without opening a new window.
7. Database actions appear contextually.
8. Escape or back gesture returns to quiet state.

Do not overbuild animation before core search works.

## Performance Rules

* No blocking UI.
* No full-size images in card grids.
* Generate thumbnails.
* Cache image metadata.
* Use CSS transforms and opacity.
* Avoid large-area blur animation.
* Use requestAnimationFrame for pointer/parallax loops.
* Respect reduced motion.
* Add low-performance mode.
* Use async Rust tasks for heavy file/AI operations.
* Keep DB queries paginated.
* Keep startup light.

## Security Rules

* Never commit API keys.
* Never log API keys.
* Never expose API keys to frontend.
* Do not send full database contents to providers.
* Add explicit provider settings.
* Use secure local storage or encrypted config.
* Sanitize imported files.
* Validate AI JSON before using it.
* Treat AI output as untrusted.
* Do not allow shell execution from AI output.
* Batch files must be simple developer utilities, not hidden magic traps.

## Development Workflow

Before editing:

1. Inspect existing file structure.
2. Identify which layer is affected.
3. Read related Rust commands and DB queries.
4. Read related Svelte components if UI is involved.
5. Preserve working logic.
6. Make small changes.
7. Run checks.

Useful commands:

```bash
npm install
npm run check
npm run build
cargo check
cargo test
npm run tauri dev
```

On Windows, preserve or adapt:

```bat
dev.bat
run.bat
build-release.bat
```

## Forbidden Behavior

Do not:

* rebuild the old Exsul UI;
* create huge unused architecture;
* invent APIs without checking docs;
* put business logic into Svelte components;
* put provider-specific AI logic into UI;
* hardcode secrets;
* break existing DB data;
* remove migrations carelessly;
* block the UI thread;
* add heavy animation before search works;
* create modals for everything;
* hallucinate Georgian text quality;
* pretend untested code is production-ready.

## First MVP Objective

Build the smallest working proof:

1. New app identity.
2. Exsul core extracted/adapted.
3. SQLite working.
4. Product records load.
5. Product images load.
6. Minimal void interface.
7. Natural language search input.
8. Local DB search.
9. AI Gateway skeleton.
10. One AI provider connected or mocked.
11. Image-to-tags metadata stored.
12. Product cards appear from main screen.
13. Build/check passes.

## Response Format for Claude

When completing a task, always report:

1. Files changed.
2. What was reused from Exsul.
3. What was intentionally not reused.
4. Commands run.
5. Build/check result.
6. Known risks.
7. Next recommended step.

Be practical. Build the thing. Do not decorate the corpse before checking whether it has a pulse.
