// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package com.tptsolutions.anvil.diff

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

/**
 * Tests for the unified-diff application logic in [AnvilDiffHandler].
 *
 * The `applyUnifiedDiff` and `parseHunkOrigStart` methods are private, so we
 * use Kotlin reflection to invoke them directly, keeping the tests free of any
 * IntelliJ Platform dependencies.
 */
class AnvilDiffHandlerTest {

    private fun applyDiff(original: String, diff: String): String? {
        val method = AnvilDiffHandler::class.java
            .getDeclaredMethod("applyUnifiedDiff", String::class.java, String::class.java)
        method.isAccessible = true
        return method.invoke(AnvilDiffHandler, original, diff) as? String
    }

    @Test
    fun appliesSimpleAddition() {
        val original = "line1\nline2\nline3"
        val diff = "@@ -1,3 +1,4 @@\n line1\n line2\n+line2.5\n line3"
        assertEquals("line1\nline2\nline2.5\nline3", applyDiff(original, diff))
    }

    @Test
    fun appliesSimpleDeletion() {
        val original = "line1\nline2\nline3"
        val diff = "@@ -1,3 +1,2 @@\n line1\n-line2\n line3"
        assertEquals("line1\nline3", applyDiff(original, diff))
    }

    @Test
    fun appliesSimpleReplacement() {
        val original = "hello\nworld"
        val diff = "@@ -1,2 +1,2 @@\n hello\n-world\n+universe"
        assertEquals("hello\nuniverse", applyDiff(original, diff))
    }

    @Test
    fun preservesLinesBeforeHunk() {
        val original = "a\nb\nc\nd"
        val diff = "@@ -3,2 +3,2 @@\n c\n-d\n+x"
        assertEquals("a\nb\nc\nx", applyDiff(original, diff))
    }

    @Test
    fun preservesLinesAfterHunk() {
        val original = "a\nb\nc\nd"
        val diff = "@@ -1,2 +1,2 @@\n-a\n+x\n b"
        assertEquals("x\nb\nc\nd", applyDiff(original, diff))
    }

    @Test
    fun returnsNullForNoHunks() {
        val original = "a\nb"
        val diff = "--- a/file.txt\n+++ b/file.txt"
        assertNull(applyDiff(original, diff))
    }

    @Test
    fun returnsNullForInvalidHunkHeader() {
        val original = "a\nb"
        val diff = "@@ invalid @@\n+a"
        assertNull(applyDiff(original, diff))
    }

    @Test
    fun skipsHeaderLines() {
        val original = "alpha\nbeta"
        val diff = "--- a/file.txt\n+++ b/file.txt\n@@ -1,2 +1,2 @@\n alpha\n-beta\n+changed"
        assertEquals("alpha\nchanged", applyDiff(original, diff))
    }

    @Test
    fun handlesMultipleHunks() {
        val original = "a\nb\nc\nd\ne"
        val diff = "@@ -1,2 +1,2 @@\n a\n-b\n+B\n@@ -4,2 +4,2 @@\n d\n-e\n+E"
        assertEquals("a\nB\nc\nd\nE", applyDiff(original, diff))
    }

    @Test
    fun handlesEmptyOriginal() {
        val diff = "@@ -0,0 +1,1 @@\n+new line"
        assertEquals("new line", applyDiff("", diff))
    }

    @Test
    fun handlesEmptyHunk() {
        val original = "line1\nline2"
        val diff = "@@ -1,2 +1,0 @@\n-line1\n-line2"
        assertEquals("", applyDiff(original, diff))
    }

    @Test
    fun handleLineCountMismatchStillApplies() {
        // The algorithm does not validate counts strictly; it applies what it finds.
        val original = "a\nb\nc"
        val diff = "@@ -1,3 +1,4 @@\n a\n+inserted\n b\n c"
        assertEquals("a\ninserted\nb\nc", applyDiff(original, diff))
    }

    @Test
    fun handlesHunkAtEndOfFile() {
        val original = "a\nb\nc"
        val diff = "@@ -3,1 +3,2 @@\n c\n+extra"
        assertEquals("a\nb\nc\nextra", applyDiff(original, diff))
    }
}
