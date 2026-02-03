<script lang="ts">
	import { onMount } from 'svelte';
	import { api, type ChallengeInfo } from '$lib/api/client';

	let challenges: ChallengeInfo[] = [];
	let loading = true;
	let error: string | null = null;

	onMount(async () => {
		try {
			const response = await api.listChallenges();
			challenges = response.challenges;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load challenges';
		} finally {
			loading = false;
		}
	});

	function getDifficultyColor(difficulty: string): string {
		switch (difficulty.toLowerCase()) {
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

	function getCategoryIcon(category: string): string {
		switch (category.toLowerCase()) {
			case 'networking':
				return '🌐';
			case 'system':
				return '💻';
			case 'crypto':
				return '🔐';
			case 'binary':
				return '📦';
			default:
				return '🎯';
		}
	}
</script>

<svelte:head>
	<title>Challenges | CTF Arena</title>
</svelte:head>

<div class="container mx-auto px-4 py-8">
	<div class="mb-8">
		<h1 class="text-3xl font-bold text-dark-50 mb-2">Challenges</h1>
		<p class="text-dark-400">
			Compete for the lowest instruction count. Write efficient code to solve each challenge.
		</p>
	</div>

	{#if loading}
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
			{#each [1, 2, 3, 4, 5, 6] as _}
				<div class="bg-dark-900 border border-dark-800 rounded-lg p-6 animate-pulse">
					<div class="h-6 bg-dark-800 rounded w-3/4 mb-4"></div>
					<div class="h-4 bg-dark-800 rounded w-full mb-2"></div>
					<div class="h-4 bg-dark-800 rounded w-2/3"></div>
				</div>
			{/each}
		</div>
	{:else if error}
		<div class="bg-red-500/10 border border-red-500/20 rounded-lg p-4 text-red-400">
			{error}
		</div>
	{:else if challenges.length === 0}
		<div class="text-center py-12 text-dark-400">
			<p class="text-lg">No challenges available yet.</p>
			<p class="text-sm mt-2">Check back soon!</p>
		</div>
	{:else}
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
			{#each challenges as challenge}
				<a
					href="/challenges/{challenge.id}"
					class="block bg-dark-900 border border-dark-800 rounded-lg p-6 hover:border-dark-700 transition-colors group"
				>
					<div class="flex items-start justify-between mb-3">
						<div class="flex items-center gap-2">
							<span class="text-2xl">{getCategoryIcon(challenge.category)}</span>
							<h2 class="text-lg font-semibold text-dark-100 group-hover:text-blue-400 transition-colors">
								{challenge.name}
							</h2>
						</div>
						<span
							class="text-xs font-medium px-2 py-1 rounded {getDifficultyColor(
								challenge.difficulty
							)}"
						>
							{challenge.difficulty}
						</span>
					</div>

					<p class="text-sm text-dark-400 line-clamp-2 mb-4">
						{challenge.description}
					</p>

					<div class="flex items-center justify-between">
						<span class="text-xs text-dark-500 uppercase tracking-wide">
							{challenge.category}
						</span>
						<span class="text-xs text-dark-500">
							{challenge.is_active ? '🟢 Active' : '🔴 Inactive'}
						</span>
					</div>
				</a>
			{/each}
		</div>
	{/if}
</div>
