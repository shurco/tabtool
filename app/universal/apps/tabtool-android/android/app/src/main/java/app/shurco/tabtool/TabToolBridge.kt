package app.shurco.tabtool

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.widget.FrameLayout
import androidx.core.graphics.Insets
import androidx.core.view.WindowInsetsCompat
import kotlin.math.roundToInt

/**
 * JNI surface for the Rust spike. Every `@JvmStatic` method is called from the
 * native thread that runs `android_main`.
 */
object TabToolBridge {
    private const val REQUEST_OPEN_DOCUMENT = 1001

    @Volatile
    private var activity: Activity? = null

    @Volatile
    private var textEditor: TabToolTextEditor? = null

    @Volatile
    private var pendingDocumentUri: String? = null

    @Volatile
    private var insetsDp: IntArray = intArrayOf(0, 0, 0, 0, 0)

    fun attach(activity: Activity) {
        this.activity = activity
    }

    fun detach() {
        textEditor?.dismissWithoutCallback()
        textEditor = null
        activity = null
    }

    fun updateInsets(insets: WindowInsetsCompat) {
        val density = activity?.resources?.displayMetrics?.density?.takeIf { it > 0f } ?: return
        val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars())
        val cutout = insets.getInsets(WindowInsetsCompat.Type.displayCutout())
        val safe = Insets.max(bars, cutout)
        val ime = insets.getInsets(WindowInsetsCompat.Type.ime())

        insetsDp = intArrayOf(
            dp(safe.left, density),
            dp(safe.top, density),
            dp(safe.right, density),
            dp(safe.bottom, density),
            dp(ime.bottom, density),
        )
    }

    private fun dp(px: Int, density: Float): Int = (px / density).roundToInt()

    @JvmStatic
    fun clipboardGet(): String? {
        val activity = activity ?: return null
        return try {
            val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            val clip = clipboard.primaryClip ?: return null
            if (clip.itemCount == 0) null else clip.getItemAt(0).text?.toString()
        } catch (error: SecurityException) {
            null
        }
    }

    @JvmStatic
    fun clipboardSet(text: String) {
        val activity = activity ?: return
        try {
            val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            clipboard.setPrimaryClip(ClipData.newPlainText("TabTool", text))
        } catch (error: SecurityException) {
            // The spike reports failures through the UI; reading is what the
            // dedicated exception test exercises.
        }
    }

    /**
     * Returns the URI picked by the previous call and clears it. When there is
     * no pending URI, opens the document picker and returns null.
     */
    @JvmStatic
    fun openDocument(): String? {
        pendingDocumentUri?.let { uri ->
            pendingDocumentUri = null
            return uri
        }

        val activity = activity ?: return null
        activity.runOnUiThread {
            val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "text/*"
            }
            try {
                activity.startActivityForResult(intent, REQUEST_OPEN_DOCUMENT)
            } catch (error: ActivityNotFoundException) {
                val fallback = Intent(Intent.ACTION_GET_CONTENT).apply {
                    addCategory(Intent.CATEGORY_OPENABLE)
                    type = "text/*"
                }
                try {
                    activity.startActivityForResult(fallback, REQUEST_OPEN_DOCUMENT)
                } catch (error: ActivityNotFoundException) {
                    // No file manager installed; the UI stays on the empty result.
                }
            }
        }
        return null
    }

    @JvmStatic
    fun insets(): IntArray = insetsDp.copyOf()

    @JvmStatic
    fun showTextEditor(
        left: Int,
        top: Int,
        width: Int,
        height: Int,
        text: String,
        multiline: Boolean,
    ) {
        val activity = activity ?: return
        activity.runOnUiThread {
            if (width <= 0 || height <= 0) {
                return@runOnUiThread
            }
            val root = activity.findViewById<FrameLayout>(android.R.id.content)
                ?: return@runOnUiThread
            val editor = textEditor ?: TabToolTextEditor(activity).also { textEditor = it }
            val params = FrameLayout.LayoutParams(width, height).apply {
                leftMargin = left
                topMargin = top
            }
            if (editor.parent == null) {
                root.addView(editor, params)
            } else {
                editor.layoutParams = params
            }
            editor.elevation = 24f
            editor.configure(text, multiline)
        }
    }

    @JvmStatic
    fun hideTextEditor() {
        val activity = activity ?: return
        activity.runOnUiThread {
            textEditor?.dismissWithoutCallback()
            textEditor = null
        }
    }

    /** Returns true when a visible editor was cancelled. Runs on the UI thread. */
    fun cancelTextEditor(): Boolean {
        val editor = textEditor ?: return false
        if (editor.parent == null) {
            textEditor = null
            return false
        }
        editor.cancelEdit()
        textEditor = null
        return true
    }

    /** Spike-only self-test: proves the Rust bridge clears JNI exceptions. */
    @JvmStatic
    fun throwSecurityException() {
        throw SecurityException("TabTool JNI exception test")
    }

    fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        if (requestCode != REQUEST_OPEN_DOCUMENT || resultCode != Activity.RESULT_OK) {
            return
        }
        pendingDocumentUri = data?.data?.toString()
    }
}
