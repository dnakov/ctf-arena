import { writable, derived } from 'svelte/store';
import type { Language, Optimization } from '$lib/api/client';

// Language metadata for display
export interface LanguageInfo {
	id: Language;
	name: string;
	extension: string;
	tier: 'native' | 'jvm' | 'scripting' | 'special';
	monacoLanguage: string;
	defaultCode: string;
}

export const languages: LanguageInfo[] = [
	// Tier 1: Native
	{
		id: 'c',
		name: 'C',
		extension: 'c',
		tier: 'native',
		monacoLanguage: 'c',
		defaultCode: `#include <stdio.h>

int main() {
    puts("Hello, World!");
    return 0;
}
`
	},
	{
		id: 'cpp',
		name: 'C++',
		extension: 'cpp',
		tier: 'native',
		monacoLanguage: 'cpp',
		defaultCode: `#include <iostream>

int main() {
    std::cout << "Hello, World!" << std::endl;
    return 0;
}
`
	},
	{
		id: 'rust',
		name: 'Rust',
		extension: 'rs',
		tier: 'native',
		monacoLanguage: 'rust',
		defaultCode: `fn main() {
    println!("Hello, World!");
}
`
	},
	{
		id: 'go',
		name: 'Go',
		extension: 'go',
		tier: 'native',
		monacoLanguage: 'go',
		defaultCode: `package main

import "fmt"

func main() {
    fmt.Println("Hello, World!")
}
`
	},
	{
		id: 'zig',
		name: 'Zig',
		extension: 'zig',
		tier: 'native',
		monacoLanguage: 'zig',
		defaultCode: `const std = @import("std");

pub fn main() void {
    std.debug.print("Hello, World!\\n", .{});
}
`
	},
	{
		id: 'asm',
		name: 'Assembly',
		extension: 'S',
		tier: 'native',
		monacoLanguage: 'asm',
		defaultCode: `.global _start
.text
_start:
    mov $1, %rax        # sys_write
    mov $1, %rdi        # stdout
    lea msg(%rip), %rsi # message
    mov $14, %rdx       # length
    syscall
    mov $60, %rax       # sys_exit
    xor %rdi, %rdi      # exit code 0
    syscall
.data
msg: .ascii "Hello, World!\\n"
`
	},
	{
		id: 'nim',
		name: 'Nim',
		extension: 'nim',
		tier: 'native',
		monacoLanguage: 'nim',
		defaultCode: `echo "Hello, World!"
`
	},
	{
		id: 'pascal',
		name: 'Pascal',
		extension: 'pas',
		tier: 'native',
		monacoLanguage: 'pascal',
		defaultCode: `program HelloWorld;
begin
    writeln('Hello, World!');
end.
`
	},
	{
		id: 'ocaml',
		name: 'OCaml',
		extension: 'ml',
		tier: 'native',
		monacoLanguage: 'fsharp',
		defaultCode: `let () = print_endline "Hello, World!"
`
	},
	{
		id: 'swift',
		name: 'Swift',
		extension: 'swift',
		tier: 'native',
		monacoLanguage: 'swift',
		defaultCode: `print("Hello, World!")
`
	},
	{
		id: 'haskell',
		name: 'Haskell',
		extension: 'hs',
		tier: 'native',
		monacoLanguage: 'haskell',
		defaultCode: `main :: IO ()
main = putStrLn "Hello, World!"
`
	},
	{
		id: 'csharp',
		name: 'C#',
		extension: 'cs',
		tier: 'native',
		monacoLanguage: 'csharp',
		defaultCode: `Console.WriteLine("Hello, World!");
`
	},

	// Tier 2: JVM
	{
		id: 'java',
		name: 'Java',
		extension: 'java',
		tier: 'jvm',
		monacoLanguage: 'java',
		defaultCode: `public class Main {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}
`
	},
	{
		id: 'kotlin',
		name: 'Kotlin',
		extension: 'kt',
		tier: 'jvm',
		monacoLanguage: 'kotlin',
		defaultCode: `fun main() {
    println("Hello, World!")
}
`
	},
	{
		id: 'scala',
		name: 'Scala',
		extension: 'scala',
		tier: 'jvm',
		monacoLanguage: 'scala',
		defaultCode: `@main def hello(): Unit =
    println("Hello, World!")
`
	},
	{
		id: 'clojure',
		name: 'Clojure',
		extension: 'clj',
		tier: 'jvm',
		monacoLanguage: 'clojure',
		defaultCode: `(ns main)

(defn -main []
  (println "Hello, World!"))
`
	},

	// Tier 3: Scripting
	{
		id: 'python',
		name: 'Python',
		extension: 'py',
		tier: 'scripting',
		monacoLanguage: 'python',
		defaultCode: `print("Hello, World!")
`
	},
	{
		id: 'javascript',
		name: 'JavaScript',
		extension: 'js',
		tier: 'scripting',
		monacoLanguage: 'javascript',
		defaultCode: `console.log("Hello, World!");
`
	},
	{
		id: 'typescript',
		name: 'TypeScript',
		extension: 'ts',
		tier: 'scripting',
		monacoLanguage: 'typescript',
		defaultCode: `console.log("Hello, World!");
`
	},
	{
		id: 'bun',
		name: 'Bun',
		extension: 'ts',
		tier: 'scripting',
		monacoLanguage: 'typescript',
		defaultCode: `console.log("Hello, World!");
`
	},
	{
		id: 'deno',
		name: 'Deno',
		extension: 'ts',
		tier: 'scripting',
		monacoLanguage: 'typescript',
		defaultCode: `console.log("Hello, World!");
`
	},
	{
		id: 'node',
		name: 'Node.js',
		extension: 'js',
		tier: 'scripting',
		monacoLanguage: 'javascript',
		defaultCode: `console.log("Hello, World!");
`
	},
	{
		id: 'lua',
		name: 'Lua',
		extension: 'lua',
		tier: 'scripting',
		monacoLanguage: 'lua',
		defaultCode: `print("Hello, World!")
`
	},
	{
		id: 'perl',
		name: 'Perl',
		extension: 'pl',
		tier: 'scripting',
		monacoLanguage: 'perl',
		defaultCode: `print "Hello, World!\\n";
`
	},
	{
		id: 'php',
		name: 'PHP',
		extension: 'php',
		tier: 'scripting',
		monacoLanguage: 'php',
		defaultCode: `<?php
echo "Hello, World!\\n";
`
	},
	{
		id: 'tcl',
		name: 'Tcl',
		extension: 'tcl',
		tier: 'scripting',
		monacoLanguage: 'tcl',
		defaultCode: `puts "Hello, World!"
`
	},

	// Tier 4: Special
	{
		id: 'erlang',
		name: 'Erlang',
		extension: 'erl',
		tier: 'special',
		monacoLanguage: 'erlang',
		defaultCode: `-module(main).
-export([main/0]).

main() ->
    io:format("Hello, World!~n").
`
	},
	{
		id: 'elixir',
		name: 'Elixir',
		extension: 'ex',
		tier: 'special',
		monacoLanguage: 'elixir',
		defaultCode: `defmodule Main do
  def main do
    IO.puts "Hello, World!"
  end
end
`
	},
	{
		id: 'racket',
		name: 'Racket',
		extension: 'rkt',
		tier: 'special',
		monacoLanguage: 'scheme',
		defaultCode: `#lang racket
(displayln "Hello, World!")
`
	},
	{
		id: 'wasm',
		name: 'WebAssembly',
		extension: 'wat',
		tier: 'special',
		monacoLanguage: 'wat',
		defaultCode: `(module
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory 1)
  (export "memory" (memory 0))
  (data (i32.const 8) "Hello, World!\\n")
  (func $main (export "_start")
    (i32.store (i32.const 0) (i32.const 8))
    (i32.store (i32.const 4) (i32.const 14))
    (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 100)))
  )
)
`
	}
];

