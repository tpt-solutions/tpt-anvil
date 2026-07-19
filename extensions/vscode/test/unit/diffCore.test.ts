// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

import { describe, it, expect } from 'vitest';

import {
    patchContent,
    parseHunkOrigStart,
    parseSlashCommand,
    extractCodeBlock,
} from '../../src/diffCore';

describe('parseHunkOrigStart', () => {
    it('parses the original start line', () => {
        expect(parseHunkOrigStart('@@ -12,5 +12,6 @@')).toBe(12);
        expect(parseHunkOrigStart('@@ -1 +1 @@')).toBe(1);
    });

    it('returns null for malformed headers', () => {
        expect(parseHunkOrigStart('not a hunk')).toBeNull();
    });
});

describe('patchContent', () => {
    it('applies a simple replacement', () => {
        const original = 'line1\nline2\nline3';
        const diff = '--- a/f\n+++ b/f\n@@ -1,3 +1,3 @@\n line1\n-line2\n+CHANGED\n line3';
        expect(patchContent(original, diff)).toBe('line1\nCHANGED\nline3');
    });

    it('applies additions', () => {
        const original = 'a\nb';
        const diff = '@@ -1,2 +1,3 @@\n a\n b\n+c';
        expect(patchContent(original, diff)).toBe('a\nb\nc');
    });

    it('returns null when there are no hunks', () => {
        expect(patchContent('anything', '--- a/f\n+++ b/f\n')).toBeNull();
    });

    it('preserves trailing lines after the last hunk', () => {
        const original = 'a\nb\nc\nd';
        const diff = '@@ -1,1 +1,1 @@\n-a\n+A';
        expect(patchContent(original, diff)).toBe('A\nb\nc\nd');
    });
});

describe('parseSlashCommand', () => {
    it('splits known commands', () => {
        expect(parseSlashCommand('/generate a function')).toEqual(['/generate', 'a function']);
        expect(parseSlashCommand('/test')).toEqual(['/test', '']);
        expect(parseSlashCommand('  /fix  the bug ')).toEqual(['/fix', 'the bug']);
    });

    it('treats unknown input as chat', () => {
        expect(parseSlashCommand('what does this do?')).toEqual(['/chat', 'what does this do?']);
    });

    it('does not match a prefix without a boundary', () => {
        // "/tester" is not "/test".
        expect(parseSlashCommand('/tester')).toEqual(['/chat', '/tester']);
    });
});

describe('extractCodeBlock', () => {
    it('extracts a labeled fenced block', () => {
        const text = 'Here:\n```ts\nconst x = 1;\n```\nDone.';
        expect(extractCodeBlock(text)).toBe('const x = 1;');
    });

    it('extracts an unlabeled block', () => {
        const text = '```\nplain\n```';
        expect(extractCodeBlock(text)).toBe('plain');
    });

    it('returns null when no block present', () => {
        expect(extractCodeBlock('no code here')).toBeNull();
    });
});
