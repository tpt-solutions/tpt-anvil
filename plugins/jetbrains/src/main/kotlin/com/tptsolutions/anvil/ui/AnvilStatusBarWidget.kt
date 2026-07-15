// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
package com.tptsolutions.anvil.ui

import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.StatusBar
import com.intellij.openapi.wm.StatusBarWidget
import com.intellij.util.Consumer
import com.tptsolutions.anvil.settings.AnvilSettings
import java.awt.Component
import java.awt.event.MouseEvent

class AnvilStatusBarWidget(private val project: Project) :
    StatusBarWidget, StatusBarWidget.TextPresentation {

    companion object {
        const val ID = "AnvilStatusBarWidget"
    }

    private var statusBar: StatusBar? = null

    // --- StatusBarWidget ---

    override fun ID(): String = ID

    override fun getPresentation(): StatusBarWidget.WidgetPresentation = this

    override fun install(statusBar: StatusBar) {
        this.statusBar = statusBar
    }

    override fun dispose() {
        statusBar = null
    }

    // --- StatusBarWidget.TextPresentation ---

    override fun getText(): String {
        val settings = AnvilSettings.getInstance()
        val state = settings.state
        return "Anvil: ${state.backend} / ${state.model}"
    }

    override fun getTooltipText(): String = "TPT Anvil — AI Code Assistant"

    override fun getAlignment(): Float = Component.CENTER_ALIGNMENT

    override fun getClickConsumer(): Consumer<MouseEvent>? = null
}
