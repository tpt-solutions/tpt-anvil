package com.tptsolutions.anvil.diff

import com.intellij.openapi.command.WriteCommandAction
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.Messages
import com.intellij.openapi.vfs.LocalFileSystem

object AnvilDiffHandler {

    fun showAndApplyDiff(project: Project, filePath: String, unifiedDiff: String) {
        val vFile = LocalFileSystem.getInstance().findFileByPath(filePath)
            ?: run {
                Messages.showErrorDialog(project, "File not found: $filePath", "Anvil")
                return
            }

        val document = FileDocumentManager.getInstance().getDocument(vFile)
            ?: run {
                Messages.showErrorDialog(project, "Could not open document for: $filePath", "Anvil")
                return
            }

        val beforeText = document.text
        val afterText = applyUnifiedDiff(beforeText, unifiedDiff)
            ?: run {
                Messages.showErrorDialog(project, "Failed to apply diff — patch did not match.", "Anvil")
                return
            }

        val preview = buildDiffPreview(beforeText, afterText, vFile.name)
        val answer = Messages.showYesNoDialog(
            project,
            preview,
            "Anvil — Apply Diff",
            Messages.getQuestionIcon()
        )

        if (answer == Messages.YES) {
            WriteCommandAction.runWriteCommandAction(project, "Anvil: Apply Diff", null, {
                document.setText(afterText)
            })
        }
    }

    private fun applyUnifiedDiff(original: String, diff: String): String? {
        val originalLines = original.lines()
        val diffLines = diff.lines()
        val result = mutableListOf<String>()
        var origCursor = 0
        var inHunk = false

        for (dline in diffLines) {
            if (dline.startsWith("---") || dline.startsWith("+++")) continue
            if (dline.startsWith("@@")) {
                val start = parseHunkOrigStart(dline) ?: return null
                val target = (start - 1).coerceAtLeast(0)
                while (origCursor < target && origCursor < originalLines.size) {
                    result.add(originalLines[origCursor])
                    origCursor++
                }
                inHunk = true
                continue
            }
            if (!inHunk) continue

            when {
                dline.startsWith(" ") -> {
                    result.add(dline.substring(1))
                    origCursor++
                }
                dline.startsWith("-") -> origCursor++
                dline.startsWith("+") -> result.add(dline.substring(1))
            }
        }

        while (origCursor < originalLines.size) {
            result.add(originalLines[origCursor])
            origCursor++
        }

        return if (inHunk) result.joinToString("\n") else null
    }

    private fun parseHunkOrigStart(header: String): Int? {
        val match = Regex("""@@\s*-(\d+)""").find(header) ?: return null
        return match.groupValues[1].toIntOrNull()
    }

    private fun buildDiffPreview(before: String, after: String, fileName: String): String {
        val beforeLines = before.lines().size
        val afterLines = after.lines().size
        return "Apply changes to $fileName?\n\n" +
                "Before: $beforeLines lines\nAfter: $afterLines lines\n\n" +
                "Review the diff in the editor before confirming."
    }
}
