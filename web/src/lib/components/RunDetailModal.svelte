<script lang="ts">
	import type { ExecutionResult, BinaryMetadata } from '$lib/api/client';
	import type { BenchmarkImpl } from '$lib/benchmarks';

	interface Props {
		result: {
			implementation: BenchmarkImpl;
			binaryId?: string;
			binaryMetadata?: BinaryMetadata;
			compileResult?: {
				binary_size: number;
				compile_time_ms: number;
				cached: boolean;
			};
			executionResult?: ExecutionResult;
		};
		onClose: () => void;
	}

	let { result, onClose }: Props = $props();

	function formatNumber(n: number): string {
		if (n >= 1_000_000_000) return (n / 1_000_000_000).toFixed(2) + 'B';
		if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + 'M';
		if (n >= 1_000) return (n / 1_000).toFixed(2) + 'K';
		return n.toLocaleString();
	}

	function formatBytes(bytes: number): string {
		if (bytes >= 1024 * 1024 * 1024) return (bytes / 1024 / 1024 / 1024).toFixed(2) + ' GB';
		if (bytes >= 1024 * 1024) return (bytes / 1024 / 1024).toFixed(2) + ' MB';
		if (bytes >= 1024) return (bytes / 1024).toFixed(2) + ' KB';
		return bytes + ' B';
	}

	function decodeBase64(str: string): string {
		try {
			return atob(str);
		} catch {
			return str;
		}
	}

	// Sort syscalls by count descending
	function sortedSyscalls(): [string, number][] {
		if (!result.executionResult?.syscall_breakdown) return [];
		return Object.entries(result.executionResult.syscall_breakdown).sort((a, b) => b[1] - a[1]);
	}

	let activeTab: 'source' | 'stdout' | 'stderr' = $state('source');
</script>

<div
	class="fixed inset-0 bg-black/60 z-50 flex items-center justify-center p-4"
	onclick={(e) => e.target === e.currentTarget && onClose()}
	onkeydown={(e) => e.key === 'Escape' && onClose()}
	role="dialog"
	aria-modal="true"
	tabindex="-1"
