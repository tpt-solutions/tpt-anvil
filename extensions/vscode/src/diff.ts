// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

import * as vscode from 'vscode';
import * as fs from 'fs/promises';

export async function applyDiff(filePath: string, unifiedDiff: string): Promise<void> {
    try {
        const uri = vscode.Uri.file(filePath);
        const doc = await vscode.workspace.openTextDocument(uri);
        const original = doc.getText();

        const modified = patchContent(original, unifiedDiff);
        if (modified === null) {
            vscode.window.showErrorMessage('Anvil: Could not apply diff — patch did not match.');
            return;
        }

        const edit = new vscode.WorkspaceEdit();
        const fullRange = new vscode.Range(
            doc.positionAt(0),
            doc.positionAt(original.length),
        );
        edit.replace(uri, fullRange, modified);
        await vscode.workspace.applyEdit(edit);
        vscode.window.showInformationMessage('Anvil: Change applied.');
    } catch (err: any) {
        vscode.window.showErrorMessage(`Anvil: Failed to apply diff — ${err.message}`);
    }
}

function patchContent(original: string, diff: string): string | null {
    const lines = original.split('\n');
    const diffLines = diff.split('\n');
    const result = [...lines];
    let offset = 0;

    for (const dline of diffLines) {
        if (dline.startsWith('---') || dline.startsWith('+++') || dline.startsWith('@@')) continue;
        if (dline.startsWith('+')) {
            // insertion — for simplified apply: just return the code block if present
        }
    }

    // Simplified: extract the new code from the diff's + lines as a replacement
    const plusLines = diffLines.filter(l => l.startsWith('+') && !l.startsWith('+++')).map(l => l.slice(1));
    if (plusLines.length > 0) {
        return plusLines.join('\n');
    }
    return null;
}
