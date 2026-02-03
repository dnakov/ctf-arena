// Benchmark types - data fetched from API

import type { Language } from './api/client';

export interface BenchmarkImpl {
	language: string;
	name: string;
	file: string;
	tier: string;
	reference_instructions?: number;
	source_code?: string; // Populated when fetching with include_source=true
}

export interface Benchmark {
	id: string;
	name: string;
	description: string;
	implementations: BenchmarkImpl[];
	env_vars?: Record<string, string>;
	stdin?: string;
}
