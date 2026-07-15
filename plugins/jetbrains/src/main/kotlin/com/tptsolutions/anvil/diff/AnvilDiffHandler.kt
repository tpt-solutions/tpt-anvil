// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
package com.tptsolutions.anvil.diff

import com.intellij.openapi.command.WriteCommandAction
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.Messages
import com.intellij.openapi.vfs.LocalFileSystem

object AnvilDiffHandler {

    /**
     * Shows a confirmation dialog and, on approval, applies the unified diff to the given file.
     *
     * The "after" content is produced by a simplified strategy: collect every line in the diff
     * that starts with `+` but is NOT a `+++` header line, strip the leading `+`, and join the
     * resulting lines with newlines.
     */
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

        // Simple unified-diff application: keep only `+` lines that are not the `+++` header.
        val afterText = unifiedDiff.lines()
            .filter { line ->
                line.startsWith("+") &&
                !line.startsWith("+++")
            }
            .joinToString("\n") { it.removePrefix("+") }

        val answer = Messages.showYesNoDialog(
            project,
            "Apply the following changes to ${vFile.name}?\n\n" +
                "(Current length: ${beforeText.length} chars → New length: ${afterText.length} chars)",
            "Anvil — Apply Diff",
            Messages.getQuestionIcon()
        )

        if (answer == Messages.YES) {
            WriteCommandAction.runWriteCommandAction(project, "Anvil: Apply Diff", null, {
                document.setText(afterText)
            })
        }
    }
}
