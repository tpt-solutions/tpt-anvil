// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

import * as path from 'path';

import { runTests } from '@vscode/test-electron';

async function main(): Promise<void> {
    try {
        // The folder containing the extension package.json (development root).
        const extensionDevelopmentPath = path.resolve(__dirname, '..');
        // The compiled test suite entry (index.js).
        const extensionTestsPath = path.resolve(__dirname, './suite/index');

        await runTests({ extensionDevelopmentPath, extensionTestsPath });
    } catch (err) {
        console.error('Failed to run E2E tests:', err);
        process.exit(1);
    }
}

main();
