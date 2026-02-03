// API client for CTF Arena backend

const API_BASE = '/api';

export interface CompileSubmitResponse {
	compile_job_id: string;
	status: string;
	position: number | null;
}

export interface CompileStatusResponse {
	compile_job_id: string;
	status: 'pending' | 'compiling' | 'completed' | 'failed';
	position: number | null;
	created_at: string | null;
	started_at: string | null;
	completed_at: string | null;
	error: string | null;
}

export interface CompileResultResponse {
	binary_id: string;
	binary_size: number;
	compile_time_ms: number;
	cached: boolean;
}

export interface SubmitResponse {
	job_id: string;
	status: string;
	position: number | null;
}

export interface StatusResponse {
	job_id: string;
	status: 'pending' | 'running' | 'completed' | 'failed';
	position: number | null;
	created_at: string | null;
	started_at: string | null;
	completed_at: string | null;
	error: string | null;
}

export interface ExecutionResult {
	instructions: number;
	memory_peak_kb: number;
	memory_rss_kb?: number;
	memory_hwm_kb?: number;
	memory_data_kb?: number;
	memory_stack_kb?: number;
	io_read_bytes?: number;
	io_write_bytes?: number;
	// Guest memory (actual binary allocations)
	guest_mmap_bytes?: number;
	guest_mmap_peak?: number;
	guest_heap_bytes?: number;
	limit_reached: boolean;
	exit_code: number;
	stdout: string; // base64 encoded
	stderr: string; // base64 encoded
	execution_time_ms: number;
	syscalls: number;
	syscall_breakdown: Record<string, number>;
}

export interface BinaryMetadata {
	language?: string;
	optimization?: string;
	compiler_version?: string;
	compile_flags?: Record<string, string>;
}

export interface RunDetails {
	id: string;
	job_id: string;
	binary_id: string;
	binary_size?: number;
	source_code?: string;
	language?: string;
	optimization?: string;
	compiler_version?: string;
	compile_time_ms?: number;
	compile_cached?: boolean;
	instructions: number;
	memory_peak_kb?: number;
	memory_rss_kb?: number;
	memory_hwm_kb?: number;
	memory_data_kb?: number;
	memory_stack_kb?: number;
	io_read_bytes?: number;
	io_write_bytes?: number;
	// Guest memory (actual binary allocations)
	guest_mmap_bytes?: number;
	guest_mmap_peak?: number;
	guest_heap_bytes?: number;
	limit_reached: boolean;
	exit_code?: number;
	execution_time_ms?: number;
	instruction_limit?: number;
	syscalls?: number;
	syscall_breakdown?: Record<string, number>;
	stdout?: string;
	stderr?: string;
	benchmark_id?: string;
	created_at: string;
	started_at?: string;
	completed_at?: string;
}

export interface BenchmarkStats {
	min_instructions: Record<string, number>;
}

export interface HealthResponse {
	status: string;
	docker_available: boolean;
	nats_connected: boolean;
	db_connected: boolean;
}

// ============ Auth Types ============

export interface PublicUser {
	id: string;
	username: string;
	avatar_url: string | null;
	display_name: string | null;
	bio: string | null;
	twitter_handle: string | null;
	is_verified: boolean;
	user_type: 'human' | 'clanker';
	created_at: string;
}

export interface AuthMeResponse {
	user: PublicUser;
}

// ============ Challenge Types ============

export interface ChallengeInfo {
	id: string;
	name: string;
	description: string;
	category: string;
	difficulty: string;
	is_active: boolean;
}

export interface ChallengeListResponse {
	challenges: ChallengeInfo[];
}

export interface PublicTestCase {
	description: string | null;
	stdin: string;
}

export interface ChallengeBaseline {
	language: string;
	name: string;
	tier: string;
	source_code: string;
	reference_instructions?: number;
}

export interface ChallengeDetail {
	id: string;
	name: string;
	description: string;
	category: string;
	difficulty: string;
	input_spec: string | null;
	output_spec: string;
	test_cases: PublicTestCase[];
	verify_mode: string;
	baselines?: ChallengeBaseline[];
}

export interface ChallengeSubmitResponse {
	submission_id: string;
	status: string;
}

export interface TestResult {
	test_index: number;
	passed: boolean;
	expected_preview: string | null;
	actual_preview: string | null;
	error: string | null;
}

export interface SubmissionStatusResponse {
	submission_id: string;
	status: 'pending' | 'compiling' | 'running' | 'passed' | 'failed';
	test_results: TestResult[] | null;
	instructions: number | null;
	error_message: string | null;
	completed_at: string | null;
}

// ============ Leaderboard Types ============

export interface LeaderboardEntry {
	rank: number;
	user: PublicUser;
	instructions: number;
	language: string;
	submitted_at: string;
}

export interface GlobalLeaderboardEntry {
	rank: number;
	user: PublicUser;
	total_score: number;
	challenges_completed: number;
	first_places: number;
}

