// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

import * as assert from 'assert';

import * as vscode from 'vscode';

suite('TPT Anvil Extension E2E', () => {
    test('extension is present', () => {
        const ext = vscode.extensions.getExtension('tpt-solutions.tpt-anvil');
        assert.ok(ext, 'extension should be discoverable by id');
    });

    test('extension activates', async () => {
        const ext = vscode.extensions.getExtension('tpt-solutions.tpt-anvil');
        assert.ok(ext);
        await ext!.activate();
        assert.strictEqual(ext!.isActive, true);
    });

    test('registers Anvil commands', async () => {
        const commands = await vscode.commands.getCommands(true);
        const expected = [
            'anvil.openChat',
            'anvil.generate',
            'anvil.test',
            'anvil.explain',
            'anvil.fix',
            'anvil.docs',
            'anvil.status',
        ];
        for (const cmd of expected) {
            assert.ok(commands.includes(cmd), `missing command: ${cmd}`);
        }
    });
});
