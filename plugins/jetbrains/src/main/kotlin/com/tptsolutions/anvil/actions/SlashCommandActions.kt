package com.tptsolutions.anvil.actions

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.wm.ToolWindowManager
import com.tptsolutions.anvil.CodeContext
import com.tptsolutions.anvil.TextSelection
import com.tptsolutions.anvil.ui.AnvilChatPanel

abstract class BaseAnvilAction(private val command: String) : AnAction() {
    override fun actionPerformed(e: AnActionEvent) {
        val editor = e.getData(CommonDataKeys.EDITOR) ?: return
        val project = e.project ?: return

        val sel = editor.selectionModel
        val hasSelection = sel.hasSelection()
        val content = if (hasSelection) sel.selectedText ?: "" else editor.document.text
        val language = editor.project?.let { 
            com.intellij.lang.LanguageUtil.getLanguageTypeByExtension(
                editor.virtualFile?.extension ?: ""
            )?.id
        } ?: ""

        val ctx = CodeContext(
            file_path = editor.virtualFile?.path ?: "",
            language = language,
            content = content,
            cursor_line = editor.caretModel.logicalPosition.line,
            selection = if (hasSelection) TextSelection(
                start_line = sel.selectionStart,
                end_line = sel.selectionEnd,
                start_col = 0,
                end_col = 0,
            ) else null,
        )

        val tw = ToolWindowManager.getInstance(project).getToolWindow("Anvil")
        if (tw != null) {
            val contentMgr = tw.contentManager
            if (contentMgr.contentCount > 0) {
                val panel = contentMgr.getContent(0)?.component as? AnvilChatPanel
                panel?.sendCommandWithContext(command, ctx)
                tw.activate(null)
            }
        }
    }

    override fun update(e: AnActionEvent) {
        e.presentation.isEnabled = e.getData(CommonDataKeys.EDITOR) != null
    }

    override fun getActionUpdateThread() = com.intellij.openapi.actionSystem.ActionUpdateThread.BGT
}

class ExplainAction : BaseAnvilAction("/explain")
class FixAction : BaseAnvilAction("/fix")
class GenerateTestAction : BaseAnvilAction("/test")
class GenerateDocsAction : BaseAnvilAction("/docs")
