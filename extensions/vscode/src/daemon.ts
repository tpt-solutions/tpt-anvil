// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

import * as net from 'net';
import * as os from 'os';
import * as path from 'path';
import * as cp from 'child_process';
import * as vscode from 'vscode';

export interface StatusResponse {
    version: string;
    active_backend: string;
    active_model?: string;
    index_status: string;
}

export interface StreamToken {
    id: number;
    delta: string;
    done: boolean;
}

export interface VerificationResult {
    passed: boolean;
    errors: string[];
    compiler_output?: string;
    lint_output?: string;
    test_output?: string;
    retries_used: number;
    max_retries: number;
    retried: boolean;
}

type StatusChangeListener = (connected: boolean) => void;

export class DaemonClient {
    private socket: net.Socket | null = null;
    private requestId = 0;
    private pending = new Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();
    private streamListeners = new Map<number, (chunk: StreamToken) => void>();
    private statusListeners: StatusChangeListener[] = [];
    private connected = false;
    private buffer = '';

    socketPath(): string {
        const runtimeDir = process.env['XDG_RUNTIME_DIR'] ?? os.tmpdir();
        return path.join(runtimeDir, 'anvil', 'anvil.sock');
    }

    async start(): Promise<void> {
        await this.connect();
    }

    private async connect(): Promise<void> {
        return new Promise((resolve) => {
            const sock = new net.Socket();
            sock.connect(this.socketPath(), () => {
                this.socket = sock;
                this.connected = true;
                this.notifyStatus(true);
                resolve();
            });
            sock.on('error', () => {
                this.connected = false;
                this.notifyStatus(false);
                resolve(); // Don't reject — daemon may not be running yet
            });
            sock.on('data', (data) => this.onData(data.toString()));
            sock.on('close', () => {
                this.connected = false;
                this.notifyStatus(false);
                this.socket = null;
            });
        });
    }

    private onData(chunk: string) {
        this.buffer += chunk;
        const lines = this.buffer.split('\n');
        this.buffer = lines.pop() ?? '';

        for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed) continue;
            try {
                const msg = JSON.parse(trimmed);
                // Notification (streaming token)
                if (msg.method === 'stream_token' && msg.params) {
                    const token = msg.params as StreamToken;
                    this.streamListeners.get(token.id)?.(token);
                    continue;
                }
                // Response
                if (msg.id !== undefined) {
                    const pending = this.pending.get(msg.id);
                    if (pending) {
                        this.pending.delete(msg.id);
                        if (msg.error) {
                            pending.reject(new Error(msg.error.message));
                        } else {
                            pending.resolve(msg.result);
                        }
                    }
                }
            } catch {
                // ignore parse errors
            }
        }
    }

    async request<T>(method: string, params: unknown): Promise<T> {
        if (!this.socket) {
            await this.connect();
        }
        if (!this.socket) {
            throw new Error('Anvil daemon is not running. Start it with: anvil start');
        }

        const id = ++this.requestId;
        const msg = JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n';

        return new Promise((resolve, reject) => {
            this.pending.set(id, {
                resolve: (v) => resolve(v as T),
                reject,
            });
            this.socket!.write(msg);
        });
    }

    async getStatus(): Promise<StatusResponse> {
        return this.request<StatusResponse>('status', {});
    }

    async slashCommand(
        command: string,
        context: CodeContext,
        conversationId?: string,
        onToken?: (chunk: StreamToken) => void,
    ): Promise<{ content: string; verification?: VerificationResult }> {
        const id = ++this.requestId;
        const params = { command, context, conversation_id: conversationId ?? null };
        const msg = JSON.stringify({ jsonrpc: '2.0', id, method: 'slash_command', params }) + '\n';

        if (onToken) {
            this.streamListeners.set(id, onToken);
        }

        return new Promise((resolve, reject) => {
            this.pending.set(id, {
                resolve: (v) => {
                    this.streamListeners.delete(id);
                    resolve(v as { content: string; verification?: VerificationResult });
                },
                reject: (e) => {
                    this.streamListeners.delete(id);
                    reject(e);
                },
            });
            this.socket?.write(msg);
        });
    }

    onStatusChange(listener: StatusChangeListener) {
        this.statusListeners.push(listener);
    }

    private notifyStatus(connected: boolean) {
        for (const l of this.statusListeners) {
            l(connected);
        }
    }

    stop() {
        this.socket?.destroy();
        this.socket = null;
    }
}

export interface CodeContext {
    file_path: string;
    language: string;
    content: string;
    cursor_line?: number;
    selection?: { start_line: number; end_line: number; start_col: number; end_col: number };
    related_chunks: unknown[];
}

export function buildContext(editor: vscode.TextEditor): CodeContext {
    const doc = editor.document;
    const sel = editor.selection;
    const hasSelection = !sel.isEmpty;

    return {
        file_path: doc.fileName,
        language: doc.languageId,
        content: hasSelection ? doc.getText(sel) : doc.getText(),
        cursor_line: sel.active.line,
        selection: hasSelection
            ? {
                  start_line: sel.start.line,
                  end_line: sel.end.line,
                  start_col: sel.start.character,
                  end_col: sel.end.character,
              }
            : undefined,
        related_chunks: [],
    };
}
