<script lang="ts">
	import { languages, currentLanguage, setLanguage, type LanguageInfo } from '$lib/stores/editor';
	import type { Language } from '$lib/api/client';

	// Group languages by tier
	const tiers: { name: string; id: string; languages: LanguageInfo[] }[] = [
		{ name: 'Native', id: 'native', languages: languages.filter((l) => l.tier === 'native') },
		{ name: 'JVM', id: 'jvm', languages: languages.filter((l) => l.tier === 'jvm') },
		{ name: 'Scripting', id: 'scripting', languages: languages.filter((l) => l.tier === 'scripting') },
		{ name: 'Special', id: 'special', languages: languages.filter((l) => l.tier === 'special') }
	];

	function handleSelect(lang: Language) {
		setLanguage(lang);
	}
</script>

<div class="space-y-4">
	{#each tiers as tier}
		<div>
			<h3 class="text-xs font-semibold text-dark-400 uppercase tracking-wider mb-2">
				{tier.name}
			</h3>
			<div class="flex flex-wrap gap-2">
				{#each tier.languages as lang}
					<button
						class="px-3 py-1.5 text-sm rounded-md transition-colors
                           {$currentLanguage === lang.id
							? 'bg-blue-600 text-white'
							: 'bg-dark-800 text-dark-300 hover:bg-dark-700 hover:text-dark-100'}"
						onclick={() => handleSelect(lang.id)}
					>
						{lang.name}
					</button>
				{/each}
			</div>
		</div>
	{/each}
</div>
