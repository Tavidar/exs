// ELS (Business module) — Action Island registration.
//
// მოდული მხოლოდ არეგისტრირებს მოქმედებებს (იხ. docs/ACTION_ISLAND.md). ბიზნეს-
// ლოგიკა ცალკე ტიპიზებულ სერვისშია (commands.ts), არა `run`-ში. ID-ები
// namespace-ით `business.*`. UI-ის ჩვენება ხდება `present` callback-ით,
// რომელსაც გვერდი აწვდის (notice-ბანერი) — ახალი მოდალების გარეშე.

import type { ActionDefinition } from '$lib/actions';
import {
	businessStartSession,
	businessGetSession,
	businessGenerateProfile
} from '$lib/business/commands';
import type { DiscoveryQuestion, SessionDetail } from '$lib/business/types';

/** The interview a user is currently in (enables the "continue" action). */
let activeSessionId: string | null = null;

function describeQuestion(q: DiscoveryQuestion): string {
	return `კითხვა ${q.index}/${q.total}: ${q.prompt}  ·  მაგ.: ${q.example}`;
}

/** Turn a session snapshot into one Georgian status line for the notice banner. */
function describe(detail: SessionDetail): string {
	activeSessionId = detail.session.id;
	const q = detail.progress.next_question;
	if (!q) {
		return `აღმოჩენის ეტაპი დასრულდა — შეგროვდა ${detail.progress.answered}/${detail.progress.total} პასუხი. შემდეგი: ბიზნეს-პროფილი.`;
	}
	return describeQuestion(q);
}

export interface BusinessActionDeps {
	/** Render a short Georgian status line (the page wires this to its notice banner). */
	present: (text: string) => void;
	/** Backend availability guard (ELS persistence needs Tauri). */
	isTauri: () => boolean;
}

/**
 * Business / ELS actions for the shared Action Island. Call from the page's
 * `onMount` registration and spread into the actions array.
 */
export function createBusinessActions({ present, isTauri }: BusinessActionDeps): ActionDefinition[] {
	return [
		{
			id: 'business.launch-els',
			label: 'ბიზნესის გაშვება',
			description: 'მეწარმის გაშვების სისტემა — ნაბიჯ-ნაბიჯ ინტერვიუ',
			icon: '◈',
			group: 'ბიზნესი',
			tone: 'accent',
			order: 100,
			enabled: () => isTauri(),
			run: async ({ close }) => {
				const detail = await businessStartSession('ka');
				present(describe(detail));
				close();
			}
		},
		{
			id: 'business.continue-els',
			label: 'ინტერვიუს გაგრძელება',
			description: 'მიმდინარე გაშვების სესიის შემდეგი კითხვა',
			icon: '◇',
			group: 'ბიზნესი',
			order: 110,
			visible: () => activeSessionId !== null,
			enabled: () => isTauri(),
			run: async ({ close }) => {
				if (!activeSessionId) return;
				const detail = await businessGetSession(activeSessionId);
				present(describe(detail));
				close();
			}
		},
		{
			id: 'business.generate-profile',
			label: 'ბიზნეს-პროფილის გენერაცია',
			description: 'ინტერვიუს მიხედვით: ტიპი, გადასახადები, რეგისტრაცია',
			icon: '◆',
			group: 'ბიზნესი',
			tone: 'success',
			order: 120,
			visible: () => activeSessionId !== null,
			enabled: () => isTauri(),
			run: async ({ close }) => {
				if (!activeSessionId) return;
				const report = await businessGenerateProfile(activeSessionId);
				const pct = Math.round(report.confidence * 100);
				present(
					`${report.tax.category_ka} · ${report.registration.recommended_path_ka} · სანდოობა ${pct}%`
				);
				close();
			}
		}
	];
}