export const languageById = new Map(languages.map((l) => [l.id, l]));

// Flag definitions per language
export interface FlagDefinition {
	name: string;
	label: string;
	description: string;
	type: 'select' | 'boolean' | 'text';
	options?: { value: string; label: string }[];
	default: string;
}

export const languageFlags: Record<string, FlagDefinition[]> = {
	c: [
		{
			name: 'compiler',
			label: 'Compiler',
			description: 'Compiler to use',
			type: 'select',
			options: [
				{ value: 'gcc', label: 'GCC (musl)' },
				{ value: 'clang', label: 'Clang' }
			],
			default: 'gcc'
		},
		{
			name: 'std',
			label: 'Standard',
			description: 'C standard version',
			type: 'select',
			options: [
				{ value: 'c89', label: 'C89' },
				{ value: 'c99', label: 'C99' },
				{ value: 'c11', label: 'C11' },
				{ value: 'c17', label: 'C17' },
				{ value: 'c23', label: 'C23' }
			],
			default: 'c17'
		},
		{
			name: 'warnings',
			label: 'Warnings',
			description: 'Warning level',
			type: 'select',
			options: [
				{ value: '', label: 'None' },
				{ value: 'all', label: '-Wall' },
				{ value: 'extra', label: '-Wall -Wextra' },
				{ value: 'pedantic', label: '-Wall -Wextra -Wpedantic' }
			],
			default: ''
		},
		{
			name: 'lto',
			label: 'LTO',
			description: 'Link-time optimization',
			type: 'boolean',
			default: 'false'
		},
		{
			name: 'freestanding',
			label: 'Freestanding',
			description: 'No libc (bare metal)',
			type: 'boolean',
			default: 'false'
		}
	],
	cpp: [
		{
			name: 'compiler',
			label: 'Compiler',
			description: 'Compiler to use',
			type: 'select',
			options: [
				{ value: 'g++', label: 'G++' },
				{ value: 'clang++', label: 'Clang++' }
			],
			default: 'g++'
		},
		{
			name: 'std',
			label: 'Standard',
			description: 'C++ standard version',
			type: 'select',
			options: [
				{ value: 'c++11', label: 'C++11' },
				{ value: 'c++14', label: 'C++14' },
				{ value: 'c++17', label: 'C++17' },
				{ value: 'c++20', label: 'C++20' },
				{ value: 'c++23', label: 'C++23' }
			],
			default: 'c++20'
		},
		{
			name: 'lto',
			label: 'LTO',
			description: 'Link-time optimization',
			type: 'boolean',
			default: 'false'
		},
		{
			name: 'rtti',
			label: 'RTTI',
			description: 'Runtime type information',
			type: 'boolean',
			default: 'true'
		},
		{
			name: 'exceptions',
			label: 'Exceptions',
			description: 'C++ exceptions',
			type: 'boolean',
			default: 'true'
		}
	],
	rust: [
		{
			name: 'lto',
			label: 'LTO',
			description: 'Link-time optimization mode',
			type: 'select',
			options: [
				{ value: 'true', label: 'Full (fat)' },
				{ value: 'thin', label: 'Thin' },
				{ value: 'false', label: 'None' }
			],
			default: 'true'
		},
		{
			name: 'panic',
			label: 'Panic',
			description: 'Panic strategy',
			type: 'select',
			options: [
				{ value: 'abort', label: 'Abort' },
				{ value: 'unwind', label: 'Unwind' }
			],
			default: 'abort'
		},
		{
			name: 'opt',
			label: 'Opt Level',
			description: 'Optimization level (overrides profile)',
			type: 'select',
			options: [
				{ value: '', label: 'Default' },
				{ value: '0', label: '0 (none)' },
				{ value: '1', label: '1' },
				{ value: '2', label: '2' },
				{ value: '3', label: '3 (max)' },
				{ value: 's', label: 's (size)' },
				{ value: 'z', label: 'z (min size)' }
			],
			default: ''
		},
		{
			name: 'nostd',
			label: 'no_std',
			description: 'Build without standard library',
			type: 'boolean',
			default: 'false'
		}
	],
	go: [
		{
			name: 'strip',
			label: 'Strip',
			description: 'Strip debug symbols',
			type: 'boolean',
			default: 'true'
		},
		{
			name: 'cgo',
			label: 'CGO',
			description: 'Enable CGO',
			type: 'boolean',
			default: 'false'
		},
		{
			name: 'race',
			label: 'Race',
			description: 'Enable race detector',
			type: 'boolean',
			default: 'false'
		},
		{
			name: 'tags',
			label: 'Build Tags',
			description: 'Build tags (comma-separated)',
			type: 'text',
			default: ''
		}
	],
	zig: [
		{
			name: 'opt',
			label: 'Opt Level',
			description: 'Optimization mode',
			type: 'select',
			options: [
				{ value: '', label: 'Default' },
				{ value: 'Debug', label: 'Debug' },
				{ value: 'ReleaseSafe', label: 'ReleaseSafe' },
				{ value: 'ReleaseFast', label: 'ReleaseFast' },
				{ value: 'ReleaseSmall', label: 'ReleaseSmall' }
			],
			default: ''
		},
		{
			name: 'strip',
			label: 'Strip',
			description: 'Strip debug info',
			type: 'boolean',
			default: 'true'
		}
	]
};

