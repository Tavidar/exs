<script lang="ts">
	import { onMount } from 'svelte';
	import VoidBackground from '$lib/components/VoidBackground.svelte';
	import WindowChrome from '$lib/components/WindowChrome.svelte';
	import ContextInput from '$lib/components/ContextInput.svelte';
	import ResultScene from '$lib/components/ResultScene.svelte';
	import { ActionIsland } from '$lib/components/actions';
	import { actionIsland, registerActions } from '$lib/actions';
	import { createBusinessActions } from '$lib/business/actions';
	import { scene } from '$lib/scene/scene.svelte';
	import { t } from '$lib/i18n';
	import {
		aiGetStatus,
		clearContextFiles,
		exportBackup,
		forgetContextFile,
		isTauri,
		rebuildSearchIndex,
		seedDemoItems,
		selectContextFiles
	} from '$lib/tauri/commands';
	import type { ContextFileMetadata } from '$lib/types';

	let draft = $state('');
	let provider = $state<string | null>(null);
	let notice = $state('');
	let attachedFiles = $state<ContextFileMetadata[]>([]);
	let pointerFrame = 0;
	let pointerX = 0;
	let pointerY = 0;
	let composer: { focusInput: (selectEnd?: boolean) => void } | null = $state(null);
	let historyViewport: HTMLElement | null = $state(null);

	const hasTurns = $derived(scene.turns.length > 0);
	const latestIndex = $derived(scene.turns.length - 1);

	function focusComposer() {
		requestAnimationFrame(() => composer?.focusInput());
	}

	function submit(q: string) {
		const query = q.trim();
		if (!query || scene.mode === 'searching') return;
		draft = '';
		const contextFileIds = attachedFiles.map((file) => file.selection_id);
		void scene.submit(query, contextFileIds).finally(focusComposer);
	}

	function removeAttachment(selectionId: string) {
		attachedFiles = attachedFiles.filter((file) => file.selection_id !== selectionId);
		if (isTauri()) void forgetContextFile(selectionId);
		focusComposer();
	}

	async function clearAttachments() {
		attachedFiles = [];
		if (isTauri()) await clearContextFiles();
	}

	async function pickAttachments() {
		const selection = await selectContextFiles();
		if (selection.cancelled) return;

		const known = new Map(
			attachedFiles.map((file) => [
				`${file.file_name}:${file.size_bytes}:${file.modified_at ?? ''}`,
				file
			])
		);
		for (const file of selection.files) {
			const key = `${file.file_name}:${file.size_bytes}:${file.modified_at ?? ''}`;
			const duplicate = known.get(key);
			if (duplicate) {
				void forgetContextFile(file.selection_id);
			} else {
				known.set(key, file);
			}
		}
		const accepted = [...known.values()];
		attachedFiles = accepted.slice(0, 10);
		for (const extra of accepted.slice(10)) {
			void forgetContextFile(extra.selection_id);
		}
		const rejected = selection.rejected.length;
		notice = rejected
			? t('files.attachedRejected', { count: selection.files.length, rejected })
			: t('files.attached', { count: attachedFiles.length });
		focusComposer();
	}

	// Pointer work is coalesced to one style update per frame.
	function onMove(event: PointerEvent) {
		pointerX = event.clientX;
		pointerY = event.clientY;
		if (pointerFrame) return;
		pointerFrame = requestAnimationFrame(() => {
			pointerFrame = 0;
			const nx = (pointerX / window.innerWidth) * 2 - 1;
			const ny = (pointerY / window.innerHeight) * 2 - 1;
			const root = document.documentElement.style;
			root.setProperty('--px', nx.toFixed(3));
			root.setProperty('--py', ny.toFixed(3));
		});
	}

	// The whole scene is the input. A printable key starts composing immediately.
	function captureKeyboard(event: KeyboardEvent) {
		if (
			event.defaultPrevented ||
			actionIsland.isOpen ||
			event.altKey ||
			event.ctrlKey ||
			event.metaKey ||
			event.isComposing
		) {
			return;
		}
		const target = event.target as HTMLElement | null;
		if (
			target?.matches('textarea, input, select, button, [contenteditable="true"]') ||
			target?.closest('[contenteditable="true"]')
		) {
			return;
		}
		if (event.key.length !== 1) return;
		event.preventDefault();
		draft += event.key;
		focusComposer();
	}

	$effect(() => {
		const count = scene.turns.length;
		if (!count || !historyViewport) return;
		requestAnimationFrame(() => {
			historyViewport?.scrollTo({
				top: historyViewport.scrollHeight,
				behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth'
			});
		});
	});

	onMount(() => {
		const unregister = registerActions(
			[
				{
					id: 'context.focus',
					label: t('actions.focus'),
					description: t('actions.focusDescription'),
					icon: '⌁',
					group: t('actions.contextGroup'),
					tone: 'accent',
					order: 10,
					dismiss: 'immediate',
					run: () => focusComposer()
				},
				{
					id: 'files.attach',
					label: t('actions.attachFiles'),
					description: t('actions.attachFilesDescription'),
					icon: '◇',
					group: t('actions.aiContextGroup'),
					tone: 'accent',
					order: 30,
					enabled: () => isTauri(),
					badge: () =>
						attachedFiles.length
							? t('files.inContext', { count: attachedFiles.length })
							: undefined,
					run: pickAttachments
				},
				{
					id: 'files.clear',
					label: t('actions.clearFiles'),
					description: t('actions.clearFilesDescription'),
					icon: '⊘',
					group: t('actions.aiContextGroup'),
					order: 40,
					visible: () => attachedFiles.length > 0,
					run: clearAttachments
				},
				{
					id: 'context.clear',
					label: t('actions.newContext'),
					description: t('actions.newContextDescription'),
					icon: '◌',
					group: t('actions.contextGroup'),
					order: 20,
					run: async () => {
						scene.clearHistory();
						draft = '';
						await clearAttachments();
						focusComposer();
					}
				},
				{
					id: 'search.rebuild',
					label: t('actions.refreshSearch'),
					description: t('actions.refreshSearchDescription'),
					icon: '⌕',
					group: t('actions.dataGroup'),
					tone: 'success',
					order: 50,
					enabled: () => isTauri(),
					run: async () => {
						const count = await rebuildSearchIndex();
						notice = t('system.indexUpdated', { count });
					}
				},
				{
					id: 'data.backup',
					label: t('actions.createBackup'),
					description: t('actions.createBackupDescription'),
					icon: '⬡',
					group: t('actions.dataGroup'),
					tone: 'neutral',
					order: 60,
					enabled: () => isTauri(),
					run: async () => {
						await exportBackup();
						notice = t('system.backupReady');
					}
				},
				...createBusinessActions({ present: (message) => (notice = message), isTauri })
			],
			{ replace: true }
		);

		// Startup stays visually immediate: backend conveniences run after first paint.
		const idle = window.setTimeout(() => {
			if (!isTauri()) {
				provider = 'mock';
				return;
			}
			void seedDemoItems().catch(() => {});
			void aiGetStatus()
				.then((status) => (provider = status.selected))
				.catch(() => {});
		}, 0);

		focusComposer();
		return () => {
			window.clearTimeout(idle);
			cancelAnimationFrame(pointerFrame);
			unregister();
		};
	});
