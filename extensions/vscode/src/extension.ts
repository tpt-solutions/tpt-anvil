// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

import * as vscode from 'vscode';
import { DaemonClient } from './daemon';
import { ChatPanel } from './chat';
import { applyDiff } from './diff';

let daemon: DaemonClient | undefined;
let chatPanel: ChatPanel | undefined;

export async function activate(context: vscode.ExtensionContext) {
    daemon = new DaemonClient();

    const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    statusBar.text = '$(hubot) Anvil';
    statusBar.tooltip = 'TPT Anvil — click to open chat';
    statusBar.command = 'anvil.openChat';
    statusBar.show();
    context.subscriptions.push(statusBar);

    await daemon.start();
    updateStatusBar(statusBar, daemon);

    context.subscriptions.push(
        vscode.commands.registerCommand('anvil.openChat', () => {
            if (!daemon) return;
            if (!chatPanel) {
                chatPanel = new ChatPanel(context, daemon);
            }
            chatPanel.show();
        }),

        vscode.commands.registerCommand('anvil.generate', () => runCommand('/generate', context)),
        vscode.commands.registerCommand('anvil.test', () => runCommand('/test', context)),
        vscode.commands.registerCommand('anvil.explain', () => runCommand('/explain', context)),
        vscode.commands.registerCommand('anvil.fix', () => runCommand('/fix', context)),
        vscode.commands.registerCommand('anvil.docs', () => runCommand('/docs', context)),

        vscode.commands.registerCommand('anvil.status', async () => {
            if (!daemon) return;
            const status = await daemon.getStatus();
            vscode.window.showInformationMessage(`Anvil: ${status.active_backend} / ${status.active_model ?? 'no model'}`);
        }),
    );
}

async function runCommand(cmd: string, context: vscode.ExtensionContext) {
    if (!daemon) return;

    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('Anvil: No active editor.');
        return;
    }

    if (!chatPanel) {
        chatPanel = new ChatPanel(context, daemon);
    }
    chatPanel.show();
    chatPanel.sendCommand(cmd, editor);
}

function updateStatusBar(bar: vscode.StatusBarItem, client: DaemonClient) {
    client.onStatusChange((connected) => {
        bar.text = connected ? '$(hubot) Anvil' : '$(hubot) Anvil (offline)';
        bar.backgroundColor = connected
            ? undefined
            : new vscode.ThemeColor('statusBarItem.warningBackground');
    });
}

export function deactivate() {
    daemon?.stop();
}
