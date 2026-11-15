package app.shurco.tabtool

import android.widget.FrameLayout
import kotlin.math.roundToInt

/** Places the native [TabToolTextEditor] over the egui surface. */
internal object TextEditorHost {
    @Volatile
    private var textEditor: TabToolTextEditor? = null

    /** Drops the editor without a JNI callback, for activity teardown. */
    fun detach() {
        textEditor?.dismissWithoutCallback()
        textEditor = null
    }

    fun show(
        left: Float,
        top: Float,
        width: Float,
        height: Float,
        text: String,
        multiline: Boolean,
    ): Boolean {
        val activity = TabToolBridge.activity ?: return false
        if (!width.isFinite() || !height.isFinite() || width <= 0f || height <= 0f) {
            return false
        }
        activity.runOnUiThread {
            val root = activity.findViewById<FrameLayout>(android.R.id.content)
            if (root == null) {
                TabToolBridge.nativeTextCancelled()
                return@runOnUiThread
            }
            val density = activity.resources.displayMetrics.density
            val editor = textEditor ?: TabToolTextEditor(activity).also { textEditor = it }
            val params =
                FrameLayout.LayoutParams(
                    (width * density).roundToInt(),
                    (height * density).roundToInt(),
                ).apply {
                    leftMargin = (left * density).roundToInt()
                    topMargin = (top * density).roundToInt()
                }
            if (editor.parent == null) {
                root.addView(editor, params)
            } else {
                editor.layoutParams = params
            }
            editor.elevation = 24f
            editor.configure(text, multiline)
        }
        return true
    }

    fun hide() {
        val activity = TabToolBridge.activity ?: return
        activity.runOnUiThread {
            textEditor?.dismissWithoutCallback()
            textEditor = null
        }
    }

    fun commit() {
        val activity = TabToolBridge.activity ?: return
        activity.runOnUiThread { textEditor?.commitEdit() }
    }

    fun move(
        left: Float,
        top: Float,
        width: Float,
        height: Float,
    ) {
        val activity = TabToolBridge.activity ?: return
        if (!width.isFinite() || !height.isFinite() || width <= 0f || height <= 0f) {
            return
        }
        activity.runOnUiThread {
            val editor = textEditor ?: return@runOnUiThread
            val density = activity.resources.displayMetrics.density
            val params =
                FrameLayout.LayoutParams(
                    (width * density).roundToInt(),
                    (height * density).roundToInt(),
                ).apply {
                    leftMargin = (left * density).roundToInt()
                    topMargin = (top * density).roundToInt()
                }
            editor.layoutParams = params
            editor.requestLayout()
        }
    }

    fun cancel(): Boolean {
        val editor = textEditor ?: return false
        if (editor.parent == null) {
            textEditor = null
            return false
        }
        editor.cancelEdit()
        textEditor = null
        return true
    }
}
