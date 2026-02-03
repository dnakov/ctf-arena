<script lang="ts">
	import { jobStore, jobPhase } from '$lib/stores/job';

	// Status configuration
	const statusConfig = {
		idle: { text: 'Ready', color: 'text-dark-400', bgColor: 'bg-dark-700' },
		compiling: { text: 'Compiling', color: 'text-yellow-400', bgColor: 'bg-yellow-900/20' },
		running: { text: 'Running', color: 'text-blue-400', bgColor: 'bg-blue-900/20' },
		completed: { text: 'Completed', color: 'text-green-400', bgColor: 'bg-green-900/20' },
		error: { text: 'Error', color: 'text-red-400', bgColor: 'bg-red-900/20' }
	};

	$: config = statusConfig[$jobPhase];
	$: showPosition = $jobPhase === 'compiling' || $jobPhase === 'running';
	$: position =
		$jobPhase === 'compiling' ? $jobStore.compilePosition : $jobStore.executePosition;
</script>

<div class="flex items-center gap-3">
	<!-- Status indicator dot -->
	<div class="flex items-center gap-2">
		<span
			class="relative flex h-3 w-3 {$jobPhase === 'compiling' || $jobPhase === 'running'
				? ''
				: ''}"
		>
			{#if $jobPhase === 'compiling' || $jobPhase === 'running'}
				<span
					class="animate-ping absolute inline-flex h-full w-full rounded-full {config.bgColor} opacity-75"
				></span>
			{/if}
			<span
				class="relative inline-flex rounded-full h-3 w-3 {$jobPhase === 'compiling'
					? 'bg-yellow-400'
					: $jobPhase === 'running'
						? 'bg-blue-400'
						: $jobPhase === 'completed'
							? 'bg-green-400'
							: $jobPhase === 'error'
								? 'bg-red-400'
								: 'bg-dark-500'}"
			></span>
		</span>

		<span class="font-medium {config.color}">
			{config.text}
		</span>
	</div>

	<!-- Queue position -->
	{#if showPosition && position !== null && position > 0}
		<span class="text-dark-400 text-sm">
			(Queue: #{position})
		</span>
	{/if}
</div>
