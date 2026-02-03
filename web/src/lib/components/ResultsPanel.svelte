<script lang="ts">
	import { jobStore, executeResult, compileResult, jobError, jobPhase } from '$lib/stores/job';

	// Decode base64 output
	function decodeBase64(encoded: string): string {
		try {
			return atob(encoded);
		} catch {
			return encoded;
		}
	}

	// Format instruction count
	function formatInstructions(n: number): string {
		if (n >= 1_000_000_000) {
			return (n / 1_000_000_000).toFixed(2) + 'B';
		} else if (n >= 1_000_000) {
			return (n / 1_000_000).toFixed(2) + 'M';
		} else if (n >= 1_000) {
			return (n / 1_000).toFixed(2) + 'K';
		}
		return n.toLocaleString();
	}

	// Format bytes
	function formatBytes(bytes: number): string {
		if (bytes >= 1_000_000) {
			return (bytes / 1_000_000).toFixed(2) + ' MB';
		} else if (bytes >= 1_000) {
			return (bytes / 1_000).toFixed(2) + ' KB';
		}
		return bytes + ' B';
	}

	$: stdout = $executeResult ? decodeBase64($executeResult.stdout) : '';
	$: stderr = $executeResult ? decodeBase64($executeResult.stderr) : '';
</script>

<div class="bg-dark-900 rounded-lg border border-dark-700 overflow-hidden">
	<!-- Header -->
	<div class="bg-dark-800 px-4 py-3 border-b border-dark-700">
		<h2 class="font-semibold text-dark-100">Results</h2>
	</div>

	<div class="p-4 space-y-4">
		{#if $jobPhase === 'idle'}
			<p class="text-dark-400 text-center py-8">
				Click "Compile & Run" to execute your code
			</p>
		{:else if $jobPhase === 'error'}
			<div class="bg-red-900/20 border border-red-800 rounded-lg p-4">
				<p class="text-red-400 font-medium">Error</p>
				<p class="text-red-300 mt-1 code-output text-sm">{$jobError}</p>
			</div>
		{:else if $jobPhase === 'compiling' || $jobPhase === 'running'}
			<div class="text-center py-8">
				<div
					class="inline-block animate-spin rounded-full h-8 w-8 border-4 border-dark-600 border-t-blue-500"
				></div>
				<p class="text-dark-400 mt-4">
					{$jobPhase === 'compiling' ? 'Compiling your code...' : 'Running your binary...'}
				</p>
			</div>
		{:else if $executeResult}
			<!-- Stats Grid -->
			<div class="grid grid-cols-2 md:grid-cols-4 gap-4">
				<!-- Instructions -->
				<div class="bg-dark-800 rounded-lg p-3">
					<p class="text-dark-400 text-xs uppercase tracking-wider">Instructions</p>
					<p
						class="text-2xl font-bold mt-1 {$executeResult.limit_reached
							? 'text-red-400'
							: 'text-green-400'}"
					>
						{formatInstructions($executeResult.instructions)}
					</p>
					{#if $executeResult.limit_reached}
						<p class="text-red-400 text-xs mt-1">Limit reached!</p>
					{/if}
				</div>

				<!-- Memory -->
				<div class="bg-dark-800 rounded-lg p-3">
					<p class="text-dark-400 text-xs uppercase tracking-wider">Memory Peak</p>
					<p class="text-2xl font-bold text-blue-400 mt-1">
						{formatBytes($executeResult.memory_peak_kb * 1024)}
					</p>
				</div>

				<!-- Exit Code -->
				<div class="bg-dark-800 rounded-lg p-3">
					<p class="text-dark-400 text-xs uppercase tracking-wider">Exit Code</p>
					<p
						class="text-2xl font-bold mt-1 {$executeResult.exit_code === 0
							? 'text-green-400'
							: 'text-yellow-400'}"
					>
						{$executeResult.exit_code}
					</p>
				</div>

				<!-- Execution Time -->
				<div class="bg-dark-800 rounded-lg p-3">
					<p class="text-dark-400 text-xs uppercase tracking-wider">Time</p>
					<p class="text-2xl font-bold text-purple-400 mt-1">
						{$executeResult.execution_time_ms}ms
					</p>
				</div>
			</div>

			<!-- Syscalls -->
			{#if $executeResult.syscalls > 0}
				<div class="bg-dark-800 rounded-lg p-3">
					<p class="text-dark-400 text-xs uppercase tracking-wider mb-2">Syscalls ({$executeResult.syscalls} total)</p>
					<div class="flex flex-wrap gap-2 text-sm">
						{#each Object.entries($executeResult.syscall_breakdown || {}).sort((a, b) => b[1] - a[1]) as [name, count]}
							<span class="bg-dark-700 px-2 py-1 rounded text-dark-200">
								<span class="text-dark-400">{name}:</span> {count}
							</span>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Compile Info (if available) -->
			{#if $compileResult}
				<div class="bg-dark-800 rounded-lg p-3">
					<p class="text-dark-400 text-xs uppercase tracking-wider mb-2">Compilation</p>
					<div class="flex items-center gap-4 text-sm">
						<span class="text-dark-300">
							Binary: <span class="text-dark-100 font-mono"
								>{formatBytes($compileResult.binary_size)}</span
							>
						</span>
						<span class="text-dark-300">
							Compile: <span class="text-dark-100">{$compileResult.compile_time_ms}ms</span>
						</span>
						{#if $compileResult.cached}
							<span
								class="bg-blue-900/30 text-blue-400 px-2 py-0.5 rounded text-xs font-medium"
							>
								Cached
							</span>
						{/if}
					</div>
				</div>
			{/if}

			<!-- stdout -->
			{#if stdout}
				<div>
					<p class="text-dark-400 text-xs uppercase tracking-wider mb-2">stdout</p>
					<pre
						class="bg-dark-950 rounded-lg p-4 text-dark-100 code-output text-sm overflow-x-auto max-h-48 overflow-y-auto">{stdout}</pre>
				</div>
			{/if}

			<!-- stderr -->
			{#if stderr}
				<div>
					<p class="text-dark-400 text-xs uppercase tracking-wider mb-2">stderr</p>
					<pre
						class="bg-dark-950 rounded-lg p-4 text-yellow-300 code-output text-sm overflow-x-auto max-h-48 overflow-y-auto">{stderr}</pre>
				</div>
			{/if}
		{/if}
	</div>
</div>
