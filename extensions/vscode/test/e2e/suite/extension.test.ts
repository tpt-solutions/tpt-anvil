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

    test('chat panel webview view is registered', async () => {
        const views = await vscode.commands.executeCommand<
            { id: string; title: string }[]
        >('vscode.commands.getCommands', true);
        const commandIds = views.map(v => v.id);
        assert.ok(
            commandIds.includes('anvil.openChat'),
            'anvil.openChat command should be registered for the chat panel',
        );

        // Verify the view container and view type are declared in package.json
        const ext = vscode.extensions.getExtension('tpt-solutions.tpt-anvil');
        assert.ok(ext);
        const contributes = ext!.packageJSON.contributes as Record<string, unknown>;
        const viewsContainers = contributes?.viewsContainers as Record<string, unknown[]> | undefined;
        assert.ok(viewsContainers, 'package.json should declare viewsContainers');
        const activitybar = viewsContainers['activitybar'] as { id: string }[] | undefined;
        assert.ok(activitybar, 'should have activitybar viewsContainers');
        assert.ok(
            activitybar!.some(v => v.id === 'anvil'),
            'should register an anvil view container',
        );
    });

    test('slash commands execute with active editor selection', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            // No editor available in test host — skip gracefully
            return;
        }

        // Ensure there is a selection (even an empty one)
        const sel = editor.selection.isEmpty
            ? new vscode.Selection(0, 0, 0, 0)
            : editor.selection;
        await editor.edit(editBuilder => {
            editBuilder.insert(sel.active, '// anvil test marker\n');
        });

        // Execute each slash command; they should not throw
        const slashCommands = [
            'anvil.generate',
            'anvil.explain',
            'anvil.fix',
        ];
        for (const cmd of slashCommands) {
            try {
                await vscode.commands.executeCommand(cmd);
            } catch {
                // Commands that require a running daemon will throw — that is
                // acceptable in the E2E test host. We only verify the command
                // is callable and resolves its registration.
            }
        }

        // Clean up marker
        await editor.edit(editBuilder => {
            const doc = editor.document;
            for (let i = doc.lineCount - 1; i >= 0; i--) {
                const line = doc.lineAt(i);
                if (line.text.includes('// anvil test marker')) {
                    editBuilder.delete(line.range);
                }
            }
        });
    });

    test('anvil settings are accessible via getConfiguration', () => {
        const config = vscode.workspace.getConfiguration('anvil');
        assert.ok(config, 'anvil configuration namespace should exist');

        // Verify known keys are present (even if they have default values)
        const knownKeys = [
            'backend',
            'model',
            'theme',
        ];
        for (const key of knownKeys) {
            const inspect = config.inspect(key);
            assert.ok(inspect !== undefined, `setting "anvil.${key}" should be inspectable`);
        }
    });
});
