// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
package com.tptsolutions.anvil.ui

import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.StatusBarWidget
import com.intellij.openapi.wm.StatusBarWidgetFactory

class AnvilStatusBarWidgetFactory : StatusBarWidgetFactory {

    override fun getId(): String = AnvilStatusBarWidget.ID

    override fun getDisplayName(): String = "Anvil Status"

    override fun isAvailable(project: Project): Boolean = true

    override fun createWidget(project: Project): StatusBarWidget = AnvilStatusBarWidget(project)

    override fun disposeWidget(widget: StatusBarWidget) {
        widget.dispose()
    }
}
