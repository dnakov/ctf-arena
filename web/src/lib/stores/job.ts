import { writable, derived } from 'svelte/store';
import { api, type ExecutionResult, type CompileResultResponse } from '$lib/api/client';

export type JobPhase = 'idle' | 'compiling' | 'running' | 'completed' | 'error';

export interface JobState {
	phase: JobPhase;
	compileJobId: string | null;
	executeJobId: string | null;
	compileResult: CompileResultResponse | null;
	executeResult: ExecutionResult | null;
	error: string | null;
	compilePosition: number | null;
	executePosition: number | null;
}

const initialState: JobState = {
	phase: 'idle',
	compileJobId: null,
	executeJobId: null,
	compileResult: null,
	executeResult: null,
	error: null,
	compilePosition: null,
	executePosition: null
};

function createJobStore() {
	const { subscribe, set, update } = writable<JobState>(initialState);

	return {
		subscribe,

		reset() {
			set(initialState);
		},

		async compileAndRun(
			sourceCode: string,
			language: string,
			optimization: string,
			instructionLimit: number,
			stdin: string,
			flags: Record<string, string> = {}
		) {
			// Reset state
			set({
				...initialState,
				phase: 'compiling'
			});

			try {
				// Step 1: Submit compile job
				const compileResponse = await api.compile(
					sourceCode,
					language as any,
					optimization as any,
					flags
				);

				update((s) => ({
					...s,
					compileJobId: compileResponse.compile_job_id,
					compilePosition: compileResponse.position
				}));

				// Step 2: Poll for compile completion
				const compileResult = await this.pollCompile(compileResponse.compile_job_id);

				update((s) => ({
					...s,
					compileResult,
					phase: 'running'
				}));

				// Step 3: Submit execution job
				const submitResponse = await api.submit(
					compileResult.binary_id,
					instructionLimit,
					stdin || undefined
				);

				update((s) => ({
					...s,
					executeJobId: submitResponse.job_id,
					executePosition: submitResponse.position
				}));

				// Step 4: Poll for execution completion
				const executeResult = await this.pollExecute(submitResponse.job_id);

				update((s) => ({
					...s,
					executeResult,
					phase: 'completed'
				}));

				return executeResult;
			} catch (err) {
				const errorMessage = err instanceof Error ? err.message : 'Unknown error';
				update((s) => ({
					...s,
					phase: 'error',
					error: errorMessage
				}));
				throw err;
			}
		},

		async pollCompile(jobId: string): Promise<CompileResultResponse> {
			const timeout = 120000;
			const startTime = Date.now();

			while (Date.now() - startTime < timeout) {
				const status = await api.compileStatus(jobId);

				update((s) => ({
					...s,
					compilePosition: status.position
				}));

				if (status.status === 'completed') {
					return api.compileResult(jobId);
				}

				if (status.status === 'failed') {
					throw new Error(status.error || 'Compilation failed');
				}

				await new Promise((resolve) => setTimeout(resolve, 500));
			}

			throw new Error('Compile timeout');
		},

		async pollExecute(jobId: string): Promise<ExecutionResult> {
			const timeout = 60000;
			const startTime = Date.now();

			while (Date.now() - startTime < timeout) {
				const status = await api.status(jobId);

				update((s) => ({
					...s,
					executePosition: status.position
				}));

				if (status.status === 'completed') {
					return api.result(jobId);
				}

				if (status.status === 'failed') {
					throw new Error(status.error || 'Execution failed');
				}

				await new Promise((resolve) => setTimeout(resolve, 250));
			}

			throw new Error('Execution timeout');
		}
	};
}

export const jobStore = createJobStore();

// Derived stores for easy access
export const jobPhase = derived(jobStore, ($job) => $job.phase);
export const isRunning = derived(jobStore, ($job) => $job.phase === 'compiling' || $job.phase === 'running');
export const executeResult = derived(jobStore, ($job) => $job.executeResult);
export const compileResult = derived(jobStore, ($job) => $job.compileResult);
export const jobError = derived(jobStore, ($job) => $job.error);
