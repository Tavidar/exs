// ELS (Business module) — frontend types mirroring the Rust gateway contract.
// ველის სახელები snake_case-ია (Core-ის სტილი, serde-ს default სერიალიზაცია).

export type LaunchStage =
	| 'discovery'
	| 'profiling'
	| 'brand'
	| 'logo'
	| 'registration'
	| 'tax'
	| 'banking'
	| 'completed';

export interface LaunchSession {
	id: string;
	stage: string;
	status: string;
	language: string;
	title: string | null;
	created_at: string;
	updated_at: string;
}

export interface DiscoveryQuestion {
	key: string;
	index: number;
	total: number;
	prompt: string;
	purpose: string;
	explanation: string;
	example: string;
	optional: boolean;
}

export interface DiscoveryAnswer {
	question_key: string;
	answer_text: string | null;
	skipped: boolean;
	answered_at: string;
}

export interface DiscoveryProgress {
	session_id: string;
	stage: string;
	answered: number;
	total: number;
	complete: boolean;
	next_question: DiscoveryQuestion | null;
}

export interface SessionDetail {
	session: LaunchSession;
	answers: DiscoveryAnswer[];
	progress: DiscoveryProgress;
}

/** Structured error surfaced by every ELS command (Tauri `Err` payload). */
export interface BusinessError {
	code: 'not_found' | 'invalid_input' | 'storage' | 'gateway' | 'internal';
	message: string;
	retryable: boolean;
}

// ── Profiling / official Georgian reference data ────────────────

export interface TaxClassification {
	category: 'micro' | 'small' | 'standard';
	category_ka: string;
	rate_ka: string;
	vat_relevant: boolean;
	reasoning_ka: string;
	obligations_ka: string[];
	confidence: number;
}

export interface RegistrationStep {
	title_ka: string;
	detail_ka: string;
	institution_ka: string;
	cost_ka: string;
	duration_ka: string;
}

export interface RegistrationPlan {
	recommended_path_ka: string;
	institutions_ka: string[];
	required_documents_ka: string[];
	steps: RegistrationStep[];
	estimated_timeline_ka: string;
	estimated_cost_ka: string;
}

/** Business Profile Report (spec RESPONSE FORMAT). All content Georgian. */
export interface BusinessProfileReport {
	session_id: string;
	business_type_ka: string;
	industry_ka: string;
	scale_ka: string;
	risk_level_ka: string;
	growth_potential_ka: string;
	tax: TaxClassification;
	registration: RegistrationPlan;
	banking_requirements_ka: string[];
	marketing_requirements_ka: string[];
	analysis_ka: string;
	recommendations_ka: string[];
	confidence: number;
	risks_ka: string[];
	next_actions_ka: string[];
	required_documents_ka: string[];
	estimated_timeline_ka: string;
	estimated_cost_ka: string;
	/** "openai" | "gemini" | "claude" | "deterministic" */
	generated_by: string;
}
