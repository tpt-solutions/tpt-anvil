// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package com.tptsolutions.anvil.util

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class CommandParsingTest {
    @Test
    fun parsesKnownCommands() {
        assertEquals("/generate" to "a function", CommandParsing.parse("/generate a function"))
        assertEquals("/test" to "", CommandParsing.parse("/test"))
        assertEquals("/fix" to "the bug", CommandParsing.parse("  /fix  the bug "))
    }

    @Test
    fun treatsUnknownAsChat() {
        assertEquals("/chat" to "what does this do?", CommandParsing.parse("what does this do?"))
    }

    @Test
    fun doesNotMatchPrefixWithoutBoundary() {
        assertEquals("/chat" to "/tester", CommandParsing.parse("/tester"))
    }

    @Test
    fun isSlashCommandDetection() {
        assertTrue(CommandParsing.isSlashCommand("/explain this"))
        assertFalse(CommandParsing.isSlashCommand("hello there"))
    }

    @Test
    fun extractsFencedCodeBlock() {
        val text = "Here:\n```kotlin\nval x = 1\n```\nDone."
        assertEquals("val x = 1", CommandParsing.extractCodeBlock(text))
    }

    @Test
    fun extractsUnlabeledCodeBlock() {
        assertEquals("plain", CommandParsing.extractCodeBlock("```\nplain\n```"))
    }

    @Test
    fun returnsNullWhenNoCodeBlock() {
        assertNull(CommandParsing.extractCodeBlock("no code here"))
    }

    @Test
    fun extractsMultilineCodeBlock() {
        val text = """
            |Here is the solution:
            |```rust
            |fn main() {
            |    println!("hello");
            |}
            |```
            |Let me know if you have questions.
        """.trimMargin()
        val expected = "fn main() {\n    println!(\"hello\");\n}"
        assertEquals(expected, CommandParsing.extractCodeBlock(text))
    }

    @Test
    fun extractCodeBlockHandlesEmptyInput() {
        assertNull(CommandParsing.extractCodeBlock(""))
    }

    @Test
    fun extractCodeBlockHandlesVeryLongInput() {
        val longCode = "x".repeat(50_000)
        val text = "start\n```\n$longCode\n```\nend"
        assertEquals(longCode, CommandParsing.extractCodeBlock(text))
    }

    @Test
    fun extractCodeBlockHandlesNestedFences() {
        val text = "doc\n```\nouter\n```inside\nmore\n```\nend"
        // Should extract from the first fence to the second fence
        val result = CommandParsing.extractCodeBlock(text)
        assertTrue(result != null, "should extract from nested fences")
        assertTrue(result!!.contains("outer"), "should contain outer content")
    }

    @Test
    fun parseHandlesEmptyInput() {
        assertEquals("/chat" to "", CommandParsing.parse(""))
    }

    @Test
    fun parseHandlesWhitespaceOnlyInput() {
        assertEquals("/chat" to "", CommandParsing.parse("   "))
    }

    @Test
    fun isSlashCommandRejectsEmptyString() {
        assertFalse(CommandParsing.isSlashCommand(""))
    }

    @Test
    fun isSlashCommandRejectsWhitespaceOnly() {
        assertFalse(CommandParsing.isSlashCommand("   "))
    }
}
