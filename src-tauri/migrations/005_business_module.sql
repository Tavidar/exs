-- ============================================================
-- EXSUL (new core): Business module — Entrepreneur Launch System (ELS)
-- Migration 005 — isolated business-launch tables
--
-- იზოლირებული მოდული. ეს მიგრაცია მხოლოდ ამატებს ცხრილებს და
-- არასოდეს ცვლის ბირთვის (Core) მონაცემებს (items / events / ...).
-- ყველა ცხრილი იდემპოტენტურია (CREATE TABLE IF NOT EXISTS) და
-- ცალკე ცხოვრობს Core-ის სქემისგან — Business Gateway-ის საზღვარი.
--
-- Изолированный модуль ELS: миграция ТОЛЬКО добавляет таблицы и
-- никогда не меняет данные ядра. Все сущности версионируются и
-- поддерживают эволюцию схемы (schema_version).
-- ============================================================

-- ── ბიზნესის გაშვების სესია (launch session) ────────────────
-- ერთი მეწარმის მთელი ELS-მოგზაურობა: აღმოჩენიდან რეგისტრაციამდე.
CREATE TABLE IF NOT EXISTS business_launch_session (
    id            TEXT PRIMARY KEY,
    -- მიმდინარე ეტაპი: discovery|profiling|brand|logo|registration|tax|banking|completed
    stage         TEXT NOT NULL DEFAULT 'discovery',
    status        TEXT NOT NULL DEFAULT 'active',   -- active|paused|completed|abandoned
    language      TEXT NOT NULL DEFAULT 'ka',
    title         TEXT,                              -- მომხმარებლის მიერ მინიჭებული სახელი
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- ── აღმოჩენის ინტერვიუს პასუხები (discovery answers) ────────
-- თანმიმდევრული კითხვა-პასუხი. ერთი პასუხი თითო კითხვის გასაღებზე.
CREATE TABLE IF NOT EXISTS business_discovery_answer (
    id            TEXT PRIMARY KEY,
    session_id    TEXT NOT NULL REFERENCES business_launch_session(id) ON DELETE CASCADE,
    question_key  TEXT NOT NULL,
    answer_text   TEXT,
    skipped       INTEGER NOT NULL DEFAULT 0,        -- 0|1 — გამოტოვებული პასუხი
    answered_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (session_id, question_key)
);

-- ── ვერსიონირებული ბიზნეს-სუბიექტები (versioned entities) ───
-- BusinessProfile | RegistrationProfile | TaxProfile | BrandProfile |
-- LogoProfile | BankProfile | MarketProfile | GrowthProfile.
-- თითო (session, kind, version) — ერთი ჩანაწერი. ბოლო ვერსია მოქმედია.
CREATE TABLE IF NOT EXISTS business_entity (
    id             TEXT PRIMARY KEY,
    session_id     TEXT NOT NULL REFERENCES business_launch_session(id) ON DELETE CASCADE,
    kind           TEXT NOT NULL,
    version        INTEGER NOT NULL DEFAULT 1,
    schema_version INTEGER NOT NULL DEFAULT 1,
    data_json      TEXT NOT NULL DEFAULT '{}',
    confidence     REAL,                             -- 0.0..1.0 — რეკომენდაციის სანდოობა
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (session_id, kind, version)
);

CREATE INDEX IF NOT EXISTS idx_business_answer_session
    ON business_discovery_answer (session_id);

CREATE INDEX IF NOT EXISTS idx_business_entity_session_kind
    ON business_entity (session_id, kind);
