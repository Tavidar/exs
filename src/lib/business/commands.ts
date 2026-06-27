// ELS (Business module) — typed wrappers around the Tauri commands.
// ერთადერთი ადგილი, სადაც ფრონტენდი ELS-ბექენდს იძახებს. არგუმენტები
// camelCase-ით გადაეცემა (Tauri ავტომატურად ხდის snake_case Rust-მხარეს).

import { invoke } from '@tauri-apps/api/core';
import type {
	BusinessProfileReport,
	DiscoveryQuestion,
	LaunchSession,
	SessionDetail
} from '$lib/business/types';

/** The full localized discovery interview (static content, no session needed). */
export async function businessDiscoveryQuestions(): Promise<DiscoveryQuestion[]> {
	return (await invoke('business_discovery_questions')) as DiscoveryQuestion[];
}

/** Begin a new entrepreneur launch journey. */
export async function businessStartSession(
	language: 'ka' | 'ru' | 'en' = 'ka',
	title?: string
): Promise<SessionDetail> {
	return (await invoke('business_start_session', { language, title })) as SessionDetail;
}

/** Full view of one launch session (session + answers + progress). */
export async function businessGetSession(sessionId: string): Promise<SessionDetail> {
	return (await invoke('business_get_session', { sessionId })) as SessionDetail;
}

/** All launch sessions, newest first. */
export async function businessListSessions(): Promise<LaunchSession[]> {
	return (await invoke('business_list_sessions')) as LaunchSession[];
}

/** Generate the Business Profile Report (tax + registration + AI narrative). */
export async function businessGenerateProfile(sessionId: string): Promise<BusinessProfileReport> {
	return (await invoke('business_generate_profile', { sessionId })) as BusinessProfileReport;
}

/** Record an answer to a discovery question (or skip an optional one). */
export async function businessSubmitAnswer(
	sessionId: string,
	questionKey: string,
	answerText: string | null,
	skipped = false
): Promise<SessionDetail> {
	return (await invoke('business_submit_answer', {
		sessionId,
		questionKey,
		answerText,
		skipped
	})) as SessionDetail;
}
