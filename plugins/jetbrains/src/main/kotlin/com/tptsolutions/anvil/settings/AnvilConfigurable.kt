// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package com.tptsolutions.anvil.settings

import com.intellij.openapi.options.BoundConfigurable
import com.intellij.openapi.ui.DialogPanel
import com.intellij.ui.dsl.builder.*

class AnvilConfigurable : BoundConfigurable("TPT Anvil") {
    private val settings = AnvilSettings.getInstance()

    override fun createPanel(): DialogPanel = panel {
        group("Inference") {
            row("Backend:") {
                comboBox(listOf("ollama", "llama_cpp", "candle"))
                    .bindItem(settings.state::backend.toNullableProperty())
            }
            row("Model:") {
                textField().bindText(settings.state::model)
            }
            row("Ollama URL:") {
                textField().bindText(settings.state::ollamaUrl)
            }
        }
        group("Cloud Fallback") {
            row("Provider:") {
                comboBox(listOf("", "openai", "anthropic", "openrouter", "azure", "custom"))
                    .bindItem(settings.state::cloudProvider.toNullableProperty())
            }
        }
        group("Generation") {
            row("Max Tokens:") {
                intTextField(100..32000).bindIntText(settings.state::maxTokens)
            }
        }
    }
}
