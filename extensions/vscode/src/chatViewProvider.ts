// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

import * as vscode from 'vscode';
import { DaemonClient, buildContext } from './daemon';

export class AnvilChatViewProvider implements vscode.WebviewViewProvider {
    public static readonly viewType = 'anvil.chatPanel';
    private view?: vscode.WebviewView;

    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly daemon: DaemonClient,
    ) {}

    resolveWebviewView(
        webviewView: vscode.WebviewView,
        _context: vscode.WebviewViewResolveContext,
        _token: vscode.CancellationToken,
    ) {
        this.view = webviewView;
        webviewView.webview.options = { enableScripts: true };
        webviewView.webview.html = this.buildHtml();
        webviewView.webview.onDidReceiveMessage((msg) => this.handleMessage(msg));
        webviewView.onDidDispose(() => { this.view = undefined; });
    }

    private async handleMessage(msg: { type: string; text?: string }) {
        if (msg.type === 'send' && msg.text) {
            const editor = vscode.window.activeTextEditor;
            const ctx = editor
                ? buildContext(editor)
                : { file_path: '', language: '', content: '', related_chunks: [] };
            await this.runCommand(msg.text, ctx as any);
        }
    }

    private async runCommand(input: string, ctx: any) {
        const convId = 'sidebar';
        this.view?.webview.postMessage({ type: 'user', text: input });
        this.view?.webview.postMessage({ type: 'assistant_start' });

        try {
            await this.daemon.slashCommand(input, ctx, convId, (chunk) => {
                this.view?.webview.postMessage({ type: 'token', delta: chunk.delta, done: chunk.done });
            });
        } catch (err: any) {
            this.view?.webview.postMessage({ type: 'error', message: err.message });
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
</style>
</head>
<body>
<div id="messages"></div>
<div id="input-row">
  <textarea id="input" placeholder="Ask Anvil..." rows="1"></textarea>
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
