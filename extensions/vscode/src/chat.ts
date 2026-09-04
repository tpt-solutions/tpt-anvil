// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

import * as vscode from 'vscode';
import { DaemonClient, VerificationResult, buildContext } from './daemon';
import { applyDiff } from './diff';

function detectDiff(text: string): string | null {
    const diffMatch = text.match(/```diff\n([\s\S]*?)```/);
    if (diffMatch) return diffMatch[1];
    if (text.includes('\n@@ ') || text.startsWith('@@ ')) return text;
    return null;
}

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

        let fullResponse = '';
        let filePath = ctx.file_path;
        let verification: VerificationResult | undefined;

        try {
            const result = await this.daemon.slashCommand(input, ctx, convId, (chunk) => {
                if (chunk.delta) fullResponse += chunk.delta;
                this.panel?.webview.postMessage({ type: 'token', delta: chunk.delta, done: chunk.done });
            });
            verification = result.verification;

            if (verification && !verification.passed) {
                this.panel?.webview.postMessage({
                    type: 'verification_warning',
                    errors: verification.errors,
                    compiler_output: verification.compiler_output,
                    lint_output: verification.lint_output,
                    test_output: verification.test_output,
                    retries_used: verification.retries_used,
                    max_retries: verification.max_retries,
                    retried: verification.retried,
                });
            } else if (verification && verification.passed && verification.retried) {
                this.panel?.webview.postMessage({
                    type: 'verification_passed_after_retry',
                    retries_used: verification.retries_used,
                    max_retries: verification.max_retries,
                });
            }

            const diff = detectDiff(fullResponse);
            if (diff && filePath) {
                const choice = verification && !verification.passed
                    ? await vscode.window.showWarningMessage(
                        `Anvil generated a code change for ${filePath}, but verification failed:\n${verification.errors[0] ?? 'unknown error'}`,
                        'Apply Anyway', 'Preview', 'Dismiss',
                    )
                    : await vscode.window.showInformationMessage(
                        `Anvil generated a code change for ${filePath}. Review the diff and choose:`,
                        'Apply', 'Preview', 'Dismiss',
                    );
                if (choice === 'Apply' || choice === 'Apply Anyway') {
                    await applyDiff(filePath, diff);
                } else if (choice === 'Preview') {
                    const doc = await vscode.workspace.openTextDocument({
                        content: diff,
                        language: 'diff',
                    });
                    await vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside);
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
  .verification-warning { background: var(--vscode-inputValidation-warningBackground); color: var(--vscode-inputValidation-warningForeground); border-left: 3px solid var(--vscode-editorWarning-foreground); padding: 8px 12px; border-radius: 4px; font-size: 0.9em; }
  .verification-warning b { display: block; margin-bottom: 4px; }
  .verification-warning code { font-size: 0.85em; display: block; margin-top: 4px; white-space: pre-wrap; max-height: 120px; overflow-y: auto; }
  code, pre { font-family: var(--vscode-editor-font-family); background: var(--vscode-textCodeBlock-background); padding: 2px 4px; border-radius: 3px; }
  pre { padding: 8px; overflow-x: auto; }
  #input-row { display: flex; gap: 8px; padding: 8px; border-top: 1px solid var(--vscode-panel-border); position: relative; }
  #input { flex: 1; background: var(--vscode-input-background); color: var(--vscode-input-foreground); border: 1px solid var(--vscode-input-border); border-radius: 4px; padding: 6px 10px; font-size: inherit; font-family: inherit; resize: none; min-height: 36px; max-height: 120px; }
  #input::placeholder { color: var(--vscode-input-placeholderForeground); }
  #send { background: var(--vscode-button-background); color: var(--vscode-button-foreground); border: none; border-radius: 4px; padding: 6px 14px; cursor: pointer; font-size: inherit; }
  #send:hover { background: var(--vscode-button-hoverBackground); }
  .commands { padding: 4px 8px; display: flex; gap: 6px; flex-wrap: wrap; }
  .cmd-btn { background: var(--vscode-badge-background); color: var(--vscode-badge-foreground); border: none; border-radius: 12px; padding: 2px 10px; cursor: pointer; font-size: 0.85em; }
  .cmd-btn:hover { opacity: 0.85; }
  #slash-dropdown { display: none; position: absolute; bottom: 100%; left: 8px; right: 8px; background: var(--vscode-dropdown-background); border: 1px solid var(--vscode-dropdown-border); border-radius: 4px; max-height: 200px; overflow-y: auto; z-index: 10; }
  #slash-dropdown .slash-item { padding: 6px 10px; cursor: pointer; font-size: 0.9em; color: var(--vscode-dropdown-foreground); }
  #slash-dropdown .slash-item.selected { background: var(--vscode-list-activeSelectionBackground); color: var(--vscode-list-activeSelectionForeground); }
  #slash-dropdown .slash-item:hover { background: var(--vscode-list-hoverBackground); }
  #slash-dropdown .slash-desc { color: var(--vscode-descriptionForeground); font-size: 0.85em; margin-left: 8px; }
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
  <div id="slash-dropdown"></div>
  <textarea id="input" placeholder="Ask Anvil... or type /generate, /test, /explain, /fix, /docs" rows="1"></textarea>
  <button id="send">Send</button>
</div>
<script>
  const vscode = acquireVsCodeApi();
  const messagesEl = document.getElementById('messages');
  const inputEl = document.getElementById('input');
  const slashDropdown = document.getElementById('slash-dropdown');
  let currentAssistant = null;
  let selectedIdx = -1;

  const SLASH_COMMANDS = [
    { name: '/generate', desc: 'Generate code' },
    { name: '/test', desc: 'Generate tests' },
    { name: '/explain', desc: 'Explain selection' },
    { name: '/fix', desc: 'Fix selection' },
    { name: '/docs', desc: 'Generate docs' },
  ];

  function renderMarkdown(text) {
    let html = text;
    html = html.replace(/\`\`\`(\w*)\\n([\\s\\S]*?)\`\`\`/g, function(m, lang, code) {
      return '<pre><code class="language-' + lang + '">' + escapeHtml(code) + '</code></pre>';
    });
    html = html.replace(/\*\*(.+?)\*\*/g, '<b>$1</b>');
    html = html.replace(/\\*(.+?)\\*/g, '<i>$1</i>');
    html = html.replace(/\`([^\`]+)\`/g, '<code>$1</code>');
    html = html.replace(/([^\\])\\n/g, '$1<br>');
    return html;
  }

  function escapeHtml(s) {
    return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
  }

  function addMsg(cls, text) {
    const el = document.createElement('div');
    el.className = 'msg ' + cls;
    if (cls === 'assistant') {
      el.innerHTML = renderMarkdown(text);
    } else {
      el.textContent = text;
    }
    messagesEl.appendChild(el);
    messagesEl.scrollTop = messagesEl.scrollHeight;
    return el;
  }

  function showSlashDropdown(filter) {
    const matches = SLASH_COMMANDS.filter(c => c.name.startsWith(filter));
    if (matches.length === 0) { hideSlashDropdown(); return; }
    slashDropdown.innerHTML = '';
    selectedIdx = -1;
    matches.forEach(function(cmd, i) {
      const item = document.createElement('div');
      item.className = 'slash-item';
      item.innerHTML = '<span>' + cmd.name + '</span><span class="slash-desc">' + cmd.desc + '</span>';
      item.addEventListener('mousedown', function(e) {
        e.preventDefault();
        inputEl.value = cmd.name + ' ';
        hideSlashDropdown();
        inputEl.focus();
      });
      slashDropdown.appendChild(item);
    });
    slashDropdown.style.display = 'block';
  }

  function hideSlashDropdown() {
    slashDropdown.style.display = 'none';
    selectedIdx = -1;
  }

  function navigateDropdown(dir) {
    const items = slashDropdown.querySelectorAll('.slash-item');
    if (items.length === 0) return;
    if (selectedIdx >= 0) items[selectedIdx].classList.remove('selected');
    selectedIdx = (selectedIdx + dir + items.length) % items.length;
    items[selectedIdx].classList.add('selected');
    items[selectedIdx].scrollIntoView({ block: 'nearest' });
  }

  function selectDropdown() {
    const items = slashDropdown.querySelectorAll('.slash-item');
    if (selectedIdx >= 0 && items[selectedIdx]) {
      items[selectedIdx].dispatchEvent(new Event('mousedown'));
    }
  }

  inputEl.addEventListener('input', function() {
    const val = inputEl.value;
    if (val.startsWith('/')) {
      showSlashDropdown(val.split(/\\s/)[0]);
    } else {
      hideSlashDropdown();
    }
  });

  document.getElementById('send').addEventListener('click', send);
  inputEl.addEventListener('keydown', (e) => {
    if (slashDropdown.style.display === 'block') {
      if (e.key === 'ArrowDown') { e.preventDefault(); navigateDropdown(1); return; }
      if (e.key === 'ArrowUp') { e.preventDefault(); navigateDropdown(-1); return; }
      if (e.key === 'Enter' || e.key === 'Tab') { e.preventDefault(); selectDropdown(); return; }
      if (e.key === 'Escape') { hideSlashDropdown(); return; }
    }
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
    hideSlashDropdown();
    vscode.postMessage({ type: 'send', text });
  }

  window.addEventListener('message', (e) => {
    const msg = e.data;
    if (msg.type === 'user') { addMsg('user', msg.text); }
    else if (msg.type === 'assistant_start') { currentAssistant = addMsg('assistant', ''); }
    else if (msg.type === 'token') {
      if (currentAssistant) {
        if (!currentAssistant.dataset.raw) currentAssistant.dataset.raw = '';
        currentAssistant.dataset.raw += msg.delta;
        currentAssistant.innerHTML = renderMarkdown(currentAssistant.dataset.raw);
      }
      messagesEl.scrollTop = messagesEl.scrollHeight;
    }
    else if (msg.type === 'error') { addMsg('error', 'Error: ' + msg.message); currentAssistant = null; }
    else if (msg.type === 'verification_warning') {
      var el = document.createElement('div');
      el.className = 'msg verification-warning';
      var retryNote = msg.retried ? '<br><i>Anvil retried ' + msg.retries_used + ' time(s) after the initial attempt failed.</i>' : '';
      el.innerHTML = '<b>Verification failed</b>' + retryNote +
        msg.errors.map(function(e) { return escapeHtml(e); }).join('<br>') +
        (msg.compiler_output ? '<code>Compiler: ' + escapeHtml(msg.compiler_output).substring(0, 500) + '</code>' : '') +
        (msg.lint_output ? '<code>Lint: ' + escapeHtml(msg.lint_output).substring(0, 500) + '</code>' : '') +
        (msg.test_output ? '<code>Tests: ' + escapeHtml(msg.test_output).substring(0, 500) + '</code>' : '');
      messagesEl.appendChild(el);
      messagesEl.scrollTop = messagesEl.scrollHeight;
    }
    else if (msg.type === 'verification_passed_after_retry') {
      var el = document.createElement('div');
      el.className = 'msg verification-warning';
      el.style.background = 'var(--vscode-terminal-ansiGreen, #d4edda)';
      el.style.borderLeftColor = 'var(--vscode-terminal-ansiGreen, #28a745)';
      el.innerHTML = '<b>Anvil checked its own work</b> — verification failed initially but passed after ' + msg.retries_used + ' retry attempt(s).';
      messagesEl.appendChild(el);
      messagesEl.scrollTop = messagesEl.scrollHeight;
    }
    else if (msg.type === 'prefill') { inputEl.value = msg.text + ' '; inputEl.focus(); }
  });
</script>
</body>
</html>`;
    }
}
