// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package com.tptsolutions.anvil.ui

import com.intellij.openapi.project.Project
import com.intellij.ui.components.JBScrollPane
import com.intellij.ui.components.JBTextArea
import com.tptsolutions.anvil.DaemonClient
import com.tptsolutions.anvil.CodeContext
import kotlinx.coroutines.*
import java.awt.BorderLayout
import java.awt.Dimension
import javax.swing.*

class AnvilChatPanel(private val project: Project) {
    val component: JPanel = JPanel(BorderLayout())
    private val daemon = DaemonClient()
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    private val chatArea = JBTextArea().apply {
        isEditable = false
        lineWrap = true
        wrapStyleWord = true
        font = font.deriveFont(13f)
    }
    private val inputField = JBTextArea(3, 40).apply {
        lineWrap = true
        wrapStyleWord = true
    }
    private val sendButton = JButton("Send")

    init {
        component.add(JBScrollPane(chatArea), BorderLayout.CENTER)

        val inputPanel = JPanel(BorderLayout()).apply {
            val cmdPanel = JPanel().apply {
                for (cmd in listOf("/generate", "/test", "/explain", "/fix", "/docs")) {
                    add(JButton(cmd).apply {
                        addActionListener { inputField.text = "$cmd "; inputField.requestFocus() }
                    })
                }
            }
            add(cmdPanel, BorderLayout.NORTH)
            add(JBScrollPane(inputField), BorderLayout.CENTER)
            add(sendButton, BorderLayout.EAST)
        }
        component.add(inputPanel, BorderLayout.SOUTH)

        sendButton.addActionListener { sendMessage() }
        inputField.addKeyListener(object : java.awt.event.KeyAdapter() {
            override fun keyPressed(e: java.awt.event.KeyEvent) {
                if (e.keyCode == java.awt.event.KeyEvent.VK_ENTER && !e.isShiftDown) {
                    e.consume()
                    sendMessage()
                }
            }
        })

        tryConnect()
    }

    private fun tryConnect() {
        scope.launch(Dispatchers.IO) {
            try { daemon.connect() } catch (_: Exception) {}
        }
    }

    private fun sendMessage() {
        val text = inputField.text.trim()
        if (text.isEmpty()) return
        inputField.text = ""
        appendMessage("You", text)

        val ctx = CodeContext(
            file_path = "",
            language = "",
            content = "",
        )

        scope.launch {
            appendMessage("Anvil", "")
            try {
                daemon.slashCommand(text, ctx, "main") { chunk ->
                    SwingUtilities.invokeLater {
                        chatArea.append(chunk.delta)
                        chatArea.caretPosition = chatArea.document.length
                    }
                }
                SwingUtilities.invokeLater { chatArea.append("\n\n") }
            } catch (e: Exception) {
                SwingUtilities.invokeLater { appendMessage("Error", e.message ?: "unknown") }
            }
        }
    }

    private fun appendMessage(sender: String, text: String) {
        chatArea.append("[$sender]: $text\n")
        chatArea.caretPosition = chatArea.document.length
    }
}
