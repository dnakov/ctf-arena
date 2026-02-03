<script lang="ts">
	import {
		sourceCode,
		currentLanguage,
		currentOptimization,
		instructionLimit,
		stdin,
		compilerFlags
	} from '$lib/stores/editor';
	import { jobStore, isRunning, jobPhase } from '$lib/stores/job';

	async function handleClick() {
		if ($isRunning) return;

		await jobStore.compileAndRun(
			$sourceCode,
			$currentLanguage,
			$currentOptimization,
			$instructionLimit,
			$stdin,
			$compilerFlags
		);
	}

	// Button text based on phase
	$: buttonText = {
		idle: 'Compile & Run',
		compiling: 'Compiling...',
		running: 'Running...',
		completed: 'Compile & Run',
		error: 'Compile & Run'
	}[$jobPhase];

	// Button disabled state
	$: isDisabled = $isRunning;
</script>

<button
	class="w-full py-3 px-6 rounded-lg font-semibold text-white transition-all
           {isDisabled
		? 'bg-dark-600 cursor-not-allowed'
		: 'bg-green-600 hover:bg-green-500 active:bg-green-700'}"
	disabled={isDisabled}
	onclick={handleClick}
>
	{#if $isRunning}
		<span class="flex items-center justify-center gap-2">
			<svg class="animate-spin h-5 w-5" viewBox="0 0 24 24">
				<circle
					class="opacity-25"
					cx="12"
					cy="12"
					r="10"
					stroke="currentColor"
					stroke-width="4"
					fill="none"
				/>
				<path
					class="opacity-75"
					fill="currentColor"
					d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
				/>
			</svg>
			{buttonText}
		</span>
	{:else}
		<span class="flex items-center justify-center gap-2">
			<svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
				/>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
				/>
			</svg>
			{buttonText}
		</span>
	{/if}
</button>
