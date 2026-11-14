package app.shurco.tabtool

import android.content.Context
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.text.InputType
import android.view.KeyEvent
import android.view.ViewGroup
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import androidx.appcompat.widget.AppCompatEditText

/**
 * Real Android [android.widget.EditText] used for text entry above the egui
 * surface. It reports the final value through the JNI callbacks.
 */
class TabToolTextEditor(context: Context) : AppCompatEditText(context) {
    private var finished = false
    private var multiline = false

    init {
        setTextColor(Color.WHITE)
        setHintTextColor(Color.GRAY)
        textSize = 16f
        setPadding(24, 16, 24, 16)
        background = GradientDrawable().apply {
            setColor(0xFF1E1E1E.toInt())
            setStroke(2, 0xFF4C8BF5.toInt())
            cornerRadius = 12f
        }
        setShowSoftInputOnFocus(true)
        setOnEditorActionListener { _, actionId, event ->
            val done =
                actionId == EditorInfo.IME_ACTION_DONE ||
                    actionId == EditorInfo.IME_ACTION_GO ||
                    (!multiline && event?.keyCode == KeyEvent.KEYCODE_ENTER &&
                        event.action == KeyEvent.ACTION_DOWN)
            if (done) {
                finish(commit = true)
                true
            } else {
                false
            }
        }
        setOnFocusChangeListener { _, hasFocus ->
            if (!hasFocus) {
                finish(commit = true)
            }
        }
    }

    fun configure(text: String, multiline: Boolean) {
        finished = false
        this.multiline = multiline
        setSingleLine(!multiline)
        inputType = if (multiline) {
            InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
        } else {
            InputType.TYPE_CLASS_TEXT
        }
        imeOptions = if (multiline) EditorInfo.IME_ACTION_NONE else EditorInfo.IME_ACTION_DONE
        setText(text)
        setSelection(text.length)
        post {
            requestFocus()
            showKeyboard()
        }
    }

    fun cancelEdit() {
        finish(commit = false)
    }

    /** Commits the current text as if the user pressed Done. */
    fun commitEdit() {
        finish(commit = true)
    }

    fun dismissWithoutCallback() {
        finished = true
        hideAndRemove()
    }

    private fun finish(commit: Boolean) {
        if (finished) {
            return
        }
        finished = true
        val value = text?.toString().orEmpty()
        if (commit) {
            TabToolBridge.nativeTextCommitted(value)
        } else {
            TabToolBridge.nativeTextCancelled()
        }
        hideAndRemove()
    }

    private fun showKeyboard() {
        val manager = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        manager.showSoftInput(this, InputMethodManager.SHOW_IMPLICIT)
    }

    private fun hideAndRemove() {
        val manager = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        manager.hideSoftInputFromWindow(windowToken, 0)
        (parent as? ViewGroup)?.removeView(this)
    }
}