export interface UserStats {
	challenges_completed: number;
	total_entries: number;
	first_places: number;
	entries: Array<{
		id: string;
		challenge_id: string;
		language: string;
		instructions: number;
		created_at: string;
	}>;
}

export interface UserProfileResponse {
	user: PublicUser;
	stats: UserStats;
}

// ============ Verification Types ============

export interface InitClankerVerificationResponse {
	code: string;
	tweet_text: string;
	expires_in_minutes: number;
}

export interface CheckClankerVerificationResponse {
	verified: boolean;
	user_type: string;
	message: string;
}

export type Language =
	| 'c'
	| 'cpp'
	| 'rust'
	| 'go'
	| 'zig'
	| 'asm'
	| 'nim'
	| 'pascal'
	| 'ocaml'
	| 'swift'
	| 'haskell'
	| 'csharp'
	| 'java'
	| 'kotlin'
	| 'scala'
	| 'clojure'
	| 'python'
	| 'javascript'
	| 'typescript'
	| 'bun'
	| 'deno'
	| 'node'
	| 'lua'
	| 'perl'
	| 'php'
	| 'tcl'
	| 'erlang'
	| 'elixir'
	| 'racket'
	| 'wasm';

export type Optimization = 'debug' | 'release' | 'size';

class ApiClient {
	private async request<T>(path: string, options?: RequestInit): Promise<T> {
		const response = await fetch(`${API_BASE}${path}`, {
			...options,
			headers: {
				...options?.headers
			}
		});

		if (!response.ok) {
			const error = await response.json().catch(() => ({ error: 'Unknown error' }));
			throw new Error(error.error || `HTTP ${response.status}`);
		}

		return response.json();
	}

	async health(): Promise<HealthResponse> {
		return this.request('/health');
	}

	async compile(
		sourceCode: string,
		language: Language,
		optimization: Optimization = 'release',
		flags: Record<string, string> = {}
	): Promise<CompileSubmitResponse> {
		const formData = new FormData();
		formData.append('source_code', sourceCode);
		formData.append('language', language);
		formData.append('optimization', optimization);

		// Add flags as JSON if any non-empty flags exist
		const activeFlags = Object.fromEntries(
			Object.entries(flags).filter(([_, v]) => v !== '' && v !== undefined)
		);
		if (Object.keys(activeFlags).length > 0) {
			formData.append('flags', JSON.stringify(activeFlags));
		}

		return this.request('/compile', {
			method: 'POST',
			body: formData
		});
	}

	async compileStatus(jobId: string): Promise<CompileStatusResponse> {
		return this.request(`/compile/status/${jobId}`);
	}

	async compileResult(jobId: string): Promise<CompileResultResponse> {
		return this.request(`/compile/result/${jobId}`);
	}

	async submit(
		binaryId: string,
		instructionLimit?: number,
		stdin?: string,
		benchmarkId?: string,
		envVars?: Record<string, string>
	): Promise<SubmitResponse> {
		const formData = new FormData();
		formData.append('binary_id', binaryId);
		if (instructionLimit !== undefined) {
			formData.append('instruction_limit', instructionLimit.toString());
		}
		if (stdin) {
			formData.append('stdin', stdin);
		}
		if (benchmarkId) {
			formData.append('benchmark_id', benchmarkId);
		}
		if (envVars && Object.keys(envVars).length > 0) {
			formData.append('env_vars', JSON.stringify(envVars));
		}

		return this.request('/submit', {
			method: 'POST',
			body: formData
		});
	}

	async status(jobId: string): Promise<StatusResponse> {
		return this.request(`/status/${jobId}`);
	}

	async result(jobId: string): Promise<ExecutionResult> {
		return this.request(`/result/${jobId}`);
	}

	// Helper to poll for compile completion
	async waitForCompile(jobId: string, timeoutMs = 120000): Promise<CompileResultResponse> {
		const startTime = Date.now();

		while (Date.now() - startTime < timeoutMs) {
			const status = await this.compileStatus(jobId);

			if (status.status === 'completed') {
				return this.compileResult(jobId);
			}

			if (status.status === 'failed') {
				throw new Error(status.error || 'Compilation failed');
			}

			await new Promise((resolve) => setTimeout(resolve, 500));
		}

		throw new Error('Compile timeout');
	}

	// Helper to poll for execution completion
	async waitForExecution(jobId: string, timeoutMs = 60000): Promise<ExecutionResult> {
		const startTime = Date.now();

		while (Date.now() - startTime < timeoutMs) {
			const status = await this.status(jobId);

			if (status.status === 'completed') {
				return this.result(jobId);
			}

			if (status.status === 'failed') {
				throw new Error(status.error || 'Execution failed');
			}

			await new Promise((resolve) => setTimeout(resolve, 250));
		}

		throw new Error('Execution timeout');
	}

	// Benchmark endpoints
	async listBenchmarks(): Promise<
		Array<{
			id: string;
			name: string;
			description: string;
			implementations: Array<{
				language: string;
				name: string;
				file: string;
				tier: string;
				reference_instructions?: number;
			}>;
		}>
	> {
		return this.request('/benchmarks');
	}

