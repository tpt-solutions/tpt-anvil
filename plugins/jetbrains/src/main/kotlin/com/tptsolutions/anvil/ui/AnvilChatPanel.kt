package com.tptsolutions.anvil.ui

import com.intellij.openapi.project.Project
import com.intellij.ui.components.JBScrollPane
import com.tptsolutions.anvil.CodeContext
import com.tptsolutions.anvil.DaemonClient
import kotlinx.coroutines.*
import java.awt.BorderLayout
import java.awt.Font
import javax.swing.*

class AnvilChatPanel(private val project: Project) {
    val component: JPanel = JPanel(BorderLayout())
    private val daemon = DaemonClient()
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    private val chatDisplay = JEditorPane().apply {
        isEditable = false
        contentType = "text/html"
        text = "<html><body style='font-family: sans-serif; font-size: 13px; padding: 8px;'>" +
                "<i>Welcome to Anvil Chat</i></body></html>"
    }
    private val inputField = JTextArea(3, 40).apply {
        lineWrap = true
        wrapStyleWord = true
        font = Font(Font.MONOSPACED, Font.PLAIN, 13)
    }
    private val sendButton = JButton("Send")
    private val htmlBuffer = StringBuilder()

    init {
        component.add(JBScrollPane(chatDisplay), BorderLayout.CENTER)

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

    fun sendCommandWithContext(command: String, ctx: CodeContext) {
        inputField.text = "$command "
        inputField.requestFocus()
        scope.launch {
            appendHtml("<p><b>You:</b> ${escapeHtml(command)}</p>")
            htmlBuffer.clear()
            appendHtml("<p><b>Anvil:</b> ")
            try {
                daemon.slashCommand(command, ctx, "main") { chunk ->
                    SwingUtilities.invokeLater {
                        htmlBuffer.append(escapeHtml(chunk.delta))
                        updateChatDisplay()
                    }
                }
                appendHtml("</p>")
            } catch (e: Exception) {
                SwingUtilities.invokeLater {
                    appendHtml("</p><p style='color:red;'>Error: ${escapeHtml(e.message ?: "unknown")}</p>")
                    updateChatDisplay()
                }
            }
        }
    }

    private fun sendMessage() {
        val text = inputField.text.trim()
        if (text.isEmpty()) return
        inputField.text = ""
        appendHtml("<p><b>You:</b> ${escapeHtml(text)}</p>")

        val ctx = CodeContext(
            file_path = "",
            language = "",
            content = "",
        )

        scope.launch {
            htmlBuffer.clear()
            appendHtml("<p><b>Anvil:</b> ")
            try {
                daemon.slashCommand(text, ctx, "main") { chunk ->
                    SwingUtilities.invokeLater {
                        htmlBuffer.append(escapeHtml(chunk.delta))
                        updateChatDisplay()
                    }
                }
                appendHtml("</p>")
            } catch (e: Exception) {
                SwingUtilities.invokeLater {
                    appendHtml("</p><p style='color:red;'>Error: ${escapeHtml(e.message ?: "unknown")}</p>")
                    updateChatDisplay()
                }
            }
        }
    }

    private fun appendHtml(html: String) {
        SwingUtilities.invokeLater {
            val bodyEnd = chatDisplay.text.indexOf("</body>")
            if (bodyEnd > 0) {
                val before = chatDisplay.text.substring(0, bodyEnd)
                chatDisplay.text = "$before$html</body></html>"
            }
        }
    }

    private fun updateChatDisplay() {
        val bodyEnd = chatDisplay.text.indexOf("</body>")
        if (bodyEnd > 0) {
            val before = chatDisplay.text.substring(0, bodyEnd)
            val rendered = renderMarkdown(htmlBuffer.toString())
            chatDisplay.text = "$before$rendered</body></html>"
        }
    }

    private fun renderMarkdown(text: String): String {
        var result = escapeHtml(text)
        // Code blocks
        result = result.replace(Regex("```(\\w*)\\n([\\s\\S]*?)```")) { match ->
            "<pre style='background:#f4f4f4; padding:8px; border-radius:4px;'>" +
            "<code>${match.groupValues[2]}</code></pre>"
        }
        // Inline code
        result = result.replace(Regex("`([^`]+)`")) { 
            "<code style='background:#f0f0f0; padding:1px 4px;'>${it.groupValues[1]}</code>"
        }
        // Bold
        result = result.replace(Regex("\\*\\*(.+?)\\*\\*")) { "<b>${it.groupValues[1]}</b>" }
        // Italic
        result = result.replace(Regex("\\*(.+?)\\*")) { "<i>${it.groupValues[1]}</i>" }
        // Newlines to <br>
        result = result.replace("\n", "<br>")
        return result
    }

    private fun escapeHtml(text: String): String {
        return text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    }
}
