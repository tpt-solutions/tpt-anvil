// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package com.tptsolutions.anvil.util

/**
 * Pure, framework-free helpers shared by the plugin. Kept out of any
 * IntelliJ-platform dependency so they can be unit tested with plain JUnit.
 */
object CommandParsing {
    private val KNOWN_COMMANDS = listOf("/generate", "/test", "/explain", "/fix", "/docs")

    /**
     * Parse a chat input into a [command, argument] pair. Unknown input is
     * treated as a free-form chat message under the "/chat" command.
     */
    fun parse(input: String): Pair<String, String> {
        val trimmed = input.trim()
        for (cmd in KNOWN_COMMANDS) {
            if (trimmed == cmd || trimmed.startsWith("$cmd ")) {
                return cmd to trimmed.removePrefix(cmd).trim()
            }
        }
        return "/chat" to trimmed
    }

    /** Whether [input] begins with a recognized slash command. */
    fun isSlashCommand(input: String): Boolean = parse(input).first != "/chat"

    /** Extract the first fenced code block from markdown, or null. */
    fun extractCodeBlock(text: String): String? {
        val regex = Regex("```[a-zA-Z0-9]*\\n([\\s\\S]*?)```")
        val match = regex.find(text) ?: return null
        return match.groupValues[1].trimEnd('\n')
    }
}
