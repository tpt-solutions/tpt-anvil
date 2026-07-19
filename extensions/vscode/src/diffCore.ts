// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

/**
 * Pure, dependency-free unified-diff utilities.
 *
 * Kept free of the `vscode` module so it can be unit tested with Vitest
 * outside the VS Code extension host. This mirrors the daemon-side Rust
 * implementation so results are consistent across editor and backend.
 */

/**
 * Apply a unified diff to `original`, returning the patched content or `null`
 * when the diff contains no applicable hunks.
 */
export function patchContent(original: string, diff: string): string | null {
    const originalLines = original.split('\n');
    const diffLines = diff.split('\n');
    const result: string[] = [];
    let origCursor = 0;
    let inHunk = false;

    for (const dline of diffLines) {
        if (dline.startsWith('---') || dline.startsWith('+++')) {
            continue;
        }
        if (dline.startsWith('@@')) {
            const start = parseHunkOrigStart(dline);
            if (start === null) {
                return null;
            }
            const target = Math.max(0, start - 1);
            while (origCursor < target && origCursor < originalLines.length) {
                result.push(originalLines[origCursor]);
                origCursor++;
            }
            inHunk = true;
            continue;
        }
        if (!inHunk) {
            continue;
        }
        const marker = dline.charAt(0);
        if (marker === ' ') {
            result.push(dline.slice(1));
            origCursor++;
        } else if (marker === '-') {
            origCursor++;
        } else if (marker === '+') {
            result.push(dline.slice(1));
        }
    }

    if (!inHunk) {
        return null;
    }

    while (origCursor < originalLines.length) {
        result.push(originalLines[origCursor]);
        origCursor++;
    }

    return result.join('\n');
}

/** Parse the original-file start line from `@@ -12,5 +12,6 @@`. */
export function parseHunkOrigStart(header: string): number | null {
    const match = header.match(/@@\s*-(\d+)/);
    if (!match) {
        return null;
    }
    const n = parseInt(match[1], 10);
    return Number.isNaN(n) ? null : n;
}

/** Parse a slash command string into `[command, rest]`. */
export function parseSlashCommand(input: string): [string, string] {
    const trimmed = input.trim();
    const known = ['/generate', '/test', '/explain', '/fix', '/docs'];
    for (const cmd of known) {
        if (trimmed === cmd || trimmed.startsWith(cmd + ' ')) {
            return [cmd, trimmed.slice(cmd.length).trim()];
        }
    }
    return ['/chat', trimmed];
}

/** Extract the first fenced code block from markdown text, if any. */
export function extractCodeBlock(text: string): string | null {
    const fenceMatch = text.match(/```[a-zA-Z0-9]*\n([\s\S]*?)```/);
    if (fenceMatch) {
        return fenceMatch[1].replace(/\n$/, '');
    }
    return null;
}
