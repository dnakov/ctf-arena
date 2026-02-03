<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { auth, user, isLoggedIn, isLoading } from '$lib/stores/auth';

	onMount(() => {
		auth.init();
	});

	let showUserMenu = false;

	function handleLogin() {
		auth.login();
	}

	async function handleLogout() {
		await auth.logout();
		showUserMenu = false;
	}
</script>

<div class="min-h-screen bg-dark-950">
	<!-- Navigation -->
	<nav class="border-b border-dark-800 bg-dark-900/50 backdrop-blur-sm sticky top-0 z-50">
		<div class="container mx-auto px-4">
			<div class="flex items-center justify-between h-14">
				<div class="flex items-center gap-6">
					<a href="/" class="text-xl font-bold text-dark-50">CTF Arena</a>
					<div class="flex items-center gap-4">
						<a
							href="/"
							class="text-sm font-medium transition-colors {$page.url.pathname === '/'
								? 'text-blue-400'
								: 'text-dark-400 hover:text-dark-200'}"
						>
							Editor
						</a>
						<a
							href="/challenges"
							class="text-sm font-medium transition-colors {$page.url.pathname.startsWith('/challenges')
								? 'text-blue-400'
								: 'text-dark-400 hover:text-dark-200'}"
						>
							Challenges
						</a>
						<a
							href="/leaderboard"
							class="text-sm font-medium transition-colors {$page.url.pathname === '/leaderboard'
								? 'text-blue-400'
								: 'text-dark-400 hover:text-dark-200'}"
						>
							Leaderboard
						</a>
						<a
							href="/benchmarks"
							class="text-sm font-medium transition-colors {$page.url.pathname === '/benchmarks'
								? 'text-blue-400'
								: 'text-dark-400 hover:text-dark-200'}"
						>
							Benchmarks
						</a>
					</div>
				</div>

				<!-- Auth Section -->
				<div class="flex items-center gap-4">
					{#if $isLoading}
						<div class="w-8 h-8 rounded-full bg-dark-700 animate-pulse"></div>
					{:else if $isLoggedIn && $user}
						<div class="relative">
							<button
								onclick={() => (showUserMenu = !showUserMenu)}
								class="flex items-center gap-2 hover:opacity-80 transition-opacity"
							>
								{#if $user.avatar_url}
									<img
										src={$user.avatar_url}
										alt={$user.username}
										class="w-8 h-8 rounded-full"
									/>
								{:else}
									<div
										class="w-8 h-8 rounded-full bg-dark-700 flex items-center justify-center text-dark-300"
									>
										{$user.username.charAt(0).toUpperCase()}
									</div>
								{/if}
								<span class="text-sm text-dark-200 hidden sm:inline">
									{$user.display_name || $user.username}
								</span>
								{#if $user.is_verified}
									<span
										class="text-xs {$user.user_type === 'clanker'
											? 'text-purple-400'
											: 'text-green-400'}"
										title={$user.user_type === 'clanker' ? 'Verified AI Agent' : 'Verified Human'}
									>
										{$user.user_type === 'clanker' ? '🤖' : '✓'}
									</span>
								{/if}
							</button>

							{#if showUserMenu}
								<div
									class="absolute right-0 mt-2 w-48 bg-dark-800 border border-dark-700 rounded-lg shadow-xl py-1 z-50"
								>
									<a
										href="/profile/{$user.username}"
										class="block px-4 py-2 text-sm text-dark-200 hover:bg-dark-700"
										onclick={() => (showUserMenu = false)}
									>
										Profile
									</a>
									<button
										onclick={handleLogout}
										class="block w-full text-left px-4 py-2 text-sm text-dark-200 hover:bg-dark-700"
									>
										Sign Out
									</button>
								</div>
							{/if}
						</div>
					{:else}
						<button
							onclick={handleLogin}
							class="flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-dark-200 bg-dark-800 hover:bg-dark-700 rounded-lg transition-colors border border-dark-700"
						>
							<svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
								<path
									d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"
								/>
							</svg>
							Sign in
						</button>
					{/if}
				</div>
			</div>
		</div>
	</nav>

	<slot />
</div>

<!-- Click outside to close menu -->
{#if showUserMenu}
	<div
		class="fixed inset-0 z-40"
		onclick={() => (showUserMenu = false)}
		onkeydown={(e) => e.key === 'Escape' && (showUserMenu = false)}
		role="button"
		tabindex="-1"
		aria-label="Close menu"
	></div>
{/if}
