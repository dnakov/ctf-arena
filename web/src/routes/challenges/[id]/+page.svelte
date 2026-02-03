<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import {
		api,
		type ChallengeDetail,
		type ChallengeBaseline,
		type LeaderboardEntry,
		type SubmissionStatusResponse,
		type ExecutionResult,
		type BinaryMetadata,
		type Language,
		type Optimization
	} from '$lib/api/client';
	import { auth, isLoggedIn, user } from '$lib/stores/auth';
	import { currentLanguage, sourceCode, currentOptimization } from '$lib/stores/editor';
	import Editor from '$lib/components/Editor.svelte';
	import LanguageSelector from '$lib/components/LanguageSelector.svelte';

	let challenge: ChallengeDetail | null = $state(null);
	let leaderboard: LeaderboardEntry[] = $state([]);
	let loading = $state(true);
	let error: string | null = $state(null);

	// Submission state
	let submitting = $state(false);
	let submissionStatus: SubmissionStatusResponse | null = $state(null);
	let submissionError: string | null = $state(null);

	// Baselines state
	interface BaselineResult {
		baseline: ChallengeBaseline;
		status: 'pending' | 'compiling' | 'running' | 'completed' | 'failed';
		error?: string;
		binaryId?: string;
		binaryMetadata?: BinaryMetadata;
		compileResult?: {
			binary_size: number;
			compile_time_ms: number;
			cached: boolean;
		};
		executionResult?: ExecutionResult;
	}

	let baselineResults: BaselineResult[] = $state([]);
	let isRunningBaselines = $state(false);
	let baselineProgress = $state(0);
	let showBaselines = $state(false);

	let challengeId = $derived($page.params.id);

	onMount(async () => {
		const id = $page.params.id;
		if (!id) return;

		try {
			[challenge, leaderboard] = await Promise.all([
				api.getChallenge(id),
				api.getChallengeLeaderboard(id, { limit: 10 })
			]);

			// Initialize baseline results
			if (challenge?.baselines) {
				baselineResults = challenge.baselines.map((baseline) => ({
					baseline,
					status: 'pending' as const
				}));
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load challenge';
		} finally {
			loading = false;
		}
	});

	async function runBaselineSingle(index: number) {
		const result = baselineResults[index];
		if (result.status === 'compiling' || result.status === 'running') return;

		result.status = 'compiling';
		result.error = undefined;
		baselineResults = baselineResults;

		try {
			// Compile
			const compileResponse = await api.compile(
				result.baseline.source_code,
				result.baseline.language as Language,
				'release'
			);

			const compileResult = await api.waitForCompile(compileResponse.compile_job_id);
			result.binaryId = compileResult.binary_id;
			result.compileResult = {
				binary_size: compileResult.binary_size,
				compile_time_ms: compileResult.compile_time_ms,
				cached: compileResult.cached
			};

			// Fetch binary metadata
			try {
				result.binaryMetadata = await api.getBinaryMetadata(compileResult.binary_id);
			} catch {
				// Metadata is optional
			}

			result.status = 'running';
			baselineResults = baselineResults;

			// Execute
			const submitResponse = await api.submit(compileResult.binary_id, 10_000_000_000);
			const executionResult = await api.waitForExecution(submitResponse.job_id);

			result.executionResult = executionResult;
			result.status = 'completed';
		} catch (err) {
			result.status = 'failed';
			result.error = err instanceof Error ? err.message : 'Unknown error';
		}

		baselineResults = baselineResults;
	}

	async function runAllBaselines() {
		if (isRunningBaselines) return;
		isRunningBaselines = true;
		baselineProgress = 0;

		for (let i = 0; i < baselineResults.length; i++) {
			await runBaselineSingle(i);
			baselineProgress = ((i + 1) / baselineResults.length) * 100;
		}

		isRunningBaselines = false;
	}

	function loadBaselineIntoEditor(baseline: ChallengeBaseline) {
		sourceCode.set(baseline.source_code);
		currentLanguage.set(baseline.language as Language);
	}

	function getTierColor(tier: string): string {
		switch (tier) {
			case 'native':
				return 'text-green-400';
			case 'managed':
				return 'text-blue-400';
			case 'scripting':
				return 'text-yellow-400';
			case 'special':
				return 'text-purple-400';
			default:
				return 'text-dark-400';
		}
	}

	function sortedBaselineResults(): BaselineResult[] {
		return [...baselineResults].sort((a, b) => {
			if (a.status === 'completed' && b.status !== 'completed') return -1;
			if (a.status !== 'completed' && b.status === 'completed') return 1;
			if (a.executionResult && b.executionResult) {
				return a.executionResult.instructions - b.executionResult.instructions;
			}
			return 0;
		});
	}

	function formatBytes(bytes: number): string {
		if (bytes >= 1024 * 1024) return (bytes / 1024 / 1024).toFixed(1) + ' MB';
		if (bytes >= 1024) return (bytes / 1024).toFixed(1) + ' KB';
		return bytes + ' B';
	}

	async function handleSubmit() {
		if (!$isLoggedIn) {
			auth.login();
			return;
		}

		const id = challengeId;
		if (!id) return;

		submitting = true;
		submissionError = null;
		submissionStatus = null;

		try {
			const response = await api.submitChallenge(
				id,
				$sourceCode,
				$currentLanguage as Language,
				$currentOptimization as Optimization
			);

			// Poll for completion
			submissionStatus = await api.waitForChallengeSubmission(id, response.submission_id);

			// Refresh leaderboard on success
			if (submissionStatus.status === 'passed') {
				leaderboard = await api.getChallengeLeaderboard(id, { limit: 10 });
			}
		} catch (e) {
			submissionError = e instanceof Error ? e.message : 'Submission failed';
		} finally {
			submitting = false;
		}
	}

	function getDifficultyColor(difficulty: string): string {
		switch (difficulty?.toLowerCase()) {
			case 'easy':
				return 'text-green-400 bg-green-400/10';
			case 'medium':
				return 'text-yellow-400 bg-yellow-400/10';
			case 'hard':
				return 'text-red-400 bg-red-400/10';
			default:
				return 'text-dark-400 bg-dark-400/10';
		}
	}

	function formatInstructions(n: number): string {
		if (n >= 1_000_000_000) return (n / 1_000_000_000).toFixed(2) + 'B';
		if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + 'M';
		if (n >= 1_000) return (n / 1_000).toFixed(2) + 'K';
		return n.toString();
	}
</script>

<svelte:head>
	<title>{challenge?.name || 'Challenge'} | CTF Arena</title>
</svelte:head>

<div class="container mx-auto px-4 py-8">
	{#if loading}
		<div class="animate-pulse">
			<div class="h-8 bg-dark-800 rounded w-1/3 mb-4"></div>
			<div class="h-4 bg-dark-800 rounded w-2/3 mb-8"></div>
			<div class="h-96 bg-dark-800 rounded"></div>
		</div>
	{:else if error}
		<div class="bg-red-500/10 border border-red-500/20 rounded-lg p-4 text-red-400">
			{error}
		</div>
	{:else if challenge}
		<!-- Header -->
		<div class="mb-8">
			<div class="flex items-center gap-4 mb-2">
				<a href="/challenges" class="text-dark-400 hover:text-dark-200">&larr; Back</a>
				<span
					class="text-xs font-medium px-2 py-1 rounded {getDifficultyColor(challenge.difficulty)}"
				>
					{challenge.difficulty}
				</span>
				<span class="text-xs text-dark-500 uppercase">{challenge.category}</span>
			</div>
			<h1 class="text-3xl font-bold text-dark-50 mb-2">{challenge.name}</h1>
			<p class="text-dark-400">{challenge.description}</p>
		</div>

		<div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
			<!-- Left: Editor and Submission -->
			<div class="lg:col-span-2 space-y-6">
				<!-- Specs -->
				<div class="bg-dark-900 border border-dark-800 rounded-lg p-4">
					<h3 class="text-sm font-medium text-dark-300 mb-2">Output Specification</h3>
					<p class="text-sm text-dark-400">{challenge.output_spec}</p>
					{#if challenge.input_spec}
						<h3 class="text-sm font-medium text-dark-300 mt-4 mb-2">Input Specification</h3>
						<p class="text-sm text-dark-400">{challenge.input_spec}</p>
					{/if}
					<h3 class="text-sm font-medium text-dark-300 mt-4 mb-2">
						Test Cases ({challenge.test_cases.length})
					</h3>
					<div class="space-y-2">
						{#each challenge.test_cases as testCase, i}
							<div class="bg-dark-800 rounded p-2 text-xs">
								<span class="text-dark-500">#{i + 1}</span>
								{#if testCase.description}
									<span class="text-dark-400 ml-2">{testCase.description}</span>
								{/if}
								{#if testCase.stdin}
									<div class="mt-1 text-dark-400">
										<span class="text-dark-500">stdin:</span>
										<code class="ml-1">{testCase.stdin.slice(0, 50)}{testCase.stdin.length > 50 ? '...' : ''}</code>
									</div>
								{/if}
							</div>
						{/each}
					</div>
					<p class="text-xs text-dark-500 mt-2">
						Verification mode: <span class="text-dark-400">{challenge.verify_mode}</span>
					</p>
				</div>

				<!-- Editor -->
				<div class="bg-dark-900 border border-dark-800 rounded-lg overflow-hidden">
					<div class="border-b border-dark-800 p-3 flex items-center justify-between">
						<LanguageSelector />
						<select
							bind:value={$currentOptimization}
							class="bg-dark-800 text-dark-200 text-sm rounded px-2 py-1 border border-dark-700"
						>
							<option value="release">Release</option>
							<option value="size">Size</option>
							<option value="debug">Debug</option>
						</select>
					</div>
					<div class="h-96">
						<Editor />
					</div>
				</div>

				<!-- Submit Button -->
				<div class="flex items-center gap-4">
					<button
						onclick={handleSubmit}
						disabled={submitting}
						class="px-6 py-2 bg-green-600 hover:bg-green-700 disabled:bg-dark-700 disabled:cursor-not-allowed text-white font-medium rounded-lg transition-colors flex items-center gap-2"
					>
						{#if submitting}
							<svg class="animate-spin w-4 h-4" fill="none" viewBox="0 0 24 24">
								<circle
									class="opacity-25"
									cx="12"
									cy="12"
									r="10"
									stroke="currentColor"
									stroke-width="4"
								></circle>
								<path
									class="opacity-75"
									fill="currentColor"
									d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
								></path>
							</svg>
							<span>Running tests...</span>
						{:else if !$isLoggedIn}
							<svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
								<path
									d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"
								/>
							</svg>
							<span>Sign in to Submit</span>
						{:else}
							<span>Submit Solution</span>
						{/if}
					</button>
				</div>

				<!-- Submission Result -->
				{#if submissionError}
					<div class="bg-red-500/10 border border-red-500/20 rounded-lg p-4 text-red-400">
						{submissionError}
					</div>
				{/if}

				{#if submissionStatus}
					<div
						class="rounded-lg p-4 border {submissionStatus.status === 'passed'
							? 'bg-green-500/10 border-green-500/20'
							: 'bg-red-500/10 border-red-500/20'}"
					>
						<div class="flex items-center justify-between mb-4">
							<h3
								class="font-medium {submissionStatus.status === 'passed'
									? 'text-green-400'
									: 'text-red-400'}"
							>
								{submissionStatus.status === 'passed' ? '✓ All tests passed!' : '✗ Tests failed'}
							</h3>
							{#if submissionStatus.instructions}
								<span class="text-dark-300 font-mono">
									{formatInstructions(submissionStatus.instructions)} instructions
								</span>
							{/if}
						</div>

						{#if submissionStatus.test_results}
							<div class="space-y-2">
								{#each submissionStatus.test_results as result}
									<div
										class="flex items-center gap-2 text-sm {result.passed
											? 'text-green-400'
											: 'text-red-400'}"
									>
										<span>{result.passed ? '✓' : '✗'}</span>
										<span>Test #{result.test_index + 1}</span>
										{#if result.error}
											<span class="text-dark-400">- {result.error}</span>
										{/if}
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/if}

				<!-- Baselines Section -->
				{#if challenge?.baselines && challenge.baselines.length > 0}
					<div class="bg-dark-900 border border-dark-800 rounded-lg overflow-hidden">
						<button
							onclick={() => (showBaselines = !showBaselines)}
							class="w-full p-4 flex items-center justify-between hover:bg-dark-800/50 transition-colors"
						>
							<div class="flex items-center gap-2">
								<svg
									class="w-4 h-4 transition-transform {showBaselines ? 'rotate-90' : ''}"
									fill="none"
									viewBox="0 0 24 24"
									stroke="currentColor"
								>
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
								</svg>
								<h3 class="font-medium text-dark-200">
									Reference Solutions ({challenge.baselines.length} languages)
								</h3>
							</div>
							<span class="text-xs text-dark-500">Click to {showBaselines ? 'hide' : 'show'}</span>
						</button>

						{#if showBaselines}
							<div class="border-t border-dark-800">
								<div class="p-4 flex items-center justify-between border-b border-dark-800">
									<p class="text-sm text-dark-400">
										Compare instruction counts across different languages
									</p>
									<button
										class="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed text-white text-sm rounded-lg font-medium transition-colors"
										disabled={isRunningBaselines}
										onclick={runAllBaselines}
									>
										{isRunningBaselines
											? `Running... ${baselineProgress.toFixed(0)}%`
											: 'Run All'}
									</button>
								</div>

								{#if isRunningBaselines}
									<div class="px-4 py-2">
										<div class="h-1 bg-dark-800 rounded-full overflow-hidden">
											<div
												class="h-full bg-blue-500 transition-all duration-300"
												style="width: {baselineProgress}%"
											></div>
										</div>
									</div>
								{/if}

								<table class="w-full">
									<thead class="bg-dark-800">
										<tr>
											<th class="px-4 py-2 text-left text-xs font-medium text-dark-400">Language</th>
											<th class="px-4 py-2 text-left text-xs font-medium text-dark-400">Tier</th>
											<th class="px-4 py-2 text-right text-xs font-medium text-dark-400">Instructions</th>
											<th class="px-4 py-2 text-right text-xs font-medium text-dark-400">Memory</th>
											<th class="px-4 py-2 text-right text-xs font-medium text-dark-400">Binary</th>
											<th class="px-4 py-2 text-center text-xs font-medium text-dark-400">Status</th>
											<th class="px-4 py-2 text-center text-xs font-medium text-dark-400">Actions</th>
										</tr>
									</thead>
									<tbody class="divide-y divide-dark-800">
										{#each sortedBaselineResults() as result, i}
											<tr class="hover:bg-dark-800/30">
												<td class="px-4 py-2 text-dark-200 font-medium text-sm">
													{result.baseline.name}
												</td>
												<td class="px-4 py-2">
													<span class="text-xs capitalize {getTierColor(result.baseline.tier)}">
														{result.baseline.tier}
													</span>
												</td>
												<td class="px-4 py-2 text-right font-mono text-sm">
													{#if result.executionResult}
														<span
															class={result.executionResult.limit_reached
																? 'text-red-400'
																: 'text-green-400'}
														>
															{formatInstructions(result.executionResult.instructions)}
														</span>
													{:else}
														<span class="text-dark-500">—</span>
													{/if}
												</td>
												<td class="px-4 py-2 text-right font-mono text-sm text-dark-400">
													{#if result.executionResult}
														{formatBytes(result.executionResult.memory_peak_kb * 1024)}
													{:else}
														<span class="text-dark-500">—</span>
													{/if}
												</td>
												<td class="px-4 py-2 text-right font-mono text-sm text-dark-400">
													{#if result.compileResult}
														{formatBytes(result.compileResult.binary_size)}
													{:else}
														<span class="text-dark-500">—</span>
													{/if}
												</td>
												<td class="px-4 py-2 text-center text-xs">
													{#if result.status === 'pending'}
														<span class="text-dark-500">Pending</span>
													{:else if result.status === 'compiling'}
														<span class="text-yellow-400">Compiling...</span>
													{:else if result.status === 'running'}
														<span class="text-blue-400">Running...</span>
													{:else if result.status === 'completed'}
														<span class="text-green-400">Done</span>
													{:else if result.status === 'failed'}
														<span class="text-red-400" title={result.error}>Failed</span>
													{/if}
												</td>
												<td class="px-4 py-2 text-center">
													<div class="flex items-center justify-center gap-2">
														<button
															class="px-2 py-1 text-xs bg-dark-700 hover:bg-dark-600 disabled:opacity-50 text-dark-200 rounded transition-colors"
															disabled={result.status === 'compiling' || result.status === 'running'}
															onclick={() => runBaselineSingle(baselineResults.indexOf(result))}
														>
															Run
														</button>
														<button
															class="px-2 py-1 text-xs bg-dark-700 hover:bg-dark-600 text-dark-200 rounded transition-colors"
															onclick={() => loadBaselineIntoEditor(result.baseline)}
															title="Load into editor"
														>
															Edit
														</button>
													</div>
												</td>
											</tr>
										{/each}
									</tbody>
								</table>

								<div class="p-3 bg-dark-800/50 text-xs text-dark-500">
									<span class="text-green-400">●</span> Native &nbsp;
									<span class="text-blue-400">●</span> Managed &nbsp;
									<span class="text-yellow-400">●</span> Scripting &nbsp;
									<span class="text-purple-400">●</span> Special
								</div>
							</div>
						{/if}
					</div>
				{/if}
			</div>

			<!-- Right: Leaderboard -->
			<div class="space-y-6">
				<div class="bg-dark-900 border border-dark-800 rounded-lg p-4">
					<div class="flex items-center justify-between mb-4">
						<h3 class="font-medium text-dark-200">Top 10</h3>
						<a
							href="/challenges/{challengeId}/leaderboard"
							class="text-xs text-blue-400 hover:text-blue-300"
						>
							View all &rarr;
						</a>
					</div>

					{#if leaderboard.length === 0}
						<p class="text-sm text-dark-500 text-center py-4">No submissions yet. Be the first!</p>
					{:else}
						<div class="space-y-2">
							{#each leaderboard as entry, i}
								<div
									class="flex items-center gap-3 py-2 {i === 0
										? 'text-yellow-400'
										: i === 1
											? 'text-gray-400'
											: i === 2
												? 'text-amber-600'
												: 'text-dark-400'}"
								>
									<span class="w-6 text-center font-mono text-sm">#{entry.rank}</span>
									<div class="flex items-center gap-2 flex-1 min-w-0">
										{#if entry.user.avatar_url}
											<img
												src={entry.user.avatar_url}
												alt=""
												class="w-6 h-6 rounded-full"
											/>
										{:else}
											<div class="w-6 h-6 rounded-full bg-dark-700"></div>
										{/if}
										<a
											href="/profile/{entry.user.username}"
											class="truncate hover:text-blue-400 transition-colors"
										>
											{entry.user.display_name || entry.user.username}
										</a>
										{#if entry.user.is_verified}
											<span
												class="text-xs {entry.user.user_type === 'clanker'
													? 'text-purple-400'
													: 'text-green-400'}"
											>
												{entry.user.user_type === 'clanker' ? '🤖' : '✓'}
											</span>
										{/if}
									</div>
									<span class="font-mono text-sm text-dark-300">
										{formatInstructions(entry.instructions)}
									</span>
									<span class="text-xs text-dark-500">{entry.language}</span>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			</div>
		</div>
	{/if}
</div>
