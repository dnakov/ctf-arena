<script lang="ts">
	import { onMount } from 'svelte';
	import type { Benchmark, BenchmarkImpl } from '$lib/benchmarks';
	import { api, type ExecutionResult, type BinaryMetadata } from '$lib/api/client';
	import RunDetailModal from '$lib/components/RunDetailModal.svelte';

	interface BenchmarkResult {
		implementation: BenchmarkImpl;
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

	let benchmarks: Benchmark[] = $state([]);
	let loading = $state(true);
	let loadError: string | null = $state(null);

	let selectedBenchmark: Benchmark | null = $state(null);
	let results: BenchmarkResult[] = $state([]);
	let isRunning = $state(false);
	let runAllProgress = $state(0);
	let minInstructions: Record<string, number> = $state({});
	let selectedResult: BenchmarkResult | null = $state(null);

	onMount(async () => {
		try {
			benchmarks = await api.listBenchmarks();
		} catch (err) {
			loadError = err instanceof Error ? err.message : 'Failed to load benchmarks';
		} finally {
			loading = false;
		}
	});

	async function selectBenchmark(benchmark: Benchmark) {
		try {
			// Fetch with source code
			const fullBenchmark = await api.getBenchmark(benchmark.id, true);
			selectedBenchmark = fullBenchmark;
			results = fullBenchmark.implementations.map((impl: BenchmarkImpl) => ({
				implementation: impl,
				status: 'pending' as const
			}));

			// Fetch historical minimums
			try {
				const stats = await api.getBenchmarkStats(benchmark.id);
				minInstructions = stats.min_instructions;
			} catch {
				minInstructions = {};
			}
		} catch (err) {
			loadError = err instanceof Error ? err.message : 'Failed to load benchmark';
		}
	}

	async function runSingle(index: number) {
		const result = results[index];
		if (result.status === 'compiling' || result.status === 'running') return;
		if (!result.implementation.source_code) {
			result.status = 'failed';
			result.error = 'No source code available';
			results = results;
			return;
		}

		result.status = 'compiling';
		result.error = undefined;
		results = results;

		try {
			// Compile
			const compileResponse = await api.compile(
				result.implementation.source_code,
				result.implementation.language as any,
				'release'
			);

			const compileResult = await api.waitForCompile(compileResponse.compile_job_id);
			result.binaryId = compileResult.binary_id;
			result.compileResult = {
				binary_size: compileResult.binary_size,
				compile_time_ms: compileResult.compile_time_ms,
				cached: compileResult.cached
			};

			// Fetch binary metadata (compiler version, flags)
			try {
				result.binaryMetadata = await api.getBinaryMetadata(compileResult.binary_id);
			} catch {
				// Metadata is optional
			}

			result.status = 'running';
			results = results;

			// Execute with benchmark's stdin and env_vars
			const submitResponse = await api.submit(
				compileResult.binary_id,
				1_000_000_000_000,
				selectedBenchmark?.stdin,
				selectedBenchmark?.id,
				selectedBenchmark?.env_vars
			);
			const executionResult = await api.waitForExecution(submitResponse.job_id);

			result.executionResult = executionResult;
			result.status = 'completed';
		} catch (err) {
			result.status = 'failed';
			result.error = err instanceof Error ? err.message : 'Unknown error';
		}

		results = results;
	}

	async function runAll() {
		if (isRunning) return;
		isRunning = true;
		runAllProgress = 0;

		for (let i = 0; i < results.length; i++) {
			await runSingle(i);
			runAllProgress = ((i + 1) / results.length) * 100;
		}

		isRunning = false;
	}

	function formatNumber(n: number): string {
		if (n >= 1_000_000_000) return (n / 1_000_000_000).toFixed(2) + 'B';
		if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + 'M';
		if (n >= 1_000) return (n / 1_000).toFixed(2) + 'K';
		return n.toString();
	}

	function formatBytes(bytes: number): string {
		if (bytes >= 1024 * 1024) return (bytes / 1024 / 1024).toFixed(1) + ' MB';
		if (bytes >= 1024) return (bytes / 1024).toFixed(1) + ' KB';
		return bytes + ' B';
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

	function sortedResults(): BenchmarkResult[] {
		return [...results].sort((a, b) => {
			if (a.status === 'completed' && b.status !== 'completed') return -1;
			if (a.status !== 'completed' && b.status === 'completed') return 1;
			if (a.executionResult && b.executionResult) {
				return a.executionResult.instructions - b.executionResult.instructions;
			}
			// Sort by min instructions (from historical data), then by reference instructions
			const minA =
				minInstructions[a.implementation.language] ??
				a.implementation.reference_instructions ??
				Infinity;
			const minB =
				minInstructions[b.implementation.language] ??
				b.implementation.reference_instructions ??
				Infinity;
			return minA - minB;
		});
	}
</script>

<div class="container mx-auto px-4 py-8 max-w-7xl">
	<header class="mb-8">
		<p class="text-dark-400">Compare instruction counts across languages</p>
	</header>

	{#if loading}
		<div class="text-center py-12 text-dark-400">Loading benchmarks...</div>
	{:else if loadError}
		<div class="text-center py-12 text-red-400">{loadError}</div>
	{:else}
		<div class="grid grid-cols-1 lg:grid-cols-4 gap-6">
			<!-- Benchmark List -->
			<div class="space-y-4">
				<h2 class="text-lg font-semibold text-dark-100">Available Benchmarks</h2>
				{#each benchmarks as benchmark}
					<button
						class="w-full text-left p-4 rounded-lg border transition-colors {selectedBenchmark?.id ===
						benchmark.id
							? 'bg-dark-800 border-blue-500'
							: 'bg-dark-900 border-dark-700 hover:border-dark-500'}"
						onclick={() => selectBenchmark(benchmark)}
					>
						<h3 class="font-medium text-dark-100">{benchmark.name}</h3>
						<p class="text-sm text-dark-400 mt-1">
							{benchmark.implementations.length} implementations
						</p>
					</button>
				{/each}
			</div>

			<!-- Benchmark Details & Results -->
			<div class="lg:col-span-3 space-y-4">
				{#if selectedBenchmark}
					<div class="bg-dark-900 rounded-lg border border-dark-700 p-4">
						<div class="flex items-start justify-between">
							<div>
								<h2 class="text-xl font-semibold text-dark-100">{selectedBenchmark.name}</h2>
								<p class="text-dark-400 mt-2">{selectedBenchmark.description}</p>
							</div>
							<button
								class="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-colors"
								disabled={isRunning}
								onclick={runAll}
							>
								{isRunning ? `Running... ${runAllProgress.toFixed(0)}%` : 'Run All'}
							</button>
						</div>

						{#if isRunning}
							<div class="mt-4 h-2 bg-dark-800 rounded-full overflow-hidden">
								<div
									class="h-full bg-blue-500 transition-all duration-300"
									style="width: {runAllProgress}%"
								></div>
							</div>
						{/if}
					</div>

					<!-- Results Table -->
					<div class="bg-dark-900 rounded-lg border border-dark-700 overflow-hidden">
						<table class="w-full">
							<thead class="bg-dark-800">
								<tr>
									<th class="px-4 py-3 text-left text-sm font-medium text-dark-300">Language</th>
									<th class="px-4 py-3 text-left text-sm font-medium text-dark-300">Tier</th>
									<th class="px-4 py-3 text-right text-sm font-medium text-dark-300">Min</th>
									<th class="px-4 py-3 text-right text-sm font-medium text-dark-300">Actual</th>
									<th class="px-4 py-3 text-right text-sm font-medium text-dark-300">Memory</th>
									<th class="px-4 py-3 text-right text-sm font-medium text-dark-300">Binary</th>
									<th class="px-4 py-3 text-center text-sm font-medium text-dark-300">Status</th>
									<th class="px-4 py-3 text-center text-sm font-medium text-dark-300">Action</th>
								</tr>
							</thead>
							<tbody class="divide-y divide-dark-700">
								{#each sortedResults() as result, i}
									<tr
										class="hover:bg-dark-800/50 cursor-pointer"
										onclick={() => (selectedResult = result)}
									>
										<td class="px-4 py-3 text-dark-100 font-medium">
											{result.implementation.name}
										</td>
										<td class="px-4 py-3">
											<span class="text-sm capitalize {getTierColor(result.implementation.tier)}">
												{result.implementation.tier}
											</span>
										</td>
										<td class="px-4 py-3 text-right font-mono text-dark-400">
											{#if minInstructions[result.implementation.language]}
												{formatNumber(minInstructions[result.implementation.language])}
											{:else if result.implementation.reference_instructions}
												<span class="text-dark-500"
													>{formatNumber(result.implementation.reference_instructions)}</span
												>
											{:else}
												—
											{/if}
										</td>
										<td class="px-4 py-3 text-right font-mono">
											{#if result.executionResult}
												<span
													class={result.executionResult.limit_reached
														? 'text-red-400'
														: 'text-green-400'}
												>
													{formatNumber(result.executionResult.instructions)}
												</span>
											{:else}
												<span class="text-dark-500">—</span>
											{/if}
										</td>
										<td class="px-4 py-3 text-right font-mono text-dark-300">
											{#if result.executionResult}
												{formatBytes(result.executionResult.memory_peak_kb * 1024)}
											{:else}
												<span class="text-dark-500">—</span>
											{/if}
										</td>
										<td class="px-4 py-3 text-right font-mono text-dark-300">
											{#if result.compileResult}
												{formatBytes(result.compileResult.binary_size)}
											{:else}
												<span class="text-dark-500">—</span>
											{/if}
										</td>
										<td class="px-4 py-3 text-center">
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
										<td class="px-4 py-3 text-center">
											<button
												class="px-3 py-1 text-sm bg-dark-700 hover:bg-dark-600 disabled:opacity-50 text-dark-200 rounded transition-colors"
												disabled={result.status === 'compiling' || result.status === 'running'}
												onclick={(e) => {
													e.stopPropagation();
													runSingle(results.indexOf(result));
												}}
											>
												Run
											</button>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>

					<!-- Legend -->
					<div class="bg-dark-900 rounded-lg border border-dark-700 p-4">
						<h3 class="text-sm font-medium text-dark-300 mb-2">Tier Legend</h3>
						<div class="flex flex-wrap gap-4 text-sm">
							<span><span class="text-green-400">●</span> Native - Direct compilation</span>
							<span><span class="text-blue-400">●</span> Managed - JVM/CLR AOT</span>
							<span><span class="text-yellow-400">●</span> Scripting - Bundled interpreter</span>
							<span><span class="text-purple-400">●</span> Special - Unique runtimes</span>
						</div>
					</div>
				{:else}
					<div class="bg-dark-900 rounded-lg border border-dark-700 p-12 text-center">
						<p class="text-dark-400">
							Select a benchmark from the list to see implementations and run comparisons
						</p>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>

{#if selectedResult}
	<RunDetailModal result={selectedResult} onClose={() => (selectedResult = null)} />
{/if}
