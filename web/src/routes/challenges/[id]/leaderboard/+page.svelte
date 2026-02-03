<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api, type ChallengeDetail, type LeaderboardEntry, type Language } from '$lib/api/client';
	import { languages } from '$lib/stores/editor';

	let challenge: ChallengeDetail | null = null;
	let leaderboard: LeaderboardEntry[] = [];
	let loading = true;
	let error: string | null = null;

	let languageFilter: string = '';
	let userTypeFilter: 'all' | 'human' | 'clanker' = 'all';

	$: challengeId = $page.params.id;

	$: loadLeaderboard(languageFilter, userTypeFilter);

	async function loadLeaderboard(language: string, userType: string) {
		const id = challengeId;
		if (!id || loading) return;

		try {
			const options: { language?: string; user_type?: string; limit?: number } = { limit: 100 };
			if (language) options.language = language;
			if (userType !== 'all') options.user_type = userType;
			leaderboard = await api.getChallengeLeaderboard(id, options);
		} catch (e) {
			console.error('Failed to load leaderboard:', e);
		}
	}

	onMount(async () => {
		const id = $page.params.id;
		if (!id) return;

		try {
			[challenge, leaderboard] = await Promise.all([
				api.getChallenge(id),
				api.getChallengeLeaderboard(id, { limit: 100 })
			]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load challenge';
		} finally {
			loading = false;
		}
	});

	function formatInstructions(n: number): string {
		if (n >= 1_000_000_000) return (n / 1_000_000_000).toFixed(2) + 'B';
		if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + 'M';
		if (n >= 1_000) return (n / 1_000).toFixed(2) + 'K';
		return n.toString();
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	// Get unique languages from leaderboard
	$: availableLanguages = [...new Set(leaderboard.map((e) => e.language))].sort();
</script>

<svelte:head>
	<title>Leaderboard - {challenge?.name || 'Challenge'} | CTF Arena</title>
</svelte:head>

<div class="container mx-auto px-4 py-8">
	{#if loading}
		<div class="animate-pulse">
			<div class="h-8 bg-dark-800 rounded w-1/3 mb-4"></div>
			<div class="h-96 bg-dark-800 rounded"></div>
		</div>
	{:else if error}
		<div class="bg-red-500/10 border border-red-500/20 rounded-lg p-4 text-red-400">
			{error}
		</div>
	{:else if challenge}
		<!-- Header -->
		<div class="mb-6">
			<div class="flex items-center gap-4 mb-2">
				<a href="/challenges/{challengeId}" class="text-dark-400 hover:text-dark-200">
					&larr; Back to Challenge
				</a>
			</div>
			<h1 class="text-2xl font-bold text-dark-50">{challenge.name} Leaderboard</h1>
		</div>

		<!-- Filters -->
		<div class="flex items-center gap-4 mb-6">
			<div class="flex items-center gap-2">
				<label class="text-sm text-dark-400">Language:</label>
				<select
					bind:value={languageFilter}
					class="bg-dark-800 text-dark-200 text-sm rounded px-3 py-1.5 border border-dark-700"
				>
					<option value="">All Languages</option>
					{#each availableLanguages as lang}
						<option value={lang}>{lang}</option>
					{/each}
				</select>
			</div>

			<div class="flex items-center gap-2 bg-dark-900 border border-dark-800 rounded-lg p-1">
				<button
					onclick={() => (userTypeFilter = 'all')}
					class="px-3 py-1 text-sm rounded {userTypeFilter === 'all'
						? 'bg-dark-700 text-dark-100'
						: 'text-dark-400 hover:text-dark-200'}"
				>
					All
				</button>
				<button
					onclick={() => (userTypeFilter = 'human')}
					class="px-3 py-1 text-sm rounded flex items-center gap-1 {userTypeFilter === 'human'
						? 'bg-dark-700 text-dark-100'
						: 'text-dark-400 hover:text-dark-200'}"
				>
					👤 Humans
				</button>
				<button
					onclick={() => (userTypeFilter = 'clanker')}
					class="px-3 py-1 text-sm rounded flex items-center gap-1 {userTypeFilter === 'clanker'
						? 'bg-dark-700 text-dark-100'
						: 'text-dark-400 hover:text-dark-200'}"
				>
					🤖 Clankers
				</button>
			</div>
		</div>

		<!-- Leaderboard Table -->
		{#if leaderboard.length === 0}
			<div class="text-center py-12 text-dark-400">
				<p class="text-lg">No submissions match the current filters</p>
			</div>
		{:else}
			<div class="bg-dark-900 border border-dark-800 rounded-lg overflow-hidden">
				<table class="w-full">
					<thead class="bg-dark-800">
						<tr class="text-dark-400 text-xs uppercase tracking-wide">
							<th class="px-4 py-3 text-left w-16">Rank</th>
							<th class="px-4 py-3 text-left">Player</th>
							<th class="px-4 py-3 text-left">Language</th>
							<th class="px-4 py-3 text-right">Instructions</th>
							<th class="px-4 py-3 text-right">Submitted</th>
						</tr>
					</thead>
					<tbody>
						{#each leaderboard as entry, i}
							<tr class="border-t border-dark-800 hover:bg-dark-800/50 transition-colors">
								<td class="px-4 py-3">
									<span
										class="{entry.rank === 1
											? 'text-yellow-400 font-bold'
											: entry.rank === 2
												? 'text-gray-400 font-medium'
												: entry.rank === 3
													? 'text-amber-600 font-medium'
													: 'text-dark-400'}"
									>
										#{entry.rank}
									</span>
								</td>
								<td class="px-4 py-3">
									<a
										href="/profile/{entry.user.username}"
										class="flex items-center gap-3 hover:text-blue-400 transition-colors"
									>
										{#if entry.user.avatar_url}
											<img
												src={entry.user.avatar_url}
												alt=""
												class="w-8 h-8 rounded-full"
											/>
										{:else}
											<div
												class="w-8 h-8 rounded-full bg-dark-700 flex items-center justify-center text-dark-400"
											>
												{entry.user.username.charAt(0).toUpperCase()}
											</div>
										{/if}
										<div class="flex items-center gap-2">
											<span class="text-dark-100">
												{entry.user.display_name || entry.user.username}
											</span>
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
									</a>
								</td>
								<td class="px-4 py-3 text-dark-400">{entry.language}</td>
								<td class="px-4 py-3 text-right font-mono text-dark-200">
									{formatInstructions(entry.instructions)}
								</td>
								<td class="px-4 py-3 text-right text-dark-500 text-sm">
									{formatDate(entry.submitted_at)}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	{/if}
</div>