</script>

<svelte:window onpointermove={onMove} onkeydown={captureKeyboard} />

<VoidBackground />
<WindowChrome
	labels={{
		titlebar: 'Exsul-ის ფანჯრის ზედა ზოლი',
		controls: 'ფანჯრის მართვა',
		minimize: 'ჩაკეცვა',
		maximize: 'გაშლა',
		restore: 'აღდგენა',
		close: 'დახურვა'
	}}
/>

<main class="stage" class:has-turns={hasTurns} class:writing={draft.length > 0}>
	<header class="hero">
		<span class="kicker">CONTEXTUAL INTELLIGENCE</span>
		<h1>{t('app.title')}</h1>
		<p>{t('app.tagline')}</p>
	</header>

	<section
		bind:this={historyViewport}
		class="memory"
		class:empty={!hasTurns && scene.mode !== 'searching' && scene.mode !== 'error'}
		aria-label="კონტექსტის ისტორია"
		aria-live="polite"
	>
		<div class="memory-stream">
			{#each scene.turns as turn, index (turn.id)}
				<article class="turn" class:archived={index !== latestIndex}>
					<div class="prompt">
						<span>თქვენ · {String(index + 1).padStart(2, '0')}</span>
						<p>{turn.query}</p>
					</div>
					{#if index === latestIndex}
						<ResultScene response={turn.response} />
					{:else}
						<div class="memory-summary">
							<span>{t('results.found', { count: turn.response.search.results.length })}</span>
							<p>{turn.response.answer?.text || turn.response.search.assistant_summary}</p>
						</div>
					{/if}
				</article>
			{/each}

			{#if scene.mode === 'searching'}
				<article class="turn pending">
					<div class="prompt">
						<span>თქვენ · ახლა</span>
						<p>{scene.query}</p>
					</div>
					<div class="thinking">
						<span></span>
						<p>{t('state.thinking')}</p>
					</div>
				</article>
			{:else if scene.mode === 'error'}
				<div class="scene-error" role="alert">
					<span>{t('state.error')}</span>
					<p>{scene.error}</p>
				</div>
			{/if}
		</div>
	</section>

	<section class="composer-zone" aria-label="კონტექსტის ველი">
		<ContextInput
			bind:this={composer}
			bind:value={draft}
			busy={scene.mode === 'searching'}
			attachments={attachedFiles}
			onremoveattachment={removeAttachment}
			onsubmit={submit}
		/>
	</section>

	<footer class="scene-meta">
		<div class="provider">
			<span class="provider-dot"></span>
			AI · {provider ?? '…'}
		</div>
		{#if notice}<span class="notice">{notice}</span>{/if}
		<button type="button" onclick={() => actionIsland.open('api')}>
			<span>{t('actions.open')}</span>
			<kbd>ALT + C</kbd>
		</button>
	</footer>
</main>

<ActionIsland
	title={t('actions.title')}
	hint={t('actions.hint')}
	emptyLabel={t('actions.empty')}
	closeLabel="ქმედებების სივრცის დახურვა"
	navigationHint="↑ ↓ ნავიგაცია · Enter არჩევა · Alt+C გახსნა"
/>

<style>
	.stage {
		position: relative;
		z-index: 1;
		width: 100vw;
		height: 100vh;
		overflow: hidden;
	}

	.hero {
		position: absolute;
		z-index: 2;
		top: clamp(5.8rem, 12vh, 8.2rem);
		left: 50%;
		display: grid;
		justify-items: center;
		width: min(900px, 90vw);
		text-align: center;
		transform: translateX(-50%);
		transition:
			opacity var(--dur) var(--ease-soft),
			transform var(--dur-slow) var(--ease-out);
	}
	.kicker {
		color: var(--accent);
		font-family: var(--font-display);
		font-size: 0.65rem;
		font-weight: 650;
		letter-spacing: 0.28em;
	}
	.hero h1 {
		margin-top: 0.3rem;
		color: var(--text-strong);
		font-family: var(--font-display);
		font-size: clamp(3.5rem, 8.5vw, 7.8rem);
		font-weight: 560;
		line-height: 0.9;
		letter-spacing: 0.18em;
		text-indent: 0.18em;
		text-transform: uppercase;
		text-shadow:
			0 4px 44px rgba(126, 232, 219, 0.14),
			0 30px 80px rgba(0, 0, 0, 0.32);
	}
	.hero p {
		margin-top: 0.85rem;
		color: var(--text-soft);
		font-size: clamp(0.85rem, 1.4vw, 1.05rem);
		font-weight: 530;
		letter-spacing: 0.08em;
	}
	.has-turns .hero {
		opacity: 0;
		transform: translate(-50%, -20px) scale(0.98);
		pointer-events: none;
	}
	.writing .hero {
		opacity: 0;
	}

	.memory {
		position: absolute;
		z-index: 1;
		inset: 4.7rem 0 48%;
		padding: 1rem max(3vw, 1rem) 3rem;
		overflow: auto;
		overscroll-behavior: contain;
		mask-image: linear-gradient(transparent, black 2rem, black calc(100% - 2rem), transparent);
		transition:
			opacity var(--dur) var(--ease-soft),
			transform var(--dur) var(--ease-out),
			filter var(--dur) var(--ease-soft);
	}
	.memory.empty {
		pointer-events: none;
	}
	.memory-stream {
		display: grid;
		gap: 3rem;
		width: min(1240px, 100%);
		margin: 0 auto;
	}
	.writing .memory {
		opacity: 0.12;
		filter: blur(3px);
		transform: scale(0.975);
		pointer-events: none;
	}

	.turn {
		display: grid;
		gap: 1.6rem;
		padding: clamp(1.3rem, 2.5vw, 2.2rem);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 30px;
		background:
			linear-gradient(145deg, rgba(255, 255, 255, 0.075), transparent 44%),
			rgba(29, 42, 52, 0.36);
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.1),
			0 28px 80px rgba(1, 7, 12, 0.25);
		backdrop-filter: blur(15px);
		animation: emerge var(--dur-slow) var(--ease-out) both;
	}
	.turn.archived {
		opacity: 0.68;
	}
	.prompt {
		display: grid;
		justify-items: center;
		gap: 0.35rem;
		text-align: center;
	}
	.prompt span,
	.memory-summary span,
	.scene-error span {
		color: var(--accent);
		font-family: var(--font-display);
		font-size: 0.64rem;
		font-weight: 680;
		letter-spacing: 0.19em;
		text-transform: uppercase;
	}
	.prompt p {
		max-width: 34ch;
		color: var(--text-strong);
		font-family: var(--font-display);
		font-size: clamp(1.5rem, 3vw, 2.8rem);
		font-weight: 590;
		line-height: 1.1;
		letter-spacing: 0.025em;
		text-transform: uppercase;
		text-wrap: balance;
	}
	.memory-summary {
		display: grid;
		justify-items: center;
		gap: 0.4rem;
		text-align: center;
	}
	.memory-summary p {
		max-width: 64ch;
		overflow: hidden;
		color: var(--text);
		font-size: 1rem;
		line-height: 1.45;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.thinking {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.8rem;
		color: var(--text-soft);
		font-size: 0.84rem;
		font-weight: 600;
		letter-spacing: 0.12em;
		text-transform: uppercase;
	}
	.thinking span {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--accent);
		box-shadow: 0 0 20px var(--accent);
		animation: pulse 1.1s ease-in-out infinite;
	}
	.scene-error {
		display: grid;
		justify-items: center;
		gap: 0.5rem;
		padding: 1rem;
		text-align: center;
	}
	.scene-error span {
		color: var(--danger);
	}
	.scene-error p {
		max-width: 70ch;
		color: #ffb1aa;
	}

	.composer-zone {
		position: absolute;
		z-index: 3;
		top: 58%;
		left: 50%;
		transform: translate(-50%, -50%);
		transition: top var(--dur-slow) var(--ease-out);
	}
	.has-turns .composer-zone {
		top: 70%;
	}
	.writing .composer-zone {
		top: 54%;
	}

	.scene-meta {
		position: fixed;
		z-index: 4;
		right: 1rem;
		bottom: 0.85rem;
		left: 1rem;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		color: var(--text-faint);
		font-family: var(--font-display);
		font-size: 0.66rem;
		font-weight: 620;
		letter-spacing: 0.11em;
		text-transform: uppercase;
	}
	.provider {
		display: flex;
		align-items: center;
		gap: 0.45rem;
	}
	.provider-dot {
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: var(--accent);
		box-shadow: 0 0 12px var(--accent);
	}
	.notice {
		color: var(--text-soft);
		animation: emerge var(--dur) var(--ease-out);
	}
	.scene-meta button {
		display: inline-flex;
		align-items: center;
		gap: 0.65rem;
		padding: 0.45rem 0.5rem 0.45rem 0.8rem;
		border: 1px solid rgba(255, 255, 255, 0.11);
		border-radius: 999px;
		background: rgba(255, 255, 255, 0.04);
		box-shadow: inset 0 1px rgba(255, 255, 255, 0.08);
		transition:
			color var(--dur-fast),
			border-color var(--dur-fast),
			background var(--dur-fast);
	}
	.scene-meta button:hover {
		color: var(--text-strong);
		border-color: rgba(132, 233, 220, 0.28);
		background: rgba(132, 233, 220, 0.07);
	}
	.scene-meta kbd {
		padding: 0.27rem 0.45rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 999px;
		font: inherit;
		letter-spacing: 0.05em;
	}

	@keyframes emerge {
		from {
			opacity: 0;
			transform: translateY(18px);
			filter: blur(8px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
			filter: blur(0);
		}
	}
	@keyframes pulse {
		0%,
		100% {
			opacity: 0.35;
			transform: scale(0.8);
		}
		50% {
			opacity: 1;
			transform: scale(1.3);
		}
	}

	@media (max-width: 700px), (max-height: 720px) {
		.hero {
			top: 5.3rem;
		}
		.hero h1 {
			font-size: clamp(3.2rem, 16vw, 5.2rem);
		}
		.memory {
			inset: 4.2rem 0 52%;
		}
		.has-turns .composer-zone {
			top: 72%;
		}
		.scene-meta {
			font-size: 0.58rem;
		}
		.notice,
		.scene-meta button > span {
			display: none;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.turn,
		.notice,
		.thinking span {
			animation: none;
		}
	}
</style>
