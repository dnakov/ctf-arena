<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { sourceCode, currentLanguageInfo } from '$lib/stores/editor';
	import type * as Monaco from 'monaco-editor';

	let container: HTMLDivElement;
	let editor: Monaco.editor.IStandaloneCodeEditor | null = null;
	let monaco: typeof Monaco;

	// Define dark theme
	const darkTheme: Monaco.editor.IStandaloneThemeData = {
		base: 'vs-dark',
		inherit: true,
		rules: [
			{ token: 'comment', foreground: '6A9955' },
			{ token: 'keyword', foreground: '569CD6' },
			{ token: 'string', foreground: 'CE9178' },
			{ token: 'number', foreground: 'B5CEA8' },
			{ token: 'type', foreground: '4EC9B0' }
		],
		colors: {
			'editor.background': '#0f172a',
			'editor.foreground': '#e2e8f0',
			'editorLineNumber.foreground': '#64748b',
			'editorLineNumber.activeForeground': '#94a3b8',
			'editor.selectionBackground': '#334155',
			'editor.lineHighlightBackground': '#1e293b'
		}
	};

	onMount(async () => {
		// Dynamic import for Monaco
		monaco = await import('monaco-editor');

		// Define custom theme
		monaco.editor.defineTheme('ctf-dark', darkTheme);

		// Create editor
		editor = monaco.editor.create(container, {
			value: $sourceCode,
			language: $currentLanguageInfo.monacoLanguage,
			theme: 'ctf-dark',
			automaticLayout: true,
			minimap: { enabled: false },
			fontSize: 14,
			lineNumbers: 'on',
			scrollBeyondLastLine: false,
			wordWrap: 'off',
			tabSize: 4,
			insertSpaces: true,
			padding: { top: 16, bottom: 16 }
		});

		// Sync editor content to store
		editor.onDidChangeModelContent(() => {
			if (editor) {
				sourceCode.set(editor.getValue());
			}
		});
	});

	// Update editor when language changes
	$: if (editor && monaco && $currentLanguageInfo) {
		const model = editor.getModel();
		if (model) {
			monaco.editor.setModelLanguage(model, $currentLanguageInfo.monacoLanguage);
		}
	}

	// Update editor content when source code changes externally (e.g., language switch)
	$: if (editor && $sourceCode !== editor.getValue()) {
		editor.setValue($sourceCode);
	}

	onDestroy(() => {
		editor?.dispose();
	});
</script>

<div class="monaco-editor-container rounded-lg overflow-hidden border border-dark-700" bind:this={container}></div>

<style>
	.monaco-editor-container {
		height: 500px;
		min-height: 300px;
	}
</style>