>
	<div class="bg-dark-900 rounded-lg max-w-5xl w-full max-h-[90vh] overflow-hidden flex flex-col">
		<!-- Header -->
		<div class="flex items-center justify-between p-4 border-b border-dark-700">
			<div>
				<h2 class="text-xl font-semibold text-dark-100">{result.implementation.name}</h2>
				<p class="text-sm text-dark-400">
					{result.implementation.language} - {result.implementation.tier}
				</p>
			</div>
			<button
				class="p-2 text-dark-400 hover:text-dark-100 hover:bg-dark-700 rounded-lg transition-colors"
				onclick={onClose}
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					class="h-6 w-6"
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M6 18L18 6M6 6l12 12"
					/>
				</svg>
			</button>
		</div>

		<div class="flex-1 overflow-auto p-4 space-y-6">
			<!-- Stats Grid -->
			{#if result.executionResult}
				<div class="grid grid-cols-2 md:grid-cols-4 gap-4">
					<div class="bg-dark-800 rounded-lg p-4">
						<div class="text-sm text-dark-400">Instructions</div>
						<div
							class="text-2xl font-bold {result.executionResult.limit_reached
								? 'text-red-400'
								: 'text-green-400'}"
						>
							{formatNumber(result.executionResult.instructions)}
						</div>
					</div>
					<div class="bg-dark-800 rounded-lg p-4">
						<div class="text-sm text-dark-400">Guest Memory</div>
						<div class="text-2xl font-bold text-blue-400">
							{#if (result.executionResult.guest_mmap_peak ?? 0) > 0 || (result.executionResult.guest_heap_bytes ?? 0) > 0}
								{formatBytes((result.executionResult.guest_mmap_peak ?? 0) + (result.executionResult.guest_heap_bytes ?? 0))}
							{:else}
								—
							{/if}
						</div>
					</div>
					<div class="bg-dark-800 rounded-lg p-4">
						<div class="text-sm text-dark-400">Exit Code</div>
						<div
							class="text-2xl font-bold {result.executionResult.exit_code === 0
								? 'text-green-400'
								: 'text-red-400'}"
						>
							{result.executionResult.exit_code}
						</div>
					</div>
					<div class="bg-dark-800 rounded-lg p-4">
						<div class="text-sm text-dark-400">Exec Time</div>
						<div class="text-2xl font-bold text-dark-100">
							{result.executionResult.execution_time_ms}ms
						</div>
					</div>
				</div>

				<!-- Guest Memory Details (actual binary allocations) -->
				{#if (result.executionResult.guest_mmap_peak ?? 0) > 0 || (result.executionResult.guest_heap_bytes ?? 0) > 0}
					<div class="bg-dark-800 rounded-lg p-4">
						<h3 class="text-sm font-medium text-dark-300 mb-3">Guest Memory (Binary Allocations)</h3>
						<div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
							{#if result.executionResult.guest_mmap_peak}
								<div>
									<span class="text-dark-400">mmap peak:</span>
									<span class="text-dark-100 ml-2">{formatBytes(result.executionResult.guest_mmap_peak)}</span>
								</div>
							{/if}
							{#if result.executionResult.guest_mmap_bytes !== undefined}
								<div>
									<span class="text-dark-400">mmap current:</span>
									<span class="text-dark-100 ml-2">{formatBytes(result.executionResult.guest_mmap_bytes)}</span>
								</div>
							{/if}
							{#if result.executionResult.guest_heap_bytes}
								<div>
									<span class="text-dark-400">heap (brk):</span>
									<span class="text-dark-100 ml-2">{formatBytes(result.executionResult.guest_heap_bytes)}</span>
								</div>
							{/if}
						</div>
					</div>
				{/if}

				<!-- QEMU Process Memory (not guest memory) -->
				{#if result.executionResult.memory_hwm_kb || result.executionResult.memory_rss_kb}
					<div class="bg-dark-800 rounded-lg p-4">
						<h3 class="text-sm font-medium text-dark-300 mb-3">QEMU Process Memory <span class="text-dark-500 font-normal">(emulator overhead)</span></h3>
						<div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
							{#if result.executionResult.memory_hwm_kb}
								<div>
									<span class="text-dark-400">HWM:</span>
									<span class="text-dark-100 ml-2"
										>{formatBytes(result.executionResult.memory_hwm_kb * 1024)}</span
									>
								</div>
							{/if}
							{#if result.executionResult.memory_rss_kb}
								<div>
									<span class="text-dark-400">RSS:</span>
									<span class="text-dark-100 ml-2"
										>{formatBytes(result.executionResult.memory_rss_kb * 1024)}</span
									>
								</div>
							{/if}
							{#if result.executionResult.memory_data_kb}
								<div>
									<span class="text-dark-400">Data:</span>
									<span class="text-dark-100 ml-2"
										>{formatBytes(result.executionResult.memory_data_kb * 1024)}</span
									>
								</div>
							{/if}
							{#if result.executionResult.memory_stack_kb}
								<div>
									<span class="text-dark-400">Stack:</span>
									<span class="text-dark-100 ml-2"
										>{formatBytes(result.executionResult.memory_stack_kb * 1024)}</span
									>
								</div>
							{/if}
						</div>
					</div>
				{/if}

				<!-- I/O Stats -->
				{#if result.executionResult.io_read_bytes || result.executionResult.io_write_bytes}
					<div class="bg-dark-800 rounded-lg p-4">
						<h3 class="text-sm font-medium text-dark-300 mb-3">I/O</h3>
						<div class="grid grid-cols-2 gap-4 text-sm">
							<div>
								<span class="text-dark-400">Read:</span>
								<span class="text-dark-100 ml-2"
									>{formatBytes(result.executionResult.io_read_bytes ?? 0)}</span
								>
							</div>
							<div>
								<span class="text-dark-400">Write:</span>
								<span class="text-dark-100 ml-2"
									>{formatBytes(result.executionResult.io_write_bytes ?? 0)}</span
								>
							</div>
						</div>
					</div>
				{/if}

				<!-- Syscalls -->
				{#if result.executionResult.syscalls && result.executionResult.syscalls > 0}
					<div class="bg-dark-800 rounded-lg p-4">
						<h3 class="text-sm font-medium text-dark-300 mb-3">
							Syscalls ({result.executionResult.syscalls.toLocaleString()})
						</h3>
						<div class="flex flex-wrap gap-2">
							{#each sortedSyscalls() as [name, count]}
								<span
									class="px-2 py-1 bg-dark-700 rounded text-sm text-dark-200 font-mono"
									title="{name}: {count}"
								>
									{name}: {count}
								</span>
							{/each}
						</div>
					</div>
				{/if}
			{/if}

			<!-- Compile Info -->
			{#if result.compileResult || result.binaryMetadata}
				<div class="bg-dark-800 rounded-lg p-4">
					<h3 class="text-sm font-medium text-dark-300 mb-3">Binary Info</h3>
					<div class="grid grid-cols-2 md:grid-cols-3 gap-4 text-sm mb-3">
						{#if result.compileResult}
							<div>
								<span class="text-dark-400">Size:</span>
								<span class="text-dark-100 ml-2">{formatBytes(result.compileResult.binary_size)}</span>
							</div>
							<div>
								<span class="text-dark-400">Compile Time:</span>
								<span class="text-dark-100 ml-2">{result.compileResult.compile_time_ms}ms</span>
							</div>
							<div>
								<span class="text-dark-400">Cached:</span>
								<span class="text-dark-100 ml-2">{result.compileResult.cached ? 'Yes' : 'No'}</span>
							</div>
						{/if}
					</div>
					{#if result.binaryMetadata?.compiler_version}
						<div class="text-sm mb-3">
							<span class="text-dark-400">Compiler:</span>
							<span class="text-dark-200 ml-2 font-mono">{result.binaryMetadata.compiler_version}</span>
						</div>
					{/if}
					{#if result.binaryMetadata?.compile_flags && Object.keys(result.binaryMetadata.compile_flags).length > 0}
						<div class="text-sm">
							<span class="text-dark-400 block mb-2">Compile Flags:</span>
							<div class="flex flex-wrap gap-2">
								{#each Object.entries(result.binaryMetadata.compile_flags) as [key, value]}
									<span class="px-2 py-1 bg-dark-700 rounded text-dark-200 font-mono text-xs">
										{key}={value}
									</span>
								{/each}
							</div>
						</div>
					{/if}
				</div>
			{/if}

			<!-- Tabs for Source/Output -->
			<div class="bg-dark-800 rounded-lg overflow-hidden">
				<div class="flex border-b border-dark-700">
					<button
						class="px-4 py-2 text-sm font-medium transition-colors {activeTab === 'source'
							? 'text-blue-400 border-b-2 border-blue-400'
							: 'text-dark-400 hover:text-dark-200'}"
						onclick={() => (activeTab = 'source')}
					>
						Source Code
					</button>
					{#if result.executionResult?.stdout}
						<button
							class="px-4 py-2 text-sm font-medium transition-colors {activeTab === 'stdout'
								? 'text-blue-400 border-b-2 border-blue-400'
								: 'text-dark-400 hover:text-dark-200'}"
							onclick={() => (activeTab = 'stdout')}
						>
							stdout
						</button>
					{/if}
					{#if result.executionResult?.stderr}
						<button
							class="px-4 py-2 text-sm font-medium transition-colors {activeTab === 'stderr'
								? 'text-blue-400 border-b-2 border-blue-400'
								: 'text-dark-400 hover:text-dark-200'}"
							onclick={() => (activeTab = 'stderr')}
						>
							stderr
						</button>
					{/if}
				</div>
				<div class="p-4 max-h-96 overflow-auto">
					{#if activeTab === 'source'}
						<pre
							class="text-sm font-mono text-dark-200 whitespace-pre-wrap">{result.implementation
								.source_code || 'No source code available'}</pre>
					{:else if activeTab === 'stdout' && result.executionResult?.stdout}
						<pre
							class="text-sm font-mono text-dark-200 whitespace-pre-wrap">{decodeBase64(
								result.executionResult.stdout
							)}</pre>
					{:else if activeTab === 'stderr' && result.executionResult?.stderr}
						<pre
							class="text-sm font-mono text-red-400 whitespace-pre-wrap">{decodeBase64(
								result.executionResult.stderr
							)}</pre>
					{/if}
				</div>
			</div>
		</div>
	</div>
</div>