// Editor state
export const currentLanguage = writable<Language>('c');
export const currentOptimization = writable<Optimization>('release');
export const sourceCode = writable<string>(languages[0].defaultCode);
export const stdin = writable<string>('');
export const instructionLimit = writable<number>(10_000_000);
export const compilerFlags = writable<Record<string, string>>({});

// Derived store for current language info
export const currentLanguageInfo = derived(currentLanguage, ($lang) => languageById.get($lang)!);

// Update source code when language changes (to default template)
export function setLanguage(lang: Language) {
	const info = languageById.get(lang);
	if (info) {
		currentLanguage.set(lang);
		sourceCode.set(info.defaultCode);
		// Reset flags to defaults for the new language
		const flags = languageFlags[lang] || [];
		const defaultFlags: Record<string, string> = {};
		for (const flag of flags) {
			if (flag.default && flag.default !== '') {
				defaultFlags[flag.name] = flag.default;
			}
		}
		compilerFlags.set(defaultFlags);
	}
}

// Get current flags, filtering out empty/default values
export function getActiveFlags(): Record<string, string> {
	let flags: Record<string, string> = {};
	compilerFlags.subscribe((f) => (flags = f))();
	// Filter out empty values
	return Object.fromEntries(Object.entries(flags).filter(([_, v]) => v !== '' && v !== undefined));
}
