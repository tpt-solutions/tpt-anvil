// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package com.tptsolutions.anvil.settings

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.*

@State(name = "AnvilSettings", storages = [Storage("anvil.xml")])
class AnvilSettings : PersistentStateComponent<AnvilSettings.State> {
    data class State(
        var backend: String = "ollama",
        var model: String = "deepseek-coder:6.7b",
        var ollamaUrl: String = "http://localhost:11434",
        var cloudProvider: String = "",
        var maxTokens: Int = 2048,
        var temperature: Double = 0.2,
    )

    private var state = State()

    override fun getState(): State = state
    override fun loadState(state: State) { this.state = state }

    companion object {
        fun getInstance(): AnvilSettings =
            ApplicationManager.getApplication().getService(AnvilSettings::class.java)
    }
}
