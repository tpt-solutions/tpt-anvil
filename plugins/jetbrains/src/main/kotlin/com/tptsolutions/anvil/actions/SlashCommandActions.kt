// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package com.tptsolutions.anvil.actions

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys

abstract class BaseAnvilAction(private val command: String) : AnAction() {
    override fun actionPerformed(e: AnActionEvent) {
        val editor = e.getData(CommonDataKeys.EDITOR) ?: return
        val project = e.project ?: return
        // TODO: wire to tool window and daemon
    }

    override fun update(e: AnActionEvent) {
        e.presentation.isEnabled = e.getData(CommonDataKeys.EDITOR) != null
    }
}

class ExplainAction : BaseAnvilAction("/explain")
class FixAction : BaseAnvilAction("/fix")
class GenerateTestAction : BaseAnvilAction("/test")
class GenerateDocsAction : BaseAnvilAction("/docs")
