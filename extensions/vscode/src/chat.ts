// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

import * as vscode from 'vscode';
import { DaemonClient, buildContext } from './daemon';
import { applyDiff } from './diff';

export class ChatPanel {
    private panel: vscode.WebviewPanel | undefined;

    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly daemon: DaemonClient,
    ) {}

    show() {
        if (this.panel) {
            this.panel.reveal();
            return;
        }
        this.panel = vscode.window.createWebviewPanel(
            'anvilChat',
            'Anvil Chat',
            vscode.ViewColumn.Beside,
            { enableScripts: true, retainContextWhenHidden: true },
        );
        this.panel.webview.html = this.buildHtml();
        this.panel.onDidDispose(() => { this.panel = undefined; });
        this.panel.webview.onDidReceiveMessage((msg) => this.handleMessage(msg));
    }

    sendCommand(cmd: string, editor: vscode.TextEditor) {
        this.show();
        const ctx = buildContext(editor);
        this.panel?.webview.postMessage({ type: 'prefill', text: cmd });
        this.runCommand(cmd, ctx);
    }

    private async handleMessage(msg: { type: string; text?: string }) {
        if (msg.type === 'send' && msg.text) {
            const editor = vscode.window.activeTextEditor;
            const ctx = editor ? buildContext(editor) : { file_path: '', language: '', content: '', related_chunks: [] };
            await this.runCommand(msg.text, ctx as any);
        }
    }

    private async runCommand(input: string, ctx: any) {
        const convId = 'main';
        this.panel?.webview.postMessage({ type: 'user', text: input });
        this.panel?.webview.postMessage({ type: 'assistant_start' });

        let diff: string | null = null;
        let filePath = ctx.file_path;

        try {
            await this.daemon.slashCommand(input, ctx, convId, (chunk) => {
                this.panel?.webview.postMessage({ type: 'token', delta: chunk.delta, done: chunk.done });
                if (chunk.done && chunk.delta?.startsWith('---')) {
                    diff = chunk.delta;
                }
            });

            if (diff && filePath) {
                const apply = await vscode.window.showInformationMessage(
                    'Anvil generated a code change. Apply it?',
                    'Apply', 'Dismiss',
                );
                if (apply === 'Apply') {
                    await applyDiff(filePath, diff);
                }
            }
        } catch (err: any) {
            this.panel?.webview.postMessage({ type: 'error', message: err.message });
        }
    }

    private buildHtml(): string {
        return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Anvil Chat</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { font-family: var(--vscode-font-family); font-size: var(--vscode-font-size); background: var(--vscode-editor-background); color: var(--vscode-editor-foreground); display: flex; flex-direction: column; height: 100vh; }
  #messages { flex: 1; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 12px; }
  .msg { padding: 8px 12px; border-radius: 6px; max-width: 90%; white-space: pre-wrap; word-break: break-word; }
  .user { background: var(--vscode-button-background); color: var(--vscode-button-foreground); align-self: flex-end; }
  .assistant { background: var(--vscode-editorWidget-background); align-self: flex-start; }
  .error { background: var(--vscode-inputValidation-errorBackground); color: var(--vscode-inputValidation-errorForeground); }
  code, pre { font-family: var(--vscode-editor-font-family); background: var(--vscode-textCodeBlock-background); padding: 2px 4px; border-radius: 3px; }
  pre { padding: 8px; overflow-x: auto; }
  #input-row { display: flex; gap: 8px; padding: 8px; border-top: 1px solid var(--vscode-panel-border); }
  #input { flex: 1; background: var(--vscode-input-background); color: var(--vscode-input-foreground); border: 1px solid var(--vscode-input-border); border-radius: 4px; padding: 6px 10px; font-size: inherit; font-family: inherit; resize: none; min-height: 36px; max-height: 120px; }
  #input::placeholder { color: var(--vscode-input-placeholderForeground); }
  #send { background: var(--vscode-button-background); color: var(--vscode-button-foreground); border: none; border-radius: 4px; padding: 6px 14px; cursor: pointer; font-size: inherit; }
  #send:hover { background: var(--vscode-button-hoverBackground); }
  .commands { padding: 4px 8px; display: flex; gap: 6px; flex-wrap: wrap; }
  .cmd-btn { background: var(--vscode-badge-background); color: var(--vscode-badge-foreground); border: none; border-radius: 12px; padding: 2px 10px; cursor: pointer; font-size: 0.85em; }
  .cmd-btn:hover { opacity: 0.85; }
</style>
</head>
<body>
<div id="messages"></div>
<div class="commands">
  <button class="cmd-btn" data-cmd="/generate">/generate</button>
  <button class="cmd-btn" data-cmd="/test">/test</button>
  <button class="cmd-btn" data-cmd="/explain">/explain</button>
  <button class="cmd-btn" data-cmd="/fix">/fix</button>
  <button class="cmd-btn" data-cmd="/docs">/docs</button>
</div>
<div id="input-row">
  <textarea id="input" placeholder="Ask Anvil... or type /generate, /test, /explain, /fix, /docs" rows="1"></textarea>
  <button id="send">Send</button>
</div>
<script>
  const vscode = acquireVsCodeApi();
  const messagesEl = document.getElementById('messages');
  const inputEl = document.getElementById('input');
  let currentAssistant = null;

  function addMsg(cls, text) {
    const el = document.createElement('div');
    el.className = 'msg ' + cls;
    el.textContent = text;
    messagesEl.appendChild(el);
    messagesEl.scrollTop = messagesEl.scrollHeight;
    return el;
  }

  document.getElementById('send').addEventListener('click', send);
  inputEl.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); send(); }
  });

  document.querySelectorAll('.cmd-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      inputEl.value = btn.dataset.cmd + ' ';
      inputEl.focus();
    });
  });

  function send() {
    const text = inputEl.value.trim();
    if (!text) return;
    inputEl.value = '';
    vscode.postMessage({ type: 'send', text });
  }

  window.addEventListener('message', (e) => {
    const msg = e.data;
    if (msg.type === 'user') { addMsg('user', msg.text); }
    else if (msg.type === 'assistant_start') { currentAssistant = addMsg('assistant', ''); }
    else if (msg.type === 'token') { if (currentAssistant) currentAssistant.textContent += msg.delta; messagesEl.scrollTop = messagesEl.scrollHeight; }
    else if (msg.type === 'error') { addMsg('error', 'Error: ' + msg.message); currentAssistant = null; }
    else if (msg.type === 'prefill') { inputEl.value = msg.text + ' '; inputEl.focus(); }
  });
</script>
</body>
</html>`;
    }
}
