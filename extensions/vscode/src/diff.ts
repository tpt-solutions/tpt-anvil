// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

import * as vscode from 'vscode';

import { patchContent } from './diffCore';

export { patchContent, parseHunkOrigStart } from './diffCore';

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
