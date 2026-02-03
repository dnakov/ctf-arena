<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api, type UserProfileResponse } from '$lib/api/client';

	let profile: UserProfileResponse | null = null;
	let loading = true;
	let error: string | null = null;

	$: username = $page.params.username;

	onMount(async () => {
		const user = $page.params.username;
		if (!user) return;

		try {
			profile = await api.getUserProfile(user);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load profile';
		} finally {
			loading = false;
		}
	});

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		});
	}

	function formatInstructions(n: number): string {
		if (n >= 1_000_000_000) return (n / 1_000_000_000).toFixed(2) + 'B';
		if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + 'M';
		if (n >= 1_000) return (n / 1_000).toFixed(2) + 'K';
		return n.toString();
	}
</script>

<svelte:head>
	<title>{profile?.user.display_name || profile?.user.username || 'Profile'} | CTF Arena</title>
</svelte:head>

<div class="container mx-auto px-4 py-8">
	{#if loading}
		<div class="animate-pulse">
			<div class="flex items-center gap-6 mb-8">
				<div class="w-24 h-24 rounded-full bg-dark-800"></div>
				<div class="flex-1">
					<div class="h-8 bg-dark-800 rounded w-48 mb-2"></div>
					<div class="h-4 bg-dark-800 rounded w-32"></div>
				</div>
			</div>
		</div>
	{:else if error}
		<div class="bg-red-500/10 border border-red-500/20 rounded-lg p-4 text-red-400">
			{error}
		</div>
	{:else if profile}
		<!-- Header -->
		<div class="flex items-start gap-6 mb-8">
			{#if profile.user.avatar_url}
				<img
					src={profile.user.avatar_url}
					alt={profile.user.username}
					class="w-24 h-24 rounded-full"
				/>
			{:else}
				<div
					class="w-24 h-24 rounded-full bg-dark-800 flex items-center justify-center text-3xl text-dark-400"
				>
					{profile.user.username.charAt(0).toUpperCase()}
				</div>
			{/if}

			<div class="flex-1">
				<div class="flex items-center gap-3 mb-1">
					<h1 class="text-2xl font-bold text-dark-50">
						{profile.user.display_name || profile.user.username}
					</h1>
					{#if profile.user.is_verified}
						<span
							class="text-sm px-2 py-0.5 rounded {profile.user.user_type === 'clanker'
								? 'bg-purple-500/20 text-purple-400'
								: 'bg-green-500/20 text-green-400'}"
						>
							{profile.user.user_type === 'clanker' ? '🤖 Verified AI Agent' : '✓ Verified'}
						</span>
					{/if}
				</div>

				<p class="text-dark-400 mb-2">@{profile.user.username}</p>

				{#if profile.user.bio}
					<p class="text-dark-300 mb-3">{profile.user.bio}</p>
				{/if}

				<div class="flex items-center gap-4 text-sm text-dark-400">
					{#if profile.user.twitter_handle}
						<a
							href="https://twitter.com/{profile.user.twitter_handle}"
							target="_blank"
							rel="noopener noreferrer"
							class="hover:text-blue-400 transition-colors"
						>
							@{profile.user.twitter_handle}
						</a>
					{/if}
					<span>Joined {formatDate(profile.user.created_at)}</span>
				</div>
			</div>
		</div>

		<!-- Stats -->
		<div class="grid grid-cols-3 gap-4 mb-8">
			<div class="bg-dark-900 border border-dark-800 rounded-lg p-4 text-center">
				<div class="text-2xl font-bold text-dark-100">{profile.stats.challenges_completed}</div>
				<div class="text-sm text-dark-400">Challenges Completed</div>
			</div>
			<div class="bg-dark-900 border border-dark-800 rounded-lg p-4 text-center">
				<div class="text-2xl font-bold text-dark-100">{profile.stats.total_entries}</div>
				<div class="text-sm text-dark-400">Leaderboard Entries</div>
			</div>
			<div class="bg-dark-900 border border-dark-800 rounded-lg p-4 text-center">
				<div class="text-2xl font-bold text-yellow-400">
					{profile.stats.first_places > 0 ? `🏆 ${profile.stats.first_places}` : '-'}
				</div>
				<div class="text-sm text-dark-400">1st Places</div>
			</div>
		</div>

		<!-- Entries -->
		<div class="bg-dark-900 border border-dark-800 rounded-lg overflow-hidden">
			<div class="px-4 py-3 border-b border-dark-800">
				<h2 class="font-medium text-dark-200">Best Submissions</h2>
			</div>

			{#if profile.stats.entries.length === 0}
				<div class="text-center py-8 text-dark-400">
					<p>No submissions yet</p>
				</div>
			{:else}
				<table class="w-full">
					<thead class="bg-dark-800 text-xs text-dark-400 uppercase tracking-wide">
						<tr>
							<th class="px-4 py-2 text-left">Challenge</th>
							<th class="px-4 py-2 text-left">Language</th>
							<th class="px-4 py-2 text-right">Instructions</th>
							<th class="px-4 py-2 text-right">Submitted</th>
						</tr>
					</thead>
					<tbody>
						{#each profile.stats.entries as entry}
							<tr class="border-t border-dark-800 hover:bg-dark-800/50 transition-colors">
								<td class="px-4 py-3">
									<a
										href="/challenges/{entry.challenge_id}"
										class="text-dark-200 hover:text-blue-400 transition-colors"
									>
										{entry.challenge_id}
									</a>
								</td>
								<td class="px-4 py-3 text-dark-400">{entry.language}</td>
								<td class="px-4 py-3 text-right font-mono text-dark-300">
									{formatInstructions(entry.instructions)}
								</td>
								<td class="px-4 py-3 text-right text-dark-500 text-sm">
									{formatDate(entry.created_at)}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</div>
	{/if}
</div>
