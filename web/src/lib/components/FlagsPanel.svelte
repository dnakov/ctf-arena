<script lang="ts">
	import { currentLanguage, languageFlags, compilerFlags, type FlagDefinition } from '$lib/stores/editor';

	$: flags = languageFlags[$currentLanguage] || [];

	function updateFlag(name: string, value: string) {
		compilerFlags.update(f => ({ ...f, [name]: value }));
	}

	function getBooleanValue(name: string): boolean {
		return $compilerFlags[name] === 'true';
	}

	function toggleBoolean(name: string) {
		const current = $compilerFlags[name] === 'true';
		updateFlag(name, current ? 'false' : 'true');
	}
</script>

{#if flags.length > 0}
	<div class="space-y-3">
		{#each flags as flag (flag.name)}
			<div class="flex items-center gap-3">
				<label class="text-dark-400 text-sm w-24 flex-shrink-0" title={flag.description}>
					{flag.label}
				</label>

				{#if flag.type === 'select'}
					<select
						class="flex-1 bg-dark-800 border border-dark-600 rounded px-2 py-1 text-dark-100 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
						value={$compilerFlags[flag.name] ?? flag.default}
						on:change={(e) => updateFlag(flag.name, e.currentTarget.value)}
					>
						{#each flag.options || [] as opt}
							<option value={opt.value}>{opt.label}</option>
						{/each}
					</select>
				{:else if flag.type === 'boolean'}
					<button
						type="button"
						class="relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500 {getBooleanValue(flag.name) ? 'bg-blue-600' : 'bg-dark-600'}"
						on:click={() => toggleBoolean(flag.name)}
					>
						<span
							class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {getBooleanValue(flag.name) ? 'translate-x-4' : 'translate-x-0'}"
						></span>
					</button>
				{:else if flag.type === 'text'}
					<input
						type="text"
						class="flex-1 bg-dark-800 border border-dark-600 rounded px-2 py-1 text-dark-100 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
						value={$compilerFlags[flag.name] ?? flag.default}
						placeholder={flag.description}
						on:input={(e) => updateFlag(flag.name, e.currentTarget.value)}
					/>
				{/if}
			</div>
		{/each}
	</div>
{:else}
	<p class="text-dark-500 text-sm italic">No compiler options available for this language</p>
{/if}
