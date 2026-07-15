// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package com.tptsolutions.anvil

import com.google.gson.Gson
import com.google.gson.JsonObject
import kotlinx.coroutines.*
import java.io.*
import java.net.StandardProtocolFamily
import java.net.UnixDomainSocketAddress
import java.nio.channels.SocketChannel
import java.nio.file.Path
import java.util.concurrent.atomic.AtomicLong

data class StatusResponse(
    val version: String,
    val active_backend: String,
    val active_model: String?,
    val index_status: String,
)

data class SlashCommandResult(val content: String)

data class StreamChunk(val id: Long, val delta: String, val done: Boolean)

class DaemonClient {
    private val gson = Gson()
    private val idCounter = AtomicLong(0)
    private var writer: PrintWriter? = null
    private var reader: BufferedReader? = null
    private val pending = mutableMapOf<Long, CompletableDeferred<Any?>>()
    private val streamListeners = mutableMapOf<Long, (StreamChunk) -> Unit>()

    fun socketPath(): Path {
        val runtime = System.getenv("XDG_RUNTIME_DIR") ?: System.getProperty("java.io.tmpdir")
        return Path.of(runtime, "anvil", "anvil.sock")
    }

    fun connect() {
        val addr = UnixDomainSocketAddress.of(socketPath())
        val channel = SocketChannel.open(StandardProtocolFamily.UNIX)
        channel.connect(addr)
        val stream = channel.socket()
        writer = PrintWriter(OutputStreamWriter(stream.getOutputStream(), Charsets.UTF_8), true)
        reader = BufferedReader(InputStreamReader(stream.getInputStream(), Charsets.UTF_8))

        CoroutineScope(Dispatchers.IO).launch { readLoop() }
    }

    private suspend fun readLoop() {
        while (true) {
            val line = withContext(Dispatchers.IO) { reader?.readLine() } ?: break
            val json = gson.fromJson(line, JsonObject::class.java) ?: continue

            if (json.has("method") && json.get("method").asString == "stream_token") {
                val params = json.getAsJsonObject("params")
                val chunk = StreamChunk(
                    id = params.get("id").asLong,
                    delta = params.get("delta").asString,
                    done = params.get("done").asBoolean,
                )
                streamListeners[chunk.id]?.invoke(chunk)
                continue
            }

            val id = json.get("id")?.asLong ?: continue
            val deferred = pending.remove(id) ?: continue
            if (json.has("error")) {
                deferred.completeExceptionally(RuntimeException(json.getAsJsonObject("error").get("message").asString))
            } else {
                deferred.complete(json.get("result"))
            }
        }
    }

    suspend fun request(method: String, params: Any): Any? {
        val id = idCounter.incrementAndGet()
        val payload = mapOf("jsonrpc" to "2.0", "id" to id, "method" to method, "params" to params)
        val deferred = CompletableDeferred<Any?>()
        pending[id] = deferred
        withContext(Dispatchers.IO) { writer?.println(gson.toJson(payload)) }
        return deferred.await()
    }

    suspend fun getStatus(): StatusResponse {
        val result = request("status", emptyMap<String, Any>())
        return gson.fromJson(gson.toJson(result), StatusResponse::class.java)
    }

    suspend fun slashCommand(
        command: String,
        context: CodeContext,
        conversationId: String? = null,
        onToken: ((StreamChunk) -> Unit)? = null,
    ): SlashCommandResult {
        val id = idCounter.incrementAndGet()
        val params = mapOf(
            "command" to command,
            "context" to context,
            "conversation_id" to conversationId,
        )
        val payload = mapOf("jsonrpc" to "2.0", "id" to id, "method" to "slash_command", "params" to params)
        val deferred = CompletableDeferred<Any?>()
        pending[id] = deferred
        if (onToken != null) streamListeners[id] = onToken
        withContext(Dispatchers.IO) { writer?.println(gson.toJson(payload)) }
        val result = deferred.await()
        streamListeners.remove(id)
        val obj = gson.fromJson(gson.toJson(result), JsonObject::class.java)
        return SlashCommandResult(obj.get("content").asString)
    }
}

data class CodeContext(
    val file_path: String,
    val language: String,
    val content: String,
    val cursor_line: Int? = null,
    val selection: TextSelection? = null,
    val related_chunks: List<Any> = emptyList(),
)

data class TextSelection(
    val start_line: Int,
    val end_line: Int,
    val start_col: Int,
    val end_col: Int,
)
