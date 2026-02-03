<script lang="ts">
	import { onMount } from 'svelte';
	import Editor from '$lib/components/Editor.svelte';
	import LanguageSelector from '$lib/components/LanguageSelector.svelte';
	import CompileRunButton from '$lib/components/CompileRunButton.svelte';
	import StatusDisplay from '$lib/components/StatusDisplay.svelte';
	import ResultsPanel from '$lib/components/ResultsPanel.svelte';
	import FlagsPanel from '$lib/components/FlagsPanel.svelte';
	import { currentOptimization, instructionLimit, stdin, currentLanguage, languageFlags } from '$lib/stores/editor';
	import { api, type HealthResponse } from '$lib/api/client';

	// Check if current language has flags
	$: hasFlags = (languageFlags[$currentLanguage] || []).length > 0;

	let health: HealthResponse | null = null;
	let healthError: string | null = null;

	onMount(async () => {
		try {
			health = await api.health();
		} catch (err) {
			healthError = err instanceof Error ? err.message : 'Failed to connect';
		}
	});
</script>

<div class="container mx-auto px-4 py-8 max-w-7xl">
	<!-- Header -->
	<header class="mb-8">
		<div class="flex items-center justify-between">
			<div>
				<p class="text-dark-400">Code Golf Challenge - Minimize instruction count</p>
			</div>

			<!-- Health Status -->
			<div class="flex items-center gap-2">
				{#if health}
					<span
						class="flex items-center gap-1.5 text-sm {health.status === 'ok'
							? 'text-green-400'
							: 'text-yellow-400'}"
					>
						<span class="h-2 w-2 rounded-full {health.status === 'ok' ? 'bg-green-400' : 'bg-yellow-400'}"></span>
						{health.status === 'ok' ? 'Connected' : 'Degraded'}
					</span>
				{:else if healthError}
					<span class="flex items-center gap-1.5 text-sm text-red-400">
						<span class="h-2 w-2 rounded-full bg-red-400"></span>
						Disconnected
					</span>
				{:else}
					<span class="flex items-center gap-1.5 text-sm text-dark-400">
						<span class="h-2 w-2 rounded-full bg-dark-400 animate-pulse"></span>
						Connecting...
					</span>
				{/if}
			</div>
		</div>
	</header>

	<div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
		<!-- Left Column: Editor + Controls -->
		<div class="lg:col-span-2 space-y-4">
			<!-- Language Selector -->
			<div class="bg-dark-900 rounded-lg border border-dark-700 p-4">
				<h2 class="font-semibold text-dark-100 mb-3">Language</h2>
				<LanguageSelector />
			</div>

			<!-- Compiler Options (always visible when available) -->
			{#if hasFlags}
				<div class="bg-dark-900 rounded-lg border border-dark-700 p-4">
					<h2 class="font-semibold text-dark-100 mb-3">Compiler Options</h2>
					<FlagsPanel />
				</div>
			{/if}

			<!-- Editor -->
			<div>
				<Editor />
			</div>

			<!-- Options Bar -->
			<div class="bg-dark-900 rounded-lg border border-dark-700 p-4">
				<div class="flex flex-wrap items-center gap-4">
					<!-- Optimization -->
					<div class="flex items-center gap-2">
						<label for="optimization" class="text-dark-400 text-sm">Optimization:</label>
						<select
							id="optimization"
							class="bg-dark-800 border border-dark-600 rounded px-3 py-1.5 text-dark-100 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
							bind:value={$currentOptimization}
						>
							<option value="debug">Debug</option>
							<option value="release">Release</option>
							<option value="size">Size</option>
						</select>
					</div>

					<!-- Instruction Limit -->
					<div class="flex items-center gap-2">
						<label for="limit" class="text-dark-400 text-sm">Limit:</label>
						<input
							id="limit"
							type="number"
							class="bg-dark-800 border border-dark-600 rounded px-3 py-1.5 text-dark-100 text-sm w-32 focus:outline-none focus:ring-2 focus:ring-blue-500"
							bind:value={$instructionLimit}
							min="1000"
							max="1000000000"
						/>
					</div>

					<!-- Status -->
					<div class="ml-auto">
						<StatusDisplay />
					</div>
				</div>
			</div>
		</div>

		<!-- Right Column: Run Button + Results -->
		<div class="space-y-4">
			<!-- Run Button -->
			<CompileRunButton />

			<!-- stdin -->
			<div class="bg-dark-900 rounded-lg border border-dark-700 p-4">
				<h2 class="font-semibold text-dark-100 mb-2">stdin (optional)</h2>
				<textarea
					class="w-full h-24 bg-dark-800 border border-dark-600 rounded-lg p-3 text-dark-100 text-sm font-mono resize-none focus:outline-none focus:ring-2 focus:ring-blue-500"
					placeholder="Enter input for your program..."
					bind:value={$stdin}
				></textarea>
			</div>

			<!-- Results -->
			<ResultsPanel />
		</div>
	</div>

	<!-- Footer -->
	<footer class="mt-12 text-center text-dark-500 text-sm">
		<p>CTF Arena - Compete to write the most efficient code</p>
		<p class="mt-1">
			Reference: Assembly 51 | Zig 594 | C 2,008 | Rust 24,530 | Go 1.1M instructions
		</p>
	</footer>
</div>