	async getBenchmark(
		id: string,
		includeSource = false
	): Promise<{
		id: string;
		name: string;
		description: string;
		implementations: Array<{
			language: string;
			name: string;
			file: string;
			tier: string;
			reference_instructions?: number;
			source_code?: string;
		}>;
		env_vars?: Record<string, string>;
		stdin?: string;
	}> {
		return this.request(`/benchmarks/${id}?include_source=${includeSource}`);
	}

	async getBenchmarkSource(benchmarkId: string, file: string): Promise<string> {
		const response = await fetch(`${API_BASE}/benchmarks/${benchmarkId}/source/${file}`);
		if (!response.ok) {
			throw new Error(`Failed to fetch source: ${response.status}`);
		}
		return response.text();
	}

	async getBenchmarkStats(benchmarkId: string): Promise<BenchmarkStats> {
		return this.request(`/benchmarks/${benchmarkId}/stats`);
	}

	// Runs endpoints
	async getRun(runId: string): Promise<RunDetails> {
		return this.request(`/runs/${runId}`);
	}

	async getRunByJob(jobId: string): Promise<RunDetails> {
		return this.request(`/runs/job/${jobId}`);
	}

	async listRuns(limit = 50, offset = 0): Promise<RunDetails[]> {
		return this.request(`/runs?limit=${limit}&offset=${offset}`);
	}

	async getBinaryMetadata(binaryId: string): Promise<BinaryMetadata> {
		return this.request(`/binaries/${binaryId}/metadata`);
	}

	// ============ Auth Endpoints ============

	async getMe(): Promise<AuthMeResponse> {
		return this.request('/auth/me', { credentials: 'include' });
	}

	async logout(): Promise<void> {
		await this.request('/auth/logout', {
			method: 'POST',
			credentials: 'include'
		});
	}

	getGitHubLoginUrl(): string {
		return `${API_BASE}/auth/github`;
	}

	// ============ Clanker Verification Endpoints ============

	async initClankerVerification(twitterHandle: string): Promise<InitClankerVerificationResponse> {
		return this.request('/verification/clanker', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ twitter_handle: twitterHandle }),
			credentials: 'include'
		});
	}

	async checkClankerVerification(): Promise<CheckClankerVerificationResponse> {
		return this.request('/verification/clanker/check', {
			method: 'POST',
			credentials: 'include'
		});
	}

	// ============ User Profile Endpoints ============

	async getUserProfile(username: string): Promise<UserProfileResponse> {
		return this.request(`/users/${username}`);
	}

	// ============ Challenge Endpoints ============

	async listChallenges(): Promise<ChallengeListResponse> {
		return this.request('/challenges');
	}

	async getChallenge(id: string): Promise<ChallengeDetail> {
		return this.request(`/challenges/${id}`);
	}

	async submitChallenge(
		challengeId: string,
		sourceCode: string,
		language: Language,
		optimization: Optimization = 'release'
	): Promise<ChallengeSubmitResponse> {
		const formData = new FormData();
		formData.append('source_code', sourceCode);
		formData.append('language', language);
		formData.append('optimization', optimization);

		return this.request(`/challenges/${challengeId}/submit`, {
			method: 'POST',
			body: formData,
			credentials: 'include'
		});
	}

	async getChallengeSubmissionStatus(
		challengeId: string,
		submissionId: string
	): Promise<SubmissionStatusResponse> {
		return this.request(`/challenges/${challengeId}/submission/${submissionId}`, {
			credentials: 'include'
		});
	}

	async getChallengeLeaderboard(
		challengeId: string,
		options?: { language?: string; user_type?: string; limit?: number }
	): Promise<LeaderboardEntry[]> {
		const params = new URLSearchParams();
		if (options?.language) params.set('language', options.language);
		if (options?.user_type) params.set('user_type', options.user_type);
		if (options?.limit) params.set('limit', options.limit.toString());
		const query = params.toString();
		return this.request(`/challenges/${challengeId}/leaderboard${query ? '?' + query : ''}`);
	}

	async getGlobalLeaderboard(
		options?: { user_type?: string; limit?: number }
	): Promise<GlobalLeaderboardEntry[]> {
		const params = new URLSearchParams();
		if (options?.user_type) params.set('user_type', options.user_type);
		if (options?.limit) params.set('limit', options.limit.toString());
		const query = params.toString();
		return this.request(`/leaderboard${query ? '?' + query : ''}`);
	}

	// Helper to poll for challenge submission completion
	async waitForChallengeSubmission(
		challengeId: string,
		submissionId: string,
		timeoutMs = 300000 // 5 minutes for challenges
	): Promise<SubmissionStatusResponse> {
		const startTime = Date.now();

		while (Date.now() - startTime < timeoutMs) {
			const status = await this.getChallengeSubmissionStatus(challengeId, submissionId);

			if (status.status === 'passed' || status.status === 'failed') {
				return status;
			}

			await new Promise((resolve) => setTimeout(resolve, 1000));
		}

		throw new Error('Submission timeout');
	}
}

export const api = new ApiClient();
